use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::emit;
use crate::cli::{ExplainDispatchArgs, ListOrchestrationArgs, ShowTaskArgs};
use earmark_core::{ObjectId, VersionId, VersionRef};
use earmark_index::QueryFilter;
use earmark_store::ObjectStore;
use serde_json::json;

use crate::app::commands::orchestration::common::*;

pub fn handle_show(ctx: &mut CommandContext, args: &ShowTaskArgs) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    require_initialized_workspace(store)?;
    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required"))?;

    let (task_oid, class, _task_summary, task_payload) =
        find_orchestration_task(index_ref, store, &args.task_id)?
            .ok_or_else(|| CliError::not_found(format!("object {} not found", args.task_id)))?;

    let task_id = task_payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let nodes = super::graph::traverse_orchestration_graph(index_ref, store, &task_oid)?;

    let mut context_packets = Vec::new();
    let mut dispatches = Vec::new();
    let mut git_snapshots = Vec::new();
    let mut gate_results = Vec::new();
    let mut evidence_records = Vec::new();
    let mut review_records = Vec::new();
    let mut closures = Vec::new();
    let mut trace_events = Vec::new();

    for node in nodes {
        if node.object_id == task_oid {
            continue;
        }

        match node.class.as_str() {
            "context_packet" => context_packets.push(node.payload),
            "dispatch" => dispatches.push(node.payload),
            "git_snapshot" => git_snapshots.push(node.payload),
            "gate_result" => gate_results.push(node.payload),
            "evidence" => evidence_records.push(node.payload),
            "review" => review_records.push(node.payload),
            "closure" => closures.push(node.payload),
            "trace_event" => trace_events.push(node.payload),
            _ => {}
        }
    }

    emit(
        as_json,
        json!({
            "kind": format!("orchestration_{}_show", class),
            "work_item_id": task_oid.as_str(),
            "task_id": task_id,
            "payload": task_payload,
            "context_packets": context_packets,
            "dispatches": dispatches,
            "git_snapshots": git_snapshots,
            "gate_results": gate_results,
            "evidence_records": evidence_records,
            "review_records": review_records,
            "closures": closures,
            "trace_events": trace_events,
        }),
    );

    Ok(())
}

pub fn handle_list(ctx: &mut CommandContext, args: &ListOrchestrationArgs) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    require_initialized_workspace(store)?;
    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required"))?;

    let filter = QueryFilter {
        class: Some("work_item".to_string()),
        ..Default::default()
    };
    let results = index_ref.query_objects(&filter)?;

    let mut tasks = Vec::new();
    for summary in results {
        let oid = ObjectId::parse(summary.object_id.clone())?;
        let vid = VersionId::parse(summary.version_id.clone())?;
        let vr = VersionRef::new(oid, vid);
        if let Ok(stored) = ObjectStore::read_version(store, &vr) {
            if let Ok(text) = stored.payload.as_utf8().map(|s| s.to_string()) {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                    let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    if !args.include_closed && (status == "closed" || status == "completed") {
                        continue;
                    }
                    if let Some(ref target_status) = args.status {
                        if status != target_status {
                            continue;
                        }
                    }
                    let mut payload_obj = payload;
                    if let Some(obj) = payload_obj.as_object_mut() {
                        obj.insert("object_id".to_string(), json!(summary.object_id));
                    }
                    tasks.push(payload_obj);
                }
            }
        }
    }

    emit(
        as_json,
        json!({
            "kind": "orchestration_list",
            "tasks": tasks,
        }),
    );

    Ok(())
}

pub fn handle_timeline(ctx: &mut CommandContext, args: &ShowTaskArgs) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    require_initialized_workspace(store)?;
    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required"))?;

    let (task_oid, _class, _task_summary, _task_payload) =
        find_orchestration_task(index_ref, store, &args.task_id)?
            .ok_or_else(|| CliError::not_found(format!("task {} not found", args.task_id)))?;

    let nodes = super::graph::traverse_orchestration_graph(index_ref, store, &task_oid)?;

    let mut events = Vec::new();
    for node in nodes {
        events.push(json!({
            "class": node.class,
            "object_id": node.object_id.as_str(),
            "payload": node.payload,
            "timestamp": node.timestamp
        }));
    }

    // Sort by timestamp
    events.sort_by_key(|e| e.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0));

    emit(
        as_json,
        json!({
            "kind": "orchestration_timeline",
            "work_item_id": task_oid.as_str(),
            "task_id": args.task_id,
            "events": events,
        }),
    );

    Ok(())
}

pub fn handle_explain_dispatch(
    ctx: &mut CommandContext,
    args: &ExplainDispatchArgs,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    require_initialized_workspace(store)?;
    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required"))?;

    let dispatch_id = if args.dispatch_id == "latest" {
        let filter = QueryFilter {
            class: Some("dispatch".to_string()),
            ..Default::default()
        };
        let results = index_ref.query_objects(&filter)?;
        results
            .first()
            .map(|s| s.object_id.clone())
            .ok_or_else(|| CliError::not_found("no dispatch records found".to_string()))?
    } else {
        args.dispatch_id.clone()
    };

    let (oid, class, _summary, payload) = find_orchestration_task(index_ref, store, &dispatch_id)?
        .ok_or_else(|| CliError::not_found(format!("dispatch {} not found", dispatch_id)))?;

    if class != "dispatch" {
        return Err(CliError::argument(format!(
            "object {} is a {}, not a dispatch",
            dispatch_id, class
        )));
    }

    emit(
        as_json,
        json!({
            "kind": "orchestration_dispatch_explanation",
            "object_id": oid.as_str(),
            "payload": payload,
        }),
    );

    Ok(())
}
