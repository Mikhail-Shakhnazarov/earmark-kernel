use std::collections::BTreeMap;

use chrono::Utc;
use earmark_connected_context::CompiledContextCompiler;
use earmark_core::{
    AssignmentStatus, ChangeSetDraft, ChangeSetId, ChangeSetValidationResult, Kind,
    ObjectRef, ProviderRequest, RunRecord, TransitionAssignment, TransitionAssignmentId,
    WorkPacketConstraints, WorkflowOperationKind,
};
use earmark_governance::{escalation_for_trigger, export_allowed, GovernanceService};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};

use crate::engine::ExecutionEngine;
use crate::error::{ExecError, ProviderFailure, ProviderFailureKind};
use crate::handoff::derive_successor_handoff;
use crate::helpers::{
    record_transition, reject_duplicate_active_assignment, store_work_packet, uuid_like,
    work_packet_from_compiled_context, work_surface_manifest_path,
};
use crate::ir::{ExecutionIr, ExecutionTransition, WorkflowRunRequest};
use crate::persistence::{
    persist_assignment_update, persist_change_set, persist_transformation_failure,
    ChangeSetPersistence,
};
use crate::persistence_helpers::write_object_and_index;
use crate::provider::{
    provider_metadata_synthetic_source, provider_record_from_failure,
    provider_response_is_synthetic, resolve_provider_profile, ProviderMode,
};
use crate::resolution::{
    load_instruction, load_provider_profile, load_standing_policy, resolve_version_for_kind,
};
use crate::state::ExecutionState;
use crate::validation::validate_transition_change_set;

impl<'a, S: CanonicalStore> ExecutionEngine<'a, S> {
    #[allow(clippy::too_many_arguments)]
    pub fn execute_transition<C: CompiledContextCompiler<S>>(
        &self,
        request: &WorkflowRunRequest,
        system: &earmark_core::SystemDefinition,
        ir: &ExecutionIr,
        transition: &ExecutionTransition,
        state: &mut ExecutionState,
        record: &mut RunRecord,
        context_compiler: &C,
    ) -> Result<(), ExecError> {

        let (_assignment_id, stored_assignment_head) = self.initialize_assignment(record, transition)?;
        let mut assignment = serde_json::from_slice::<TransitionAssignment>(
            stored_assignment_head.payload.bytes.as_slice(),
        )?;

        let mut change_set_draft = ChangeSetDraft::default();
        let mut synthetic_output_warning: Option<String> = None;

        let filtered_inputs = self.get_filtered_inputs(state, transition);

        let exec_result: Result<(), ExecError> = match transition.operation {
            WorkflowOperationKind::CompileContext => self.handle_compile_context(
                request,
                system,
                transition,
                state,
                record,
                context_compiler,
                &filtered_inputs,
                &mut change_set_draft,
            ),
            WorkflowOperationKind::Transform => {
                let (res, warning) = self.handle_transform(
                    request,
                    system,
                    transition,
                    state,
                    record,
                    &filtered_inputs,
                    &mut change_set_draft,
                );
                synthetic_output_warning = warning;
                res
            }
            WorkflowOperationKind::Review => {
                self.handle_review(request, transition, state, record, &mut change_set_draft)
            }
            WorkflowOperationKind::Export => {
                self.handle_export(system, transition, state, record, &mut change_set_draft)
            }
            WorkflowOperationKind::Nop => Ok(()),
        };

        if let Err(error) = exec_result {
            return self.finalize_execution_failure(
                record,
                transition,
                &mut assignment,
                &stored_assignment_head,
                &change_set_draft,
                &filtered_inputs,
                error,
            );
        }

        self.finalize_transition_success(
            request,
            system,
            ir,
            transition,
            state,
            record,
            &filtered_inputs,
            &mut assignment,
            &stored_assignment_head,
            &mut change_set_draft,
            synthetic_output_warning,
        )
    }

