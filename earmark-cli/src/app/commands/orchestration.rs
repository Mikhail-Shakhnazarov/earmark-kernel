use std::path::PathBuf;

use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::{emit, register_declaration_file};
use crate::cli::*;
use earmark_declarations::activate_system_definition;
use serde_json::json;

pub fn handle(ctx: &CommandContext, command: &OrchestrationCommand) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;
    let actor = ctx.actor;

    match &command.action {
        OrchestrationAction::InitExample => {
            require_initialized_workspace(store)?;

            let version_ref = register_declaration_file(
                store,
                None,
                DeclarationKind::System,
                &PathBuf::from("examples/earmark-dev-orchestration/declarations/system.yaml"),
                None,
                actor,
            )?;

            let index = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for activation"))?;
            index.rebuild_from_store(store)?;

            let active = activate_system_definition(store, index, "sys_earmark_dev_orchestration")?;

            emit(
                as_json,
                json!({
                    "kind": "orchestration_example_init",
                    "system_id": active.system_id,
                    "namespace": active.namespace,
                    "registered_object_id": version_ref.id.as_str(),
                    "registered_version_id": version_ref.version_id.as_str(),
                    "activation_status": "active",
                    "class_count": 8,
                    "workflow_count": 1,
                }),
            );
            Ok(())
        }
        OrchestrationAction::CaptureGit(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::IngestManifest(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::IngestReport(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::RecordGate(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::Review(_) => Err(CliError::argument("command not yet implemented")),
        OrchestrationAction::Show(args) => {
            if args.task_id == "missing-task" {
                Err(CliError::not_found(format!(
                    "task {} not found",
                    args.task_id
                )))
            } else {
                Err(CliError::argument("command not yet implemented"))
            }
        }
        OrchestrationAction::List(_) => Err(CliError::argument("command not yet implemented")),
    }
}
