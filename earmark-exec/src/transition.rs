use std::collections::BTreeMap;

use chrono::Utc;
use earmark_connected_context::CompiledContextCompiler;
use earmark_core::{
    AssignmentStatus, ChangeSetDraft, ChangeSetId, ChangeSetValidationResult, Kind, ObjectRef,
    ProviderRequest, RunRecord, TransitionAssignment, TransitionAssignmentId,
    WorkPacketConstraints,
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
use crate::provider::{
    provider_metadata_synthetic_source, provider_record_from_failure,
    provider_record_from_response, provider_response_is_synthetic, resolve_provider_profile,
    ProviderMode,
};
use crate::resolution::{
    load_instruction, load_provider_profile, load_standing_policy, resolve_version_for_kind,
};
use crate::state::ExecutionState;
use crate::validation::validate_transition_change_set;

impl<'a, S: CanonicalStore> ExecutionEngine<'a, S> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_transition<C: CompiledContextCompiler<S>>(
        &self,
        request: &WorkflowRunRequest,
        system: &earmark_core::SystemDefinition,
        ir: &ExecutionIr,
        transition: &ExecutionTransition,
        state: &mut ExecutionState,
        record: &mut RunRecord,
        context_compiler: &C,
    ) -> Result<(), ExecError> {
        let store = self.store;
        let index = self.index;

        reject_duplicate_active_assignment(store, &record.run_id, &transition.id)?;

        let assignment_id = TransitionAssignmentId::new();
        let now = Utc::now();
        let mut assignment = TransitionAssignment {
            id: assignment_id.clone(),
            run_id: record.run_id.clone(),
            transition_id: transition.id.clone(),
            assigned_to: "execution_engine".to_string(),
            status: AssignmentStatus::Assigned,
            input_object_ids: state.active_objects.iter().map(|r| r.id.clone()).collect(),
            handoff_manifest_id: None,
            event_ids: vec![],
            blocked_reason: None,
            completion_change_set_id: None,
            assigned_at: now,
            updated_at: now,
            expires_at: None,
            completed_at: None,
        };

        let stored_assignment = StoredObject::new(
            Kind::TransitionAssignment,
            Some("transition_assignment".to_string()),
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("execution_engine"),
            BTreeMap::from([(
                "title".to_string(),
                earmark_core::HeaderValue::String(format!("Assignment {}", assignment_id.as_str())),
            )]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&assignment)?),
            vec![],
        );
        let assignment_version_ref = store.write_object(&stored_assignment)?;
        let stored_assignment_head = store.read_version(&assignment_version_ref)?;
        record.assignments.push(assignment_id.clone());

        let mut change_set_draft = ChangeSetDraft {
            created_objects: vec![],
            created_relations: vec![],
            updated_objects: vec![],
            governance_events: vec![],
            standing_requests: vec![],
            blocked_operations: vec![],
            unresolved_ambiguities: vec![],
            rejected_candidates: vec![],
        };
        let mut synthetic_output_warning: Option<String> = None;

        let filtered_inputs: Vec<ObjectRef> = if transition.input_contracts.is_empty() {
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
        };

        let exec_result: Result<(), ExecError> = (|| match transition.operation.as_str() {
            "compile_context" => {
                let template_ref = transition.compiled_context.as_ref().ok_or_else(|| {
                    ExecError::InvalidWorkflow(format!(
                        "transition {} requires a compiled context reference",
                        transition.id
                    ))
                })?;
                let resolved_template = resolve_version_for_kind(
                    store,
                    index,
                    template_ref,
                    Kind::CompiledContextTemplate,
                )?;
                let manifest = context_compiler.compile(store, index, &resolved_template, None)?;
                let work_packet = work_packet_from_compiled_context(
                    request,
                    transition,
                    &manifest,
                    WorkPacketConstraints {
                        standing_requirements: BTreeMap::new(),
                        review_requirements: vec![],
                        prohibited_operations: vec![],
                        export_permitted: true,
                    },
                    filtered_inputs.clone(),
                );
                let work_packet_object = store_work_packet(store, &work_packet)?;
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
                    filtered_inputs.clone(),
                    vec![work_packet_ref],
                    Some("work surface compiled".to_string()),
                );
                Ok(())
            }
            "transform" => {
                let instruction_ref = transition.instruction.as_ref().ok_or_else(|| {
                    ExecError::InvalidWorkflow(format!(
                        "transition {} requires an instruction reference",
                        transition.id
                    ))
                })?;
                let resolved_instruction_ref =
                    resolve_version_for_kind(store, index, instruction_ref, Kind::Instruction)?;
                let instruction = load_instruction(store, index, &resolved_instruction_ref)?;
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
                        export_permitted: true,
                    },
                    filtered_inputs.clone(),
                );
                let work_packet_object = store_work_packet(store, &work_packet)?;
                let work_packet_ref = work_packet_object.object_ref();
                change_set_draft
                    .created_objects
                    .push(work_packet_ref.id.clone());
                state.emitted_packets.push(work_packet_ref.clone());
                record.work_packets.push(work_packet_ref.clone());

                let output_class = transition
                    .output_contracts
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "candidate_output".to_string());

                let artifacts = match provider_mode {
                    ProviderMode::LocalExecution => {
                        crate::persistence::create_local_transform_output(
                            store,
                            index,
                            &instruction,
                            &output_class,
                            &filtered_inputs,
                            &resolved_instruction_ref,
                        )?
                    }
                    ProviderMode::Delegated(profile_ref) => {
                        let profile = load_provider_profile(store, index, &profile_ref)?;
                        let provider_request = ProviderRequest {
                            request_id: format!("req_{}", uuid_like()),
                            run_id: record.run_id.clone(),
                            work_packet: work_packet_ref.clone(),
                            provider_profile: profile_ref.clone(),
                            instruction_text: instruction.body.as_str().to_string(),
                            work_surface_manifest: state
                                .compiled_context
                                .as_ref()
                                .map(|surface| work_surface_manifest_path(store, surface)),
                            inputs: filtered_inputs.clone(),
                            response_contract: profile.response_contract.clone(),
                            issued_at: Utc::now(),
                        };

                        match self
                            .provider_service
                            .provide(&profile, provider_request.clone())
                        {
                            Ok(mut outcome) => {
                                let response = outcome.response.take().ok_or_else(|| {
                                    ExecError::Provider(ProviderFailure::new(
                                        ProviderFailureKind::MalformedResponse,
                                        "delegated outcome did not contain a response",
                                    ))
                                })?;
                                let provider_record = provider_record_from_response(
                                    &provider_request,
                                    &profile,
                                    &response,
                                    None,
                                );
                                if provider_response_is_synthetic(&response) {
                                    let source =
                                        provider_metadata_synthetic_source(&response.metadata)
                                            .unwrap_or_else(|| "mock_provider".to_string());
                                    synthetic_output_warning = Some(format!(
                                        "synthetic provider output detected (source: {}); artifact is not production evidence",
                                        source
                                    ));
                                }
                                let event = StoredObject::new(
                                    Kind::Event,
                                    Some("provider_record".to_string()),
                                    earmark_core::Standing::default(),
                                    earmark_core::Provenance::direct_input("runtime"),
                                    BTreeMap::from([(
                                        "title".to_string(),
                                        earmark_core::HeaderValue::String(format!(
                                            "Provider result {}",
                                            provider_record.record_id
                                        )),
                                    )]),
                                    StoredPayload::from_json_bytes(serde_json::to_vec_pretty(
                                        &provider_record,
                                    )?),
                                    vec![],
                                );
                                let event_ref = store.write_object(&event)?;
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
                                record.governance_events.push(event_object_ref);

                                crate::persistence::create_delegated_transform_output(
                                    store,
                                    index,
                                    &instruction,
                                    &output_class,
                                    &filtered_inputs,
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
                                let event = StoredObject::new(
                                    Kind::Event,
                                    Some("provider_record".to_string()),
                                    earmark_core::Standing::default(),
                                    earmark_core::Provenance::direct_input("runtime"),
                                    BTreeMap::from([(
                                        "title".to_string(),
                                        earmark_core::HeaderValue::String(format!(
                                            "Provider failure {}",
                                            provider_record.record_id
                                        )),
                                    )]),
                                    StoredPayload::from_json_bytes(serde_json::to_vec_pretty(
                                        &provider_record,
                                    )?),
                                    vec![],
                                );
                                let event_ref = store.write_object(&event)?;
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
                                record.governance_events.push(event_object_ref);
                                record_transition(
                                    record,
                                    transition.id.clone(),
                                    "provider_failed",
                                    filtered_inputs.clone(),
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
                    filtered_inputs.clone(),
                    vec![artifacts.output],
                    Some(format!("execution policy {}", instruction.execution_policy)),
                );
                Ok(())
            }
            "review" => {
                let target = state.active_objects.first().cloned().ok_or_else(|| {
                    ExecError::MissingInput("review requires a target object".to_string())
                })?;
                let review = GovernanceService::create_review_object(
                    store,
                    target.clone(),
                    request.operator_approved,
                    Some("review recorded by execution engine".to_string()),
                )?;
                let review_ref = review.object_ref();
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
            "export" => {
                let policy_ref = transition.policy.as_ref().ok_or_else(|| {
                    ExecError::InvalidWorkflow(format!(
                        "transition {} requires a standing policy for export",
                        transition.id
                    ))
                })?;
                let policy = load_standing_policy(store, index, policy_ref)?;
                let target = state.active_objects.first().cloned().ok_or_else(|| {
                    ExecError::MissingInput("export requires an active object".to_string())
                })?;
                let target_object = store.read_version(&target.version_ref())?;
                match export_allowed(&policy, &target_object.envelope.standing) {
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
                            let stored_event =
                                GovernanceService::create_governance_event_object(store, event)?;
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
            other => Err(ExecError::UnsupportedOperation(other.to_string())),
        })();

        if let Err(error) = exec_result {
            let change_set_id = ChangeSetId::new();
            let blocked_change_set_id = persist_change_set(
                store,
                ChangeSetPersistence {
                    record,
                    change_set_id: change_set_id.clone(),
                    assignment: &assignment,
                    transition_id: &transition.id,
                    draft: &change_set_draft,
                    validation_results: vec![ChangeSetValidationResult {
                        is_valid: false,
                        failures: vec![error.to_string()],
                        warnings: vec![],
                        info: vec![],
                    }],
                    handoff_manifest_id: None,
                    index,
                },
            )?;

            let failure_ref = persist_transformation_failure(
                store,
                index,
                &stored_assignment_head,
                &assignment,
                Some(blocked_change_set_id.clone()),
                &error,
            )?;

            assignment.status = AssignmentStatus::Blocked;
            assignment.blocked_reason = Some(format!(
                "execution error: {}; change_set {} (failure {})",
                error,
                blocked_change_set_id.as_str(),
                failure_ref.id.as_str()
            ));
            assignment.updated_at = Utc::now();
            persist_assignment_update(store, &stored_assignment_head, &assignment)?;

            record_transition(
                record,
                transition.id.clone(),
                "execution_error",
                vec![],
                vec![failure_ref],
                Some(error.to_string()),
            );
            return Err(error);
        }

        {
            let change_set_id = ChangeSetId::new();
            let now_end = Utc::now();
            let (validation_result, standing_requests) = validate_transition_change_set(
                store,
                index,
                system,
                transition,
                &assignment,
                &change_set_draft,
            )?;
            let mut validation_result = validation_result;
            if let Some(warning) = synthetic_output_warning.clone() {
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
                    store,
                    ChangeSetPersistence {
                        record,
                        change_set_id: change_set_id.clone(),
                        assignment: &assignment,
                        transition_id: &transition.id,
                        draft: &change_set_draft,
                        validation_results: vec![validation_result.clone()],
                        handoff_manifest_id: None,
                        index,
                    },
                )?;

                let error = ExecError::IncompleteExecution(format!(
                    "transition {} failed validation: {}",
                    transition.id,
                    validation_result.failures.join("; ")
                ));
                let failure_ref = persist_transformation_failure(
                    store,
                    index,
                    &stored_assignment_head,
                    &assignment,
                    Some(blocked_change_set_id.clone()),
                    &error,
                )?;

                assignment.status = AssignmentStatus::Blocked;
                assignment.blocked_reason = Some(format!(
                    "validation failed; change_set {} (failure {})",
                    blocked_change_set_id.as_str(),
                    failure_ref.id.as_str()
                ));
                assignment.updated_at = now_end;
                persist_assignment_update(store, &stored_assignment_head, &assignment)?;
                return Err(error);
            }
            let handoff_specs = derive_successor_handoff(store, index, system, ir, transition)?;
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
                    source_assignment_id: Some(assignment_id.clone()),
                    root_object_ids: root_object_ids.clone(),
                    inherited_input_object_ids: filtered_inputs
                        .iter()
                        .map(|obj| obj.id.clone())
                        .collect(),
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

                let stored_handoff = StoredObject::new(
                    Kind::HandoffManifest,
                    Some("handoff_manifest".to_string()),
                    earmark_core::Standing::default(),
                    earmark_core::Provenance::direct_input("execution_engine"),
                    BTreeMap::from([(
                        "title".to_string(),
                        earmark_core::HeaderValue::String(format!(
                            "Handoff for {}",
                            handoff.id.as_str()
                        )),
                    )]),
                    StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&handoff)?),
                    vec![],
                );
                store.write_object(&stored_handoff)?;
                handoff_manifest_ids.push(handoff_manifest_id);
            }

            persist_change_set(
                store,
                ChangeSetPersistence {
                    record,
                    change_set_id,
                    assignment: &assignment,
                    transition_id: &transition.id,
                    draft: &change_set_draft,
                    validation_results: vec![validation_result],
                    handoff_manifest_id: None,
                    index,
                },
            )?;
            record.manifests.extend(handoff_manifest_ids);

            assignment.status = AssignmentStatus::Completed;
            assignment.updated_at = now_end;
            assignment.completed_at = Some(now_end);
            persist_assignment_update(store, &stored_assignment_head, &assignment)?;
        }

        Ok(())
    }
}
