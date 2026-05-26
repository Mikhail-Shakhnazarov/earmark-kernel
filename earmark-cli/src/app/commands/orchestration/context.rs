use std::fs;

use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::emit;
use crate::cli::RecordContextArgs;
use serde_json::json;

use crate::app::commands::orchestration::common::*;

pub fn handle_record_context(
    ctx: &mut CommandContext,
    args: &RecordContextArgs,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    require_initialized_workspace(store)?;
    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required"))?;

    let (task_oid, _class, _task_summary, task_payload) =
        find_orchestration_task(index_ref, store, &args.task_id)?
            .ok_or_else(|| CliError::not_found(format!("task {} not found", args.task_id)))?;

    let task_id = task_payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if !args.path.exists() {
        return Err(CliError::argument(format!(
            "context file does not exist: {}",
            args.path.display()
        )));
    }

    let raw_content = fs::read_to_string(&args.path)
        .map_err(|e| CliError::argument(format!("failed to read context file: {}", e)))?;

    let payload = if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&raw_content) {
        json!({
            "task_id": task_id,
            "structured": json_val,
            "captured_by": "orchestration record-context"
        })
    } else {
        json!({
            "task_id": task_id,
            "raw_text": raw_content,
            "captured_by": "orchestration record-context"
        })
    };

    let mut headers = std::collections::BTreeMap::new();
    headers.insert(
        "task_id".to_string(),
        earmark_core::HeaderValue::String(task_id.clone()),
    );

    let title = format!("Context packet for {}", task_id);
    let obj_ref = deposit_orchestration_object(
        ctx.store,
        index_ref,
        ctx.provider_registry,
        "context_packet",
        Some(title),
        payload,
        headers,
    )?;

    create_orchestration_relation(
        ctx.store,
        index_ref,
        ctx.provider_registry,
        task_oid.clone(),
        obj_ref.id.clone(),
        "has_context",
    )?;

    emit(
        as_json,
        json!({
            "kind": "orchestration_context_packet",
            "task_id": task_id,
            "object_id": obj_ref.id.as_str(),
            "version_id": obj_ref.version_id.as_str()
        }),
    );

    Ok(())
}