    fn initialize_assignment(
        &self,
        record: &mut RunRecord,
        transition: &ExecutionTransition,
    ) -> Result<(TransitionAssignmentId, StoredObject), ExecError> {
        reject_duplicate_active_assignment(self.store, &record.run_id, &transition.id)?;

        let assignment_id = TransitionAssignmentId::new();
        let now = Utc::now();
        let assignment = TransitionAssignment {
            id: assignment_id.clone(),
            run_id: record.run_id.clone(),
            transition_id: transition.id.clone(),
            assigned_to: "execution_engine".to_string(),
            status: AssignmentStatus::Assigned,
            input_object_ids: vec![], // Will be updated on completion if needed, or kept as intent
            handoff_manifest_id: None,
            event_ids: vec![],
            blocked_reason: None,
            completion_change_set_id: None,
            assigned_at: now,
            updated_at: now,
            expires_at: None,
            completed_at: None,
        };

        let stored_assignment = StoredObject::builder(
            Kind::TransitionAssignment,
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
        )
        .id(assignment.id.as_object_id())
        .class("transition_assignment")
        .provenance(earmark_core::Provenance::direct_input("execution_engine"))
        .header(
            "title",
            format!("TransitionAssignment {}", assignment.id.as_str()),
        )
        .build()
        .map_err(ExecError::IncompleteExecution)?;

        write_object_and_index(self.store, self.index, &stored_assignment)?;
        record.assignments.push(assignment_id.clone());
        Ok((assignment_id, stored_assignment))
    }

