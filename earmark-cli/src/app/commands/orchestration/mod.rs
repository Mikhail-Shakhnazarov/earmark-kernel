pub mod common;
pub mod ingest;
pub mod git;
pub mod gates;
pub mod review;
pub mod view;
pub mod adapters;

use crate::app::common::{CliError, CommandContext};
use crate::cli::{OrchestrationAction, OrchestrationCommand};
use earmark_store::ObjectStore;

pub fn handle(ctx: &mut CommandContext, command: &OrchestrationCommand) -> Result<(), CliError> {
    match &command.action {
        OrchestrationAction::InitExample(args) => handle_init_example(ctx, args),
        OrchestrationAction::CaptureGit(args) => git::handle_capture_git(ctx, args),
        OrchestrationAction::IngestManifest(args) => ingest::handle_ingest_manifest(ctx, args),
        OrchestrationAction::IngestReport(args) => ingest::handle_ingest_report(ctx, args),
        OrchestrationAction::IngestTask(args) => ingest::handle_ingest_task(ctx, args),
        OrchestrationAction::RecordGate(args) => gates::handle_record_gate(ctx, args),
        OrchestrationAction::Review(args) => review::handle_review(ctx, args),
        OrchestrationAction::Show(args) => view::handle_show(ctx, args),
        OrchestrationAction::List(args) => view::handle_list(ctx, args),
        OrchestrationAction::Timeline(args) => view::handle_timeline(ctx, args),
        OrchestrationAction::ExplainDispatch(args) => view::handle_explain_dispatch(ctx, args),
        OrchestrationAction::RecordContext(_args) => {
            Err(CliError::argument("record-context not yet implemented in modularized version".to_string()))
        }
    }
}

// Internal helper for init-example (could be moved to a separate file if it grows)
fn handle_init_example(ctx: &mut CommandContext, args: &crate::cli::InitExampleArgs) -> Result<(), CliError> {
    use std::path::PathBuf;
    use crate::app::common::require_initialized_workspace;
    use crate::app::{register_declaration_file, mirror_surface};
    use earmark_declarations::activate_system_definition;
    use crate::app::emit;
    use serde_json::json;

    let store = ctx.store;
    let as_json = ctx.as_json;
    let actor = ctx.actor;

    require_initialized_workspace(store)?;

    let relative_path = "examples/earmark-dev-orchestration/declarations/system.yaml";
    let mut resolved_path = PathBuf::from(relative_path);

    if let Some(ref root) = args.example_root {
        resolved_path = root.join("declarations/system.yaml");
    } else if !resolved_path.exists() {
        // Try to find repository root by looking for Cargo.toml
        let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut found = false;
        for _ in 0..10 {
            if current.join("Cargo.toml").exists() {
                let candidate = current.join(relative_path);
                if candidate.exists() {
                    resolved_path = candidate;
                    found = true;
                    break;
                }
            }
            if !current.pop() {
                break;
            }
        }
        if !found {
            return Err(CliError::not_found(format!(
                "could not find orchestration example at {}. Try passing --example-root",
                relative_path
            )));
        }
    }

    let version_ref = register_declaration_file(
        store,
        None,
        crate::cli::DeclarationKind::System,
        &resolved_path,
        None,
        actor,
    )?;

    let index = ctx
        .index
        .as_mut()
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
            "class_count": 12,
            "workflow_count": 1,
        }),
    );
    
    // Mirror the newly registered system definition
    let stored_object = ObjectStore::read_version(store, &version_ref)?;
    mirror_surface(store, &stored_object)?;

    Ok(())
}
