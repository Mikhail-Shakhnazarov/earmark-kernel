use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::emit;
use crate::cli::OrchReviewArgs;
use serde_json::json;

use super::common::*;

pub fn handle_review(ctx: &mut CommandContext, args: &OrchReviewArgs) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    require_initialized_workspace(store)?;
    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required"))?;

    let (task_oid, _class, _task_summary, task_payload) =
        find_orchestration_task(index_ref, store, &args.task_id)?.ok_or_else(|| {
            CliError::not_found(format!("task {} not found", args.task_id))
        })?;

    let task_id = task_payload
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let normalized_status = normalize_review_status(&args.decision);

    let payload = json!({
        "task_id": task_id,
        "decision": normalized_status,
        "comment": args.comment.clone().unwrap_or_default(),
        "captured_by": "orchestration review"
    });

    let mut headers = std::collections::BTreeMap::new();
    headers.insert(
        "task_id".to_string(),
        earmark_core::HeaderValue::String(task_id.clone()),
    );

    let title = format!("Review decision: {} for {}", normalized_status, task_id);
    let obj_ref = deposit_orchestration_object(ctx.store, index_ref, ctx.provider_registry, "review", Some(title), payload, headers)?;

    create_orchestration_relation(
        ctx.store,
        index_ref,
        ctx.provider_registry,
        task_oid.clone(),
        obj_ref.id.clone(),
        "has_review",
    )?;

    emit(
        as_json,
        json!({
            "kind": "orchestration_review_decision",
            "task_id": task_id,
            "task_object_id": task_oid.as_str(),
            "review_object_id": obj_ref.id.as_str(),
            "review_version_id": obj_ref.version_id.as_str(),
            "decision": normalized_status
        }),
    );

    Ok(())
}
