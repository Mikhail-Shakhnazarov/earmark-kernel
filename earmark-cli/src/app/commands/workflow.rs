use crate::app::common::{CliError, CommandContext};
use crate::app::{
    emit, list_assignments_by_run, list_change_sets_by_run, list_failures_by_run,
    list_handoffs_by_run, resolve_object_ref, resolve_system_version_ref,
    resolve_workflow_version_ref,
};
use crate::cli::{WorkflowAction, WorkflowCommand};
use crate::config::resolve_system_id;
use earmark_exec::{ExecutionEngine, WorkflowRunRequest};
use serde_json::json;

pub fn handle(ctx: &CommandContext, command: &WorkflowCommand) -> Result<(), CliError> {
    let store = ctx.store;
    let index = ctx.index.as_ref().expect("index required for workflow");
    let provider_registry = ctx.provider_registry;
    let config = ctx.config;
    let as_json = ctx.as_json;

    match &command.action {
        WorkflowAction::Run(args) => {
            let system_id = resolve_system_id(args.system_id.as_deref(), config).ok_or_else(|| {
                CliError::argument(
                    "system id required: pass --system-id, set EM_SYSTEM_ID, or set default_system_id in config"
                )
            })?;
            tracing::info!(
                workflow_id = %args.workflow_id,
                system_id = %system_id,
                input_count = args.inputs.len(),
                "starting workflow run dispatch"
            );
            let workflow = resolve_workflow_version_ref(
                store,
                index,
                &args.workflow_id,
                args.version_id.as_deref(),
            )?;
            let system = resolve_system_version_ref(index, &system_id)?;
            let inputs = args
                .inputs
                .iter()
                .map(|object_id| resolve_object_ref(store, object_id))
                .collect::<Result<Vec<_>, _>>()?;
            let engine = ExecutionEngine {
                store,
                index,
                provider_service: provider_registry,
            };
            let outcome = engine.run_workflow(WorkflowRunRequest {
                run_id: format!(
                    "run_{}",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                system_definition: system,
                workflow,
                inputs,
                handoff_manifest: args
                    .handoff
                    .as_ref()
                    .map(|h| earmark_core::HandoffManifestId::parse(h.clone()))
                    .transpose()?,
                transition_assignment: args
                    .assignment
                    .as_ref()
                    .map(|a| earmark_core::TransitionAssignmentId::parse(a.clone()))
                    .transpose()?,
                operator_approved: args.approve_review,
            })?;
            let assignments = list_assignments_by_run(store, &outcome.record.run_id)?
                .into_iter()
                .map(|assignment| assignment.id.as_str().to_string())
                .collect::<Vec<_>>();
            let change_sets = list_change_sets_by_run(store, &outcome.record.run_id)?
                .into_iter()
                .map(|change_set| change_set.id.as_str().to_string())
                .collect::<Vec<_>>();
            let handoffs = list_handoffs_by_run(store, &outcome.record.run_id)?
                .into_iter()
                .map(|handoff| handoff.id.as_str().to_string())
                .collect::<Vec<_>>();
            let failures = list_failures_by_run(store, &outcome.record.run_id)?
                .into_iter()
                .collect::<Vec<_>>();
            emit(
                as_json,
                json!({
                    "ok": true,
                    "run_id": outcome.record.run_id,
                    "summary": "workflow run completed",
                    "status": format!("{:?}", outcome.record.status).to_lowercase(),
                    "event_count": outcome.record.events.len(),
                    "packet_count": outcome.emitted_packets.len(),
                    "output_count": outcome.emitted_objects.len(),
                    "governance_event_count": outcome.governance_events.len(),
                    "created_assignments": assignments,
                    "created_change_sets": change_sets,
                    "created_handoffs": handoffs,
                    "created_failures": failures,
                    "next_commands": [
                        format!("em run timeline {}", outcome.record.run_id),
                        format!("em run artifacts {}", outcome.record.run_id),
                    ],
                }),
            );
        }
    }
    Ok(())
}
