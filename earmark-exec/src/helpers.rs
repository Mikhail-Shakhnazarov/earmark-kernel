use chrono::Utc;
use earmark_connected_context::WorkSurfaceManifest;
use earmark_core::{
    projection::project_visibility, Kind, ObjectRef, RunRecord, RunStatus, StandingRegistry,
    TokenRecord, TransitionAssignment, VersionRef, WorkPacket, WorkPacketConstraints,
    WorkSurfaceRef, WorkflowDefinition,
};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use std::collections::{BTreeMap, BTreeSet};

use crate::error::ExecError;
use crate::ir::{ExecutionEdge, ExecutionIr, ExecutionTransition, WorkflowRunRequest};
use crate::persistence_helpers::write_object_and_index;
use earmark_index::DerivedIndex;

pub(crate) fn estimate_tokens_approx(text: &str) -> u32 {
    ((text.chars().count() + 3) / 4) as u32
}

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

pub fn work_packet_from_compiled_context(
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
        purpose: transition.operation.as_str().to_string(),
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
        advisory_warnings: vec![],
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

pub fn render_provider_context(manifest: &WorkSurfaceManifest) -> String {
    let mut rendered = String::new();
    rendered.push_str("### Work Surface Context\n\n");
    rendered.push_str(&format!("Surface ID: {}\n", manifest.surface_id));
    rendered.push_str(&format!("Object Count: {}\n", manifest.objects.len()));
    rendered.push('\n');
    rendered
}

pub fn render_provider_input<S: CanonicalStore>(
    store: &S,
    instruction: &earmark_core::InstructionPayload,
    manifest: Option<&WorkSurfaceManifest>,
    inputs: &[ObjectRef],
    profile: &earmark_core::ProviderProfile,
    registry: &StandingRegistry,
) -> Result<String, ExecError> {
    let mut rendered = String::new();

    // 1. Instruction
    rendered.push_str("### Instruction\n\n");
    rendered.push_str(instruction.body.as_str());
    rendered.push_str("\n\n");

    // 2. Context
    if let Some(m) = manifest {
        rendered.push_str(&render_provider_context(m));
    }

    // 3. Inputs / Evidence
    rendered.push_str("### Input Evidence\n\n");

    // Build the evidence set: manifest objects if present, otherwise active inputs.
    // Deduplicate by (id, version_id).
    let mut to_render_refs: Vec<ObjectRef> = if let Some(m) = manifest {
        m.objects.iter().map(|o| o.object.clone()).collect()
    } else {
        inputs.to_vec()
    };

    // If allow_work_surface_only is false, ensure all active inputs are also included
    if !profile.exposure.allow_work_surface_only {
        for input in inputs {
            if !to_render_refs
                .iter()
                .any(|r| r.id == input.id && r.version_id == input.version_id)
            {
                to_render_refs.push(input.clone());
            }
        }
    }

    let active_refs: BTreeSet<(earmark_core::ObjectId, earmark_core::VersionId)> = inputs
        .iter()
        .map(|o| (o.id.clone(), o.version_id.clone()))
        .collect();

    for (i, obj_ref) in to_render_refs.iter().enumerate() {
        let obj = store.read_version(&obj_ref.version_ref()).map_err(|e| {
            ExecError::IncompleteExecution(format!(
                "failed to read input object {}: {}",
                obj_ref.id, e
            ))
        })?;

        // Standing visibility gate — checked before any metadata rendering
        let vis = project_visibility(&obj.envelope.standing, registry);

        if !vis.expose_to_provider {
            rendered.push_str(&format!("#### Evidence [{}]\n", i + 1));
            rendered.push_str(
                "Evidence item omitted from provider input by standing visibility policy.\n",
            );
            rendered.push('\n');
            continue;
        }

        let is_active = active_refs.contains(&(obj_ref.id.clone(), obj_ref.version_id.clone()));
        let active_marker = if is_active { " [Active Input]" } else { "" };

        rendered.push_str(&format!("#### Evidence [{}]{}\n", i + 1, active_marker));
        rendered.push_str(&format!("ID: {}\n", obj_ref.id));
        rendered.push_str(&format!("Kind: {}\n", obj_ref.kind.as_str()));
        rendered.push_str(&format!(
            "Class: {}\n",
            obj_ref.class.as_deref().unwrap_or("unknown")
        ));

        if let Some(title) = obj.envelope.headers.get("title") {
            rendered.push_str(&format!("Title: {:?}\n", title));
        }

        // Determine if this is a structured declaration
        let is_structured = match obj_ref.kind {
            Kind::Instruction
            | Kind::Policy
            | Kind::Workflow
            | Kind::CompiledContextTemplate
            | Kind::ProviderProfile
            | Kind::SystemDefinition => true,
            Kind::Object if obj_ref.class.as_deref() == Some("class_definition") => true,
            _ => false,
        };

        let profile_permits = if is_structured {
            profile.exposure.allow_structured_declarations
        } else {
            profile.exposure.allow_prose_objects
        };

        // Second gate: provider profile exposure must also permit
        if profile_permits {
            if let Ok(payload_str) = String::from_utf8(obj.payload.bytes.clone()) {
                rendered.push_str("\nPayload:\n---\n");
                rendered.push_str(&payload_str);
                rendered.push_str("\n---\n");
            } else {
                rendered.push_str("\n(Binary payload not displayed)\n");
            }
        } else {
            let reason = if is_structured {
                "Structured declarations hidden by exposure policy"
            } else {
                "Payload content hidden by exposure policy"
            };
            rendered.push_str(&format!("\n({})\n", reason));
        }
        rendered.push('\n');
    }

    Ok(rendered)
}
