use chrono::Utc;
use earmark_connected_context::{
    CompiledContextCompiler, WorkSurfaceManifest, DEFAULT_COMPILED_CONTEXT_COMPILER,
};
use earmark_core::{Kind, RunStatus, ScalarOrRef, TokenRecord, VersionId, VersionRef};
use earmark_index::DerivedIndex;
use earmark_store::CanonicalStore;
use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::error::ExecError;
use crate::handoff::load_handoff;
use crate::helpers::{compile_workflow, new_run_record, record_transition};
use crate::ir::{WorkflowRunOutcome, WorkflowRunRequest};
use crate::persistence::persist_run_record;
use crate::provider::ProviderService;
use crate::resolution::{
    load_system_definition, load_workflow, resolve_continuation_inputs, resolve_version_for_kind,
};
use crate::state::ExecutionState;
use crate::validation::{
    deadlock_warnings, edge_condition_allows, entry_transition_ids, incoming_edges, outgoing_edges,
    reachability_warnings, transition_is_ready,
};

pub struct ExecutionEngine<'a, S: CanonicalStore> {
    pub store: &'a S,
    pub index: &'a DerivedIndex,
    pub provider_service: &'a dyn ProviderService,
}

impl<'a, S: CanonicalStore> ExecutionEngine<'a, S> {
    pub fn new(
        store: &'a S,
        index: &'a DerivedIndex,
        provider_service: &'a dyn ProviderService,
    ) -> Self {
        Self {
            store,
            index,
            provider_service,
        }
    }

