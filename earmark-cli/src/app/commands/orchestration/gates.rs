use std::fs;

use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::emit;
use crate::cli::RecordGateArgs;
use serde_json::json;

use crate::app::commands::orchestration::common::*;

pub fn handle_record_gate(ctx: &mut CommandContext, args: &RecordGateArgs) -> Result<(), CliError> {
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

    let dispatch_oid = if let Some(ref d_id) = args.dispatch_id {
        let (oid, class, _, _) = find_orchestration_task(index_ref, store, d_id)?
            .ok_or_else(|| CliError::not_found(format!("dispatch {} not found", d_id)))?;
        if class != "dispatch" {
            return Err(CliError::argument(format!(
                "object {} is a {}, not a dispatch",
                d_id, class
            )));
        }
        Some(oid)
    } else {
        None
    };

    let log_content = if let Some(ref log_path) = args.log {
        fs::read_to_string(log_path).unwrap_or_default()
    } else {
        String::new()
    };

    let normalized_status = normalize_gate_status(&args.status)?;

    let mut payload = json!({
        "task_id": task_id,
        "command": args.command,
        "status": normalized_status,
        "log_excerpt": log_content,
        "captured_by": "orchestration record-gate"
    });

    if let Some(ref d_oid) = dispatch_oid {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("dispatch_id".to_string(), json!(d_oid.as_str()));
        }
    }

    let mut headers = std::collections::BTreeMap::new();
    headers.insert(
        "task_id".to_string(),
        earmark_core::HeaderValue::String(task_id.clone()),
    );
    headers.insert(
        "command".to_string(),
        earmark_core::HeaderValue::String(args.command.clone()),
    );
    headers.insert(
        "status".to_string(),
        earmark_core::HeaderValue::String(normalized_status.clone()),
    );

    let title = format!("Gate result: {} for {}", args.command, task_id);
    let obj_ref = deposit_orchestration_object(
        ctx.store,
        index_ref,
        ctx.provider_registry,
        "gate_result",
        Some(title),
        payload,
        headers,
    )?;

    if let Some(d_oid) = dispatch_oid {
        create_orchestration_relation(
            ctx.store,
            index_ref,
            ctx.provider_registry,
            d_oid,
            obj_ref.id.clone(),
            "gated_by",
        )?;
    } else {
        create_orchestration_relation(
            ctx.store,
            index_ref,
            ctx.provider_registry,
            task_oid.clone(),
            obj_ref.id.clone(),
            "has_gate_result",
        )?;
    }

    emit(
        as_json,
        json!({
            "kind": "orchestration_gate_result",
            "task_id": task_id,
            "task_object_id": task_oid.as_str(),
            "gate_object_id": obj_ref.id.as_str(),
            "gate_version_id": obj_ref.version_id.as_str(),
            "status": normalized_status,
            "command": args.command
        }),
    );

    Ok(())
}
