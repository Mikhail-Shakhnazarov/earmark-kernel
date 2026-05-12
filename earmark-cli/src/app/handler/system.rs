use earmark_declarations::activate_system_definition;
use earmark_index::DerivedIndex;
use earmark_store::GitCanonicalStore;
use serde_json::json;

use crate::app::{emit, register_declaration_file, CliError};
use crate::cli::{DeclarationKind, SystemAction, SystemCommand};
use crate::config::{resolve_system_id, CliConfig};

pub fn handle(
    store: &GitCanonicalStore,
    index: &DerivedIndex,
    config: &CliConfig,
    as_json: bool,
    command: SystemCommand,
) -> Result<(), CliError> {
    match command.action {
        SystemAction::Register { manifest } => {
            tracing::info!(manifest = %manifest.display(), "registering system declaration");
            let version_ref =
                register_declaration_file(store, None, DeclarationKind::System, &manifest)?;
            index.rebuild_from_store(store)?;
            emit(
                as_json,
                json!({
                    "ok": true,
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
                    "ok": true,
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
