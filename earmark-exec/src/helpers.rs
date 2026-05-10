use chrono::Utc;
use earmark_connected_context::WorkSurfaceManifest;
use earmark_core::{
    Kind, ObjectRef, RunRecord, RunStatus, TokenRecord, TransitionAssignment, VersionRef,
    WorkPacket, WorkPacketConstraints, WorkSurfaceRef, WorkflowDefinition,
};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use std::collections::{BTreeMap, BTreeSet};

use crate::error::ExecError;
use crate::ir::{ExecutionEdge, ExecutionIr, ExecutionTransition, WorkflowRunRequest};
use crate::persistence_helpers::write_object_and_index;
use earmark_index::DerivedIndex;

pub(crate) fn compile_workflow(workflow: &WorkflowDefinition) -> Result<ExecutionIr, ExecError> {
    let mut seen_ids = BTreeSet::new();
    let transitions = workflow
        .operations
        .iter()
        .map(|operation| {
            if !seen_ids.insert(operation.id.clone()) {
                return Err(ExecError::InvalidWorkflow(format!(
                    "duplicate transition id {}",
                    operation.id
                )));
            }
            Ok(ExecutionTransition {
                id: operation.id.clone(),
                operation: operation.kind.clone(),
                input_contracts: operation.input_contracts.clone(),
                output_contracts: operation.output_contracts.clone(),
                instruction: operation.instruction.clone(),
                compiled_context: operation.compiled_context.clone(),
                policy: operation.policy.clone(),
                provider_profile: operation.provider_profile.clone(),
            })
        })
        .collect::<Result<Vec<_>, ExecError>>()?;

    let edges = workflow
        .edges
        .iter()
        .map(|t| ExecutionEdge {
            from: t.from.clone(),
            to: t.to.clone(),
            condition: t.condition.clone(),
        })
        .collect::<Vec<_>>();

    Ok(ExecutionIr {
        transitions,
        guards: workflow.guards.clone(),
        edges,
    })
}

pub(crate) fn new_run_record(
    run_id: String,
    system_definition: VersionRef,
    workflow: VersionRef,
    initial_marking: Vec<TokenRecord>,
) -> RunRecord {
    RunRecord {
        run_id,
        system_definition,
        workflow,
        status: RunStatus::Running,
        initial_marking,
        final_marking: vec![],
        assignments: vec![],
        change_sets: vec![],
        work_packets: vec![],
        governance_events: vec![],
        events: vec![],
        manifests: vec![],
        started_at: Utc::now(),
        ended_at: None,
    }
}

pub(crate) fn record_transition(
    record: &mut RunRecord,
    transition_id: String,
    event_type: &str,
    consumed: Vec<ObjectRef>,
    produced: Vec<ObjectRef>,
    message: Option<String>,
) {
    record.events.push(earmark_core::RunEvent {
        event_id: format!("ev_{}", uuid_like()),
        transition: transition_id,
        event_type: event_type.to_string(),
        inputs: consumed,
        outputs: produced,
        message,
        timestamp: Utc::now(),
    });
}

pub(crate) fn work_packet_from_compiled_context(
    request: &WorkflowRunRequest,
    transition: &ExecutionTransition,
    manifest: &WorkSurfaceManifest,
    constraints: WorkPacketConstraints,
    inputs: Vec<ObjectRef>,
) -> WorkPacket {
    WorkPacket {
        work_packet_id: format!("wp_{}", uuid_like()),
        run_id: request.run_id.clone(),
        work_packet_type: "execution".to_string(),
        purpose: transition.operation.clone(),
        system_definition: request.system_definition.clone(),
        workflow: Some(request.workflow.clone()),
        instruction: transition.instruction.clone(),
        provider_profile: transition.provider_profile.clone(),
        inputs,
        compiled_contexts: vec![], // This would be populated if we linked existing contexts
        constraints,
        expected_outputs: transition.output_contracts.clone(),
        work_surface: Some(WorkSurfaceRef {
            surface_id: manifest.surface_id.clone(),
            manifest_path: format!(
                ".earmark/work_surfaces/{}/manifest.json",
                manifest.surface_id
            ),
            render_mode: "prose".to_string(), // Default
        }),
        created_at: Utc::now(),
    }
}

pub fn store_work_packet<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    work_packet: &WorkPacket,
) -> Result<StoredObject, ExecError> {
    let stored = StoredObject::new(
        Kind::WorkPacket,
        Some("work_packet".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("execution_engine"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String(format!("WorkPacket for {}", work_packet.purpose)),
        )]),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&work_packet)?),
        vec![],
    );
    write_object_and_index(store, index, &stored)?;
    Ok(stored)
}

pub(crate) fn reject_duplicate_active_assignment<S: CanonicalStore>(
    store: &S,
    run_id: &str,
    transition_id: &str,
) -> Result<(), ExecError> {
    let now = Utc::now();
    for object in store.scan_objects()? {
        if object.envelope.kind != Kind::TransitionAssignment {
            continue;
        }
        let assignment: TransitionAssignment = serde_json::from_slice(&object.payload.bytes)?;
        if assignment.run_id != run_id || assignment.transition_id != transition_id {
            continue;
        }

        // IMPORTANT: Only consider the current head version of the assignment
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id != object.envelope.version_id {
                continue;
            }
        }

        if assignment.status != earmark_core::AssignmentStatus::Assigned {
            continue;
        }
        let still_active = assignment
            .expires_at
            .map(|expires_at| expires_at > now)
            .unwrap_or(true);
        if still_active {
            return Err(ExecError::IncompleteExecution(format!(
                "transition {} in run {} is already actively assigned to {}",
                transition_id, run_id, assignment.assigned_to
            )));
        }
    }
    Ok(())
}

pub(crate) fn load_current_transition_assignment<S: CanonicalStore>(
    store: &S,
    assignment_id: &earmark_core::TransitionAssignmentId,
) -> Result<(StoredObject, TransitionAssignment), ExecError> {
    for object in store.scan_objects()? {
        if object.envelope.kind != Kind::TransitionAssignment {
            continue;
        }
        let assignment: TransitionAssignment = serde_json::from_slice(&object.payload.bytes)?;
        if &assignment.id != assignment_id {
            continue;
        }
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id == object.envelope.version_id {
                return Ok((object, assignment));
            }
        }
    }
    Err(ExecError::MissingTransitionAssignment(
        assignment_id.as_str().to_string(),
    ))
}

pub(crate) fn work_surface_manifest_path<S: CanonicalStore>(
    _store: &S,
    manifest: &WorkSurfaceManifest,
) -> String {
    format!(
        ".earmark/work_surfaces/{}/manifest.json",
        manifest.surface_id
    )
}

pub(crate) fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

pub(crate) fn uuid_like() -> String {
    format!("{}", Utc::now().timestamp_nanos_opt().unwrap_or_default())
}
