use crate::app::common::{CliError, CommandContext};
use crate::app::{emit, register_declaration_file};
use crate::cli::{DeclarationKind, SystemAction, SystemCommand};
use crate::config::resolve_system_id;
use earmark_declarations::activate_system_definition;
use serde_json::json;

pub fn handle(ctx: &CommandContext, command: &SystemCommand) -> Result<(), CliError> {
    let store = ctx.store;
    let index = ctx
        .index
        .as_ref()
        .expect("index required for system commands");
    let config = ctx.config;
    let as_json = ctx.as_json;
    let actor = ctx.actor;

    match &command.action {
        SystemAction::Register { manifest } => {
            tracing::info!(manifest = %manifest.display(), "registering system declaration");
            let version_ref =
                register_declaration_file(store, None, DeclarationKind::System, manifest, None, actor)?;
            index.rebuild_from_store(store)?;
            emit(
                as_json,
                json!({
                    "kind": "system_registration",
                    "object_id": version_ref.id.as_str(),
                    "version_id": version_ref.version_id.as_str(),
                }),
            );
        }
        SystemAction::Activate { system_id } => {
            let system_id = resolve_system_id(system_id.as_deref(), config).ok_or_else(|| {
                CliError::argument(
                    "system id required: pass --system-id, set EM_SYSTEM_ID, or set default_system_id in config"
                )
            })?;
            let active = activate_system_definition(store, index, &system_id)?;
            emit(
                as_json,
                json!({
                    "namespace": active.namespace,
                    "system_id": active.system_id,
                    "object_id": active.object_id,
                    "version_id": active.version_id,
                    "activated_at": active.activated_at,
                }),
            );
        }
    }
    Ok(())
}