    fn get_filtered_inputs(
        &self,
        state: &ExecutionState,
        transition: &ExecutionTransition,
    ) -> Vec<ObjectRef> {
        if transition.input_contracts.is_empty() {
            state.active_objects.clone()
        } else {
            let matching_objects = state
                .active_objects
                .iter()
                .filter(|obj| {
                    obj.class
                        .as_ref()
                        .map(|c| transition.input_contracts.contains(c))
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>();
            if matching_objects.is_empty() {
                state.active_objects.clone()
            } else {
                matching_objects
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_compile_context<C: CompiledContextCompiler<S>>(
        &self,
        request: &WorkflowRunRequest,
        system: &earmark_core::SystemDefinition,
        transition: &ExecutionTransition,
        state: &mut ExecutionState,
        record: &mut RunRecord,
        context_compiler: &C,
        filtered_inputs: &[ObjectRef],
        change_set_draft: &mut ChangeSetDraft,
    ) -> Result<(), ExecError> {
        let template_ref = transition.compiled_context.as_ref().ok_or_else(|| {
            ExecError::InvalidWorkflow(format!(
                "transition {} requires a compiled context reference",
                transition.id
            ))
        })?;
        let resolved_template = resolve_version_for_kind(
            self.store,
            self.index,
            template_ref,
            Kind::CompiledContextTemplate,
        )?;
        let registry = load_registry(system)?;
        let manifest = context_compiler.compile(
            self.store,
            self.index,
            &resolved_template,
            None,
            &registry,
        )?;
        let work_packet = work_packet_from_compiled_context(
            request,
            transition,
            &manifest,
            WorkPacketConstraints {
                standing_requirements: BTreeMap::new(),
                review_requirements: vec![],
                prohibited_operations: vec![],
                export_permitted: false,
            },
            filtered_inputs.to_vec(),
        );
        let work_packet_object = store_work_packet(self.store, self.index, &work_packet)?;
        let work_packet_ref = work_packet_object.object_ref();
        change_set_draft
            .created_objects
            .push(work_packet_ref.id.clone());
        state.emitted_packets.push(work_packet_ref.clone());
        record.work_packets.push(work_packet_ref.clone());
        *state.compiled_context = Some(manifest);
        record_transition(
            record,
            transition.id.clone(),
            "context_compiled",
            filtered_inputs.to_vec(),
            vec![work_packet_ref],
            Some("work surface compiled".to_string()),
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_transform(
        &self,
        request: &WorkflowRunRequest,
        system: &earmark_core::SystemDefinition,
        transition: &ExecutionTransition,
        state: &mut ExecutionState,
        record: &mut RunRecord,
        filtered_inputs: &[ObjectRef],
        change_set_draft: &mut ChangeSetDraft,
    ) -> (Result<(), ExecError>, Option<String>) {
        let mut synthetic_output_warning = None;

        let res = (|| {
            if transition.output_contracts.len() > 1 {
                return Err(ExecError::Validation(earmark_core::ChangeSetValidationResult {
                    is_valid: false,
                    failures: vec!["multi-output transform operations are not yet implemented".to_string()],
                    warnings: vec![],
                    info: vec![],
                }));
            }
            let instruction_ref = transition.instruction.as_ref().ok_or_else(|| {
                ExecError::InvalidWorkflow(format!(
                    "transition {} requires an instruction reference",
                    transition.id
                ))
            })?;
            let resolved_instruction_ref = resolve_version_for_kind(
                self.store,
                self.index,
                instruction_ref,
                Kind::Instruction,
            )?;
            let instruction = load_instruction(self.store, self.index, &resolved_instruction_ref)?;
            let provider_mode = resolve_provider_profile(
                transition.provider_profile.as_ref(),
                Some(&instruction),
                system.default_provider_profile.as_ref(),
            );

            let work_surface = state.compiled_context.as_ref().ok_or_else(|| {
                ExecError::MissingWorkSurface(
                    "transform requires a prior compile_context operation".to_string(),
                )
            })?;

            let work_packet = work_packet_from_compiled_context(
                request,
                transition,
                work_surface,
                WorkPacketConstraints {
                    standing_requirements: BTreeMap::new(),
                    review_requirements: vec![],
                    prohibited_operations: vec![],
                    export_permitted: false,
                },
                filtered_inputs.to_vec(),
            );
            let work_packet_object = store_work_packet(self.store, self.index, &work_packet)?;
            let work_packet_ref = work_packet_object.object_ref();
            change_set_draft
                .created_objects
                .push(work_packet_ref.id.clone());
            state.emitted_packets.push(work_packet_ref.clone());
            record.work_packets.push(work_packet_ref.clone());

            let output_class = if !instruction.register.is_empty() {
                instruction.register.clone()
            } else {
                transition
                    .output_contracts
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "candidate_output".to_string())
            };

            let artifacts = match provider_mode {
                ProviderMode::LocalExecution => crate::persistence::create_local_transform_output(
                    self.store,
                    self.index,
                    &instruction,
                    &output_class,
                    filtered_inputs,
                    &resolved_instruction_ref,
                )?,
                ProviderMode::Delegated(profile_ref) => {
                    let profile = load_provider_profile(self.store, self.index, &profile_ref)?;
                    let context_text = state
                        .compiled_context
                        .as_ref()
                        .map(crate::helpers::render_provider_context);
                    let registry = load_registry(system)?;
                    let input_text = crate::helpers::render_provider_input(
                        self.store,
                        &instruction,
                        state.compiled_context.as_ref(),
                        filtered_inputs,
                        &profile,
                        &registry,
                    )?;

                    let provider_request = ProviderRequest {
                        request_id: format!("req_{}", uuid_like()),
                        run_id: record.run_id.clone(),
                        work_packet: work_packet_ref.clone(),
                        provider_profile: profile_ref.clone(),
                        instruction_text: instruction.body.as_str().to_string(),
                        context_text,
                        input_text,
                        work_surface_manifest: state
                            .compiled_context
                            .as_ref()
                            .map(|surface| work_surface_manifest_path(self.store, surface)),
                        inputs: filtered_inputs.to_vec(),
                        response_contract: profile.response_contract.clone(),
                        issued_at: Utc::now(),
                    };

                    // Input Budget Enforcement
                    if let Some(max_input) = profile.budget.max_input_tokens {
                        let mut input_text_to_estimate = provider_request.instruction_text.clone();
                        if let Some(ctx) = &provider_request.context_text {
                            input_text_to_estimate.push_str(ctx);
                        }
                        input_text_to_estimate.push_str(&provider_request.input_text);

                        let estimated = crate::helpers::estimate_tokens_approx(&input_text_to_estimate);
                        if estimated > max_input {
                            return Err(ExecError::Provider(ProviderFailure::new(
                                ProviderFailureKind::BudgetExceeded,
                                format!(
                                    "estimated input tokens {} exceeded budget of {}",
                                    estimated, max_input
                                ),
                            )));
                        }
                    }

                    match self.provider_service.provide(
                        &profile,
                        provider_request.clone(),
                        transition.operation.as_str(),
                    ) {
                        Ok(mut outcome) => {
                            let response = outcome.response.take().ok_or_else(|| {
                                ExecError::Provider(ProviderFailure::new(
                                    ProviderFailureKind::MalformedResponse,
                                    "delegated outcome did not contain a response",
                                ))
                            })?;

                            // Output Budget Enforcement
                            if let Some(max_output) = profile.budget.max_output_tokens {
                                let estimated = crate::helpers::estimate_tokens_approx(
                                    &response.candidate_payload,
                                );
                                if estimated > max_output {
                                    return Err(ExecError::Provider(ProviderFailure::new(
                                        ProviderFailureKind::BudgetExceeded,
                                        format!(
                                            "estimated output tokens {} exceeded budget of {}",
                                            estimated, max_output
                                        ),
                                    )));
                                }
                            }

                            // Cost Budget Enforcement
                            let mut budget_warning = None;
                            if let Some(max_cost) = profile.budget.max_cost_usd {
                                if let Some(estimated_cost) =
                                    response.usage.as_ref().and_then(|u| u.estimated_cost_usd)
                                {
                                    if estimated_cost > max_cost {
                                        return Err(ExecError::Provider(ProviderFailure::new(
                                            ProviderFailureKind::BudgetExceeded,
                                            format!(
                                                "estimated cost {} USD exceeded budget of {} USD",
                                                estimated_cost, max_cost
                                            ),
                                        )));
                                    }
                                } else {
                                    budget_warning = Some("Budget not enforceable: provider response did not report estimated_cost_usd.".to_string());
                                }
                            }

                            if provider_response_is_synthetic(&response) {
                                let source = provider_metadata_synthetic_source(&response.metadata)
                                    .unwrap_or_else(|| "mock_provider".to_string());
                                synthetic_output_warning = Some(format!(
                                    "synthetic provider output detected (source: {}); artifact is not production evidence",
                                    source
                                ));
                            }

                            let mut provider_record = outcome.record;
                            if let Some(warning) = &synthetic_output_warning {
                                provider_record.advisory_warnings.push(warning.clone());
                            }
                            if let Some(warning) = budget_warning {
                                provider_record.advisory_warnings.push(warning);
                            }

                            let _event_ref = self.record_provider_event(
                                record,
                                state,
                                change_set_draft,
                                provider_record,
                            )?;

                            crate::persistence::create_delegated_transform_output(
                                self.store,
                                self.index,
                                &instruction,
                                &output_class,
                                filtered_inputs,
                                &resolved_instruction_ref,
                                response,
                            )?
                        }
                        Err(failure) => {
                            let failure_message = failure.message.clone();
                            let provider_record = provider_record_from_failure(
                                &provider_request,
                                &profile,
                                &failure,
                            );

                            self.record_provider_event(
                                record,
                                state,
                                change_set_draft,
                                provider_record,
                            )?;

                            record_transition(
                                record,
                                transition.id.clone(),
                                "provider_failed",
                                filtered_inputs.to_vec(),
                                vec![],
                                Some(failure_message),
                            );
                            return Err(ExecError::Provider(failure));
                        }
                    }
                }
            };

            *state.active_objects = vec![artifacts.output.clone()];
            change_set_draft
                .created_objects
                .push(artifacts.output.id.clone());
            change_set_draft
                .created_relations
                .extend(artifacts.relation_ids);
            state.emitted_objects.push(artifacts.output.clone());
            record_transition(
                record,
                transition.id.clone(),
                "transformed",
                filtered_inputs.to_vec(),
                vec![artifacts.output],
                Some(format!("execution policy {}", instruction.execution_policy)),
            );
            Ok(())
        })();

        (res, synthetic_output_warning)
    }

    fn handle_review(
        &self,
        request: &WorkflowRunRequest,
        transition: &ExecutionTransition,
        state: &mut ExecutionState,
        record: &mut RunRecord,
        change_set_draft: &mut ChangeSetDraft,
    ) -> Result<(), ExecError> {
        let target = state.active_objects.first().cloned().ok_or_else(|| {
            ExecError::MissingInput("review requires a target object".to_string())
        })?;
        let review_object = GovernanceService::create_review_object(
            target.clone(),
            request.operator_approved,
            Some("review recorded by execution engine".to_string()),
        )?;
        write_object_and_index(self.store, self.index, &review_object)?;
        let review_ref = review_object.object_ref();
        change_set_draft.created_objects.push(review_ref.id.clone());
        state.emitted_objects.push(review_ref.clone());
        record_transition(
            record,
            transition.id.clone(),
            "reviewed",
            vec![target],
            vec![review_ref],
            Some(if request.operator_approved {
                "accepted".to_string()
            } else {
                "rejected".to_string()
            }),
        );
        Ok(())
    }

    fn handle_export(
        &self,
        system: &earmark_core::SystemDefinition,
        transition: &ExecutionTransition,
        state: &mut ExecutionState,
        record: &mut RunRecord,
        change_set_draft: &mut ChangeSetDraft,
    ) -> Result<(), ExecError> {
        let policy_ref = transition.policy.as_ref().ok_or_else(|| {
            ExecError::InvalidWorkflow(format!(
                "transition {} requires a standing policy for export",
                transition.id
            ))
        })?;
        let policy = load_standing_policy(self.store, self.index, policy_ref)?;
        let target = state.active_objects.first().cloned().ok_or_else(|| {
            ExecError::MissingInput("export requires an active object".to_string())
        })?;
        let target_object = self.store.read_version(&target.version_ref())?;
        let registry = load_registry(system)?;
        match export_allowed(&policy, &registry, &target_object.envelope.standing) {
            Ok(()) => {
                record_transition(
                    record,
                    transition.id.clone(),
                    "export_permitted",
                    vec![target],
                    vec![],
                    Some("export legality check passed".to_string()),
                );
                Ok(())
            }
            Err(error) => {
                if let Some(event) = escalation_for_trigger(
                    &policy,
                    "attempted_export_without_review",
                    Some(target.clone()),
                ) {
                    let stored_event = GovernanceService::create_governance_event_object(event)?;
                    write_object_and_index(self.store, self.index, &stored_event)?;
                    let event_ref = stored_event.object_ref();
                    change_set_draft
                        .governance_events
                        .push(event_ref.id.clone());
                    state.governance_events.push(event_ref.clone());
                    record.governance_events.push(event_ref);
                }
                Err(ExecError::Governance(error))
            }
        }
    }

    fn record_provider_event(
        &self,
        record: &mut RunRecord,
        state: &mut ExecutionState,
        change_set_draft: &mut ChangeSetDraft,
        provider_record: earmark_core::ProviderRecord,
    ) -> Result<ObjectRef, ExecError> {
        let event = StoredObject::builder(
            Kind::Event,
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&provider_record)?),
        )
        .class("provider_record")
        .provenance(earmark_core::Provenance::direct_input("runtime"))
        .header(
            "title",
            format!("Provider result {}", provider_record.record_id),
        )
        .build()
        .map_err(ExecError::IncompleteExecution)?;
        let event_ref = write_object_and_index(self.store, self.index, &event)?;
        change_set_draft
            .governance_events
            .push(event_ref.id.clone());
        let event_object_ref = ObjectRef::new(
            event_ref.id,
            event_ref.version_id,
            Kind::Event,
            Some("provider_record".to_string()),
        );
        state.governance_events.push(event_object_ref.clone());
        record.governance_events.push(event_object_ref.clone());
        Ok(event_object_ref)
    }

    #[allow(clippy::too_many_arguments)]
    fn finalize_execution_failure(
        &self,
        record: &mut RunRecord,
        transition: &ExecutionTransition,
        assignment: &mut TransitionAssignment,
        stored_assignment_head: &earmark_store::StoredObject,
        change_set_draft: &ChangeSetDraft,
        filtered_inputs: &[ObjectRef],
        error: ExecError,
    ) -> Result<(), ExecError> {
        assignment.input_object_ids = filtered_inputs.iter().map(|obj| obj.id.clone()).collect();
        let change_set_id = ChangeSetId::new();
        let blocked_change_set_id = persist_change_set(
            self.store,
            ChangeSetPersistence {
                record,
                change_set_id: change_set_id.clone(),
                assignment,
                transition_id: &transition.id,
                draft: change_set_draft,
                validation_results: vec![ChangeSetValidationResult {
                    is_valid: false,
                    failures: vec![error.to_string()],
                    warnings: vec![],
                    info: vec![],
                }],
                handoff_manifest_id: None,
                index: self.index,
            },
        )?;

        let failure_error_type = match &error {
            ExecError::Provider(_) => "provider_error",
            ExecError::IncompleteExecution(_) => "execution_error",
            ExecError::MissingInput(_) => "missing_input",
            ExecError::MissingWorkSurface(_) => "missing_work_surface",
            ExecError::UnsupportedOperation(_) => "unsupported_operation",
            ExecError::GovernanceOperation(_) => "governance_error",
            ExecError::HandoffReconstruction(_) => "handoff_error",
            ExecError::Governance(_) => "governance_error",
            ExecError::MissingTransitionAssignment(_) => "missing_assignment",
            ExecError::InvalidTransitionAssignment(_) => "invalid_assignment",
            ExecError::MissingHandoffManifest(_) => "missing_handoff",
            ExecError::InvalidWorkflow(_) => "invalid_workflow",
            ExecError::InvalidRelationMode(_) => "invalid_relation_mode",
            ExecError::ConflictingContinuationSources(_) => "conflicting_continuation",
            _ => "execution_error",
        };

        let failure_ref = persist_transformation_failure(
            self.store,
            self.index,
            stored_assignment_head,
            assignment,
            Some(blocked_change_set_id.clone()),
            &error,
            failure_error_type,
        )?;

        assignment.status = AssignmentStatus::Blocked;
        assignment.blocked_reason = Some(format!(
            "execution error: {}; change_set {} (failure {})",
            error,
            blocked_change_set_id.as_str(),
            failure_ref.id.as_str()
        ));
        assignment.updated_at = Utc::now();
        persist_assignment_update(
            self.store,
            self.index,
            stored_assignment_head,
            assignment,
        )?;

        record_transition(
            record,
            transition.id.clone(),
            "execution_error",
            filtered_inputs.to_vec(),
            vec![failure_ref],
            Some(error.to_string()),
        );
        Err(error)
    }

    #[allow(clippy::too_many_arguments)]
    fn finalize_transition_success(
        &self,
        _request: &WorkflowRunRequest,
        system: &earmark_core::SystemDefinition,
        ir: &ExecutionIr,
        transition: &ExecutionTransition,
        state: &mut ExecutionState,
        record: &mut RunRecord,
        filtered_inputs: &[ObjectRef],
        assignment: &mut TransitionAssignment,
        stored_assignment_head: &earmark_store::StoredObject,
        change_set_draft: &mut ChangeSetDraft,
        synthetic_output_warning: Option<String>,
    ) -> Result<(), ExecError> {
        let change_set_id = ChangeSetId::new();
        let now_end = Utc::now();
        assignment.input_object_ids = filtered_inputs.iter().map(|obj| obj.id.clone()).collect();
        let (validation_result, standing_requests) = validate_transition_change_set(
            self.store,
            self.index,
            system,
            transition,
            assignment,
            change_set_draft,
        )?;

        let mut validation_result = validation_result;
        if let Some(warning) = synthetic_output_warning {
            validation_result.warnings.push(warning);
        }
        change_set_draft.standing_requests.extend(standing_requests);

        if !validation_result.is_valid {
            change_set_draft
                .blocked_operations
                .push(earmark_core::BlockedOperation {
                    reason: "validation_failed".to_string(),
                    operation: transition.id.clone(),
                });
            let blocked_change_set_id = persist_change_set(
                self.store,
                ChangeSetPersistence {
                    record,
                    change_set_id: change_set_id.clone(),
                    assignment,
                    transition_id: &transition.id,
                    draft: change_set_draft,
                    validation_results: vec![validation_result.clone()],
                    handoff_manifest_id: None,
                    index: self.index,
                },
            )?;

            let error = ExecError::IncompleteExecution(format!(
                "transition {} failed validation: {}",
                transition.id,
                validation_result.failures.join("; ")
            ));
            let failure_ref = persist_transformation_failure(
                self.store,
                self.index,
                stored_assignment_head,
                assignment,
                Some(blocked_change_set_id.clone()),
                &error,
                "validation_error",
            )?;

            assignment.status = AssignmentStatus::Blocked;
            assignment.blocked_reason = Some(format!(
                "validation failed; change_set {} (failure {})",
                blocked_change_set_id.as_str(),
                failure_ref.id.as_str()
            ));
            assignment.updated_at = now_end;
            persist_assignment_update(
                self.store,
                self.index,
                stored_assignment_head,
                assignment,
            )?;
            record_transition(
                record,
                transition.id.clone(),
                "validation_error",
                filtered_inputs.to_vec(),
                vec![failure_ref],
                Some(error.to_string()),
            );
            return Err(error);
        }

        let handoff_specs =
            derive_successor_handoff(self.store, self.index, system, ir, transition)?;
        let root_object_ids = if change_set_draft.created_objects.is_empty() {
            state
                .active_objects
                .iter()
                .map(|object| object.id.clone())
                .collect::<Vec<_>>()
        } else {
            change_set_draft.created_objects.clone()
        };

        let mut handoff_manifest_ids = Vec::new();
        for spec in handoff_specs {
            let handoff_manifest_id = earmark_core::HandoffManifestId::new();
            let mut ambiguities = change_set_draft.unresolved_ambiguities.clone();
            for request in &change_set_draft.standing_requests {
                if request.status == earmark_core::StandingRequestStatus::Proposed {
                    ambiguities.push(earmark_core::UnresolvedAmbiguity {
                        description: format!(
                            "standing request: {} {} -> {} for object {}",
                            request.dimension,
                            request.from_value,
                            request.to_value,
                            request.target_object_id.as_str()
                        ),
                        context: "standing_request".to_string(),
                    });
                }
            }

            let handoff = earmark_core::HandoffManifest {
                id: handoff_manifest_id.clone(),
                run_id: record.run_id.clone(),
                from_transition_id: transition.id.clone(),
                to_transition_id: spec.to_transition_id,
                source_change_set_id: change_set_id.clone(),
                source_assignment_id: Some(assignment.id.clone()),
                root_object_ids: root_object_ids.clone(),
                inherited_input_object_ids: filtered_inputs.iter().map(|obj| obj.id.clone()).collect(),
                newly_created_object_ids: change_set_draft.created_objects.clone(),
                newly_created_relation_ids: change_set_draft.created_relations.clone(),
                allowed_input_classes: spec.allowed_input_classes,
                allowed_output_classes: spec.allowed_output_classes,
                allowed_relation_types: spec.allowed_relation_types,
                standing_constraints: spec.standing_constraints,
                unresolved_ambiguities: ambiguities,
                blocked_conditions: vec![],
                required_checks: spec.required_checks,
                compiled_context_template_id: spec.compiled_context_template_id,
                created_at: now_end,
            };

            let stored_handoff = StoredObject::builder(
                Kind::HandoffManifest,
                StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&handoff)?),
            )
            .id(handoff.id.as_object_id())
            .class("handoff_manifest")
            .provenance(earmark_core::Provenance::direct_input("execution_engine"))
            .header("title", format!("HandoffManifest {}", handoff.id.as_str()))
            .build()
            .map_err(ExecError::IncompleteExecution)?;
            write_object_and_index(self.store, self.index, &stored_handoff)?;
            handoff_manifest_ids.push(handoff_manifest_id);
        }

        persist_change_set(
            self.store,
            ChangeSetPersistence {
                record,
                change_set_id,
                assignment,
                transition_id: &transition.id,
                draft: change_set_draft,
                validation_results: vec![validation_result],
                handoff_manifest_id: None,
                index: self.index,
            },
        )?;
        record.manifests.extend(handoff_manifest_ids);

        assignment.status = AssignmentStatus::Completed;
        assignment.updated_at = now_end;
        assignment.completed_at = Some(now_end);
        persist_assignment_update(
            self.store,
            self.index,
            stored_assignment_head,
            assignment,
        )?;

        Ok(())
    }
}

fn load_registry(
    system: &earmark_core::SystemDefinition,
) -> Result<earmark_core::StandingRegistry, ExecError> {
    earmark_core::StandingRegistry::from_system_definition(system).map_err(ExecError::Core)
}