    pub fn run_workflow(
        &self,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunOutcome, ExecError> {
        self.run_workflow_with_context_compiler(request, &DEFAULT_COMPILED_CONTEXT_COMPILER)
    }

    pub(crate) fn run_workflow_with_context_compiler<C: CompiledContextCompiler<S>>(
        &self,
        request: WorkflowRunRequest,
        context_compiler: &C,
    ) -> Result<WorkflowRunOutcome, ExecError> {
        let system = load_system_definition(self.store, self.index, &request.system_definition)?;
        let workflow = load_workflow(self.store, self.index, &request.workflow)?;
        let ir = compile_workflow(&workflow)?;
        let effective_inputs = resolve_continuation_inputs(self.store, self.index, &request)?;

        let warnings = reachability_warnings(&ir)
            .into_iter()
            .chain(deadlock_warnings(&ir))
            .collect::<Vec<_>>();

        let initial_marking = effective_inputs
            .iter()
            .cloned()
            .map(|input| TokenRecord {
                token_type: "object_ref".to_string(),
                value: ScalarOrRef::Object(input),
                place: "input".to_string(),
            })
            .collect::<Vec<_>>();

        let mut record = new_run_record(
            request.run_id.clone(),
            request.system_definition.clone(),
            request.workflow.clone(),
            initial_marking,
        );

        if !warnings.is_empty() {
            record_transition(
                &mut record,
                "analysis".to_string(),
                "warning",
                vec![],
                vec![],
                Some(warnings.join("; ")),
            );
        }

        let transition_map = ir
            .transitions
            .iter()
            .map(|transition| (transition.id.clone(), transition))
            .collect::<HashMap<_, _>>();
        let incoming = incoming_edges(&ir);
        let outgoing = outgoing_edges(&ir);

        let mut executed = BTreeSet::new();
        let mut ready_queue = VecDeque::new();
        let mut emitted_packets = Vec::new();
        let mut emitted_objects = Vec::new();
        let mut governance_events = Vec::new();
        let mut compiled_context: Option<WorkSurfaceManifest> = None;

        let initial_contracts_set = crate::validation::initial_contracts(&effective_inputs);
        let mut available_contracts = initial_contracts_set.clone();

        let ready_seed_ids: Vec<String> = if let Some(handoff_id) = &request.handoff_manifest {
            let handoff = load_handoff(self.store, handoff_id)?;

            if let Some(template_id) = &handoff.compiled_context_template_id {
                let template_ref = VersionRef::new(
                    template_id.clone(),
                    VersionId::parse("ver_00000000000000000000000000000000").unwrap(),
                );
                let resolved = resolve_version_for_kind(
                    self.store,
                    self.index,
                    &template_ref,
                    Kind::CompiledContextTemplate,
                )?;
                compiled_context =
                    Some(context_compiler.compile(self.store, self.index, &resolved, None)?);
            }

            match handoff.to_transition_id.clone() {
                Some(target_id) => {
                    if !transition_map.contains_key(&target_id) {
                        return Err(ExecError::InvalidWorkflow(format!(
                            "handoff {} targets transition {}, which is not present in workflow {}",
                            handoff_id.as_str(),
                            target_id,
                            request.workflow.id.as_str()
                        )));
                    }

                    record_transition(
                        &mut record,
                        target_id.clone(),
                        "continuation",
                        vec![],
                        vec![],
                        Some(format!("continued from handoff {}", handoff_id.as_str())),
                    );

                    let mut ancestors = BTreeSet::new();
                    let mut stack: Vec<String> = incoming
                        .get(&target_id)
                        .map(|edges| edges.iter().map(|e| e.from.clone()).collect())
                        .unwrap_or_default();
                    while let Some(ancestor_id) = stack.pop() {
                        if ancestors.insert(ancestor_id.clone()) {
                            if let Some(preds) = incoming.get(&ancestor_id) {
                                for edge in preds {
                                    stack.push(edge.from.clone());
                                }
                            }
                        }
                    }
                    executed.extend(ancestors);

                    vec![target_id]
                }
                None => entry_transition_ids(&ir),
            }
        } else {
            entry_transition_ids(&ir)
        };

        let mut final_marking = effective_inputs.clone();

        for transition_id in ready_seed_ids {
            let transition = transition_map.get(&transition_id).ok_or_else(|| {
                ExecError::InvalidWorkflow(format!(
                    "transition {} missing from workflow",
                    transition_id
                ))
            })?;
            if transition_is_ready(
                transition,
                &available_contracts,
                &executed,
                &incoming,
                &ir,
                &request,
                &effective_inputs,
            )? {
                ready_queue.push_back((transition_id.clone(), effective_inputs.clone()));
            }
        }

        while let Some((transition_id, mut current_marking)) = ready_queue.pop_front() {
            if executed.contains(&transition_id) {
                continue;
            }

            let transition = transition_map.get(&transition_id).ok_or_else(|| {
                ExecError::InvalidWorkflow(format!(
                    "transition {} missing from compiled graph",
                    transition_id
                ))
            })?;

            if !transition_is_ready(
                transition,
                &available_contracts,
                &executed,
                &incoming,
                &ir,
                &request,
                &current_marking,
            )? {
                continue;
            }

            let mut state = ExecutionState {
                active_objects: &mut current_marking,
                emitted_packets: &mut emitted_packets,
                emitted_objects: &mut emitted_objects,
                governance_events: &mut governance_events,
                compiled_context: &mut compiled_context,
            };

            if let Err(error) = self.execute_transition(
                &request,
                &system,
                &ir,
                transition,
                &mut state,
                &mut record,
                context_compiler,
            ) {
                record.status = RunStatus::Failed;
                record.final_marking = current_marking
                    .iter()
                    .cloned()
                    .map(|object| TokenRecord {
                        token_type: "object_ref".to_string(),
                        value: ScalarOrRef::Object(object),
                        place: "failed".to_string(),
                    })
                    .collect();
                record.ended_at = Some(Utc::now());
                persist_run_record(self.store, self.index, &record)?;
                return Err(error);
            }

            final_marking = current_marking.clone();

            executed.insert(transition_id.clone());
            for contract in &transition.output_contracts {
                available_contracts.insert(contract.clone());
            }

            for edge in outgoing.get(&transition_id).into_iter().flatten() {
                if !edge_condition_allows(
                    edge,
                    &ir,
                    &request,
                    &available_contracts,
                    &current_marking,
                )? {
                    record_transition(
                        &mut record,
                        transition_id.clone(),
                        "edge_blocked",
                        current_marking.clone(),
                        vec![],
                        Some(format!(
                            "edge {} -> {} blocked by condition {}",
                            edge.from,
                            edge.to,
                            edge.condition
                                .clone()
                                .unwrap_or_else(|| "<none>".to_string())
                        )),
                    );
                    continue;
                }

                let successor = transition_map.get(&edge.to).ok_or_else(|| {
                    ExecError::InvalidWorkflow(format!(
                        "successor transition {} missing from compiled graph",
                        edge.to
                    ))
                })?;

                if transition_is_ready(
                    successor,
                    &available_contracts,
                    &executed,
                    &incoming,
                    &ir,
                    &request,
                    &current_marking,
                )? {
                    ready_queue.push_back((edge.to.clone(), current_marking.clone()));
                }
            }
        }

        let remaining = ir
            .transitions
            .iter()
            .filter(|transition| !executed.contains(&transition.id))
            .map(|transition| transition.id.clone())
            .collect::<Vec<_>>();

        if !remaining.is_empty() {
            let message = format!(
                "workflow execution finished with {} transitions unreached",
                remaining.len()
            );
            record_transition(
                &mut record,
                "analysis".to_string(),
                "partial_execution",
                final_marking.clone(),
                vec![],
                Some(message),
            );
        }

        record.status = RunStatus::Completed;
        record.final_marking = final_marking
            .iter()
            .cloned()
            .map(|object| TokenRecord {
                token_type: "object_ref".to_string(),
                value: ScalarOrRef::Object(object),
                place: "completed".to_string(),
            })
            .collect();
        record.ended_at = Some(Utc::now());
        persist_run_record(self.store, self.index, &record)?;

        Ok(WorkflowRunOutcome {
            record: record.clone(),
            emitted_packets,
            emitted_objects,
            governance_events,
        })
    }
}
