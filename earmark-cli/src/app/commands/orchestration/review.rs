use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::emit;
use crate::cli::OrchReviewArgs;
use serde_json::json;

use crate::app::commands::orchestration::common::*;

pub fn handle_review(ctx: &mut CommandContext, args: &OrchReviewArgs) -> Result<(), CliError> {
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

    let normalized_status = normalize_review_status(&args.decision);

    let (payload_status, process_standing, review_standing) = next_task_status(&normalized_status);

    let review_payload = json!({
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
    let review_ref = deposit_orchestration_object(
        ctx.store,
        index_ref,
        ctx.provider_registry,
        "review",
        Some(title),
        review_payload,
        headers,
    )?;

    create_orchestration_relation(
        ctx.store,
        index_ref,
        ctx.provider_registry,
        task_oid.clone(),
        review_ref.id.clone(),
        "has_review",
    )?;

    // Update the task status
    let mut updated_task_payload = task_payload.clone();
    if let Some(obj) = updated_task_payload.as_object_mut() {
        obj.insert("status".to_string(), json!(payload_status));

        // Update standings if present
        if let Some(process_s) = process_standing {
            obj.insert("kernel:process".to_string(), json!(process_s));
        }
        if let Some(review_s) = review_standing {
            obj.insert("kernel:review".to_string(), json!(review_s));
        }
    }

    let mut task_headers = std::collections::BTreeMap::new();
    task_headers.insert(
        "task_id".to_string(),
        earmark_core::HeaderValue::String(task_id.clone()),
    );
    task_headers.insert(
        "status".to_string(),
        earmark_core::HeaderValue::String(payload_status.to_string()),
    );

    let task_title = task_payload
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let updated_task_ref = update_orchestration_object_head(
        ctx.store,
        index_ref,
        task_oid.clone(),
        updated_task_payload,
        task_headers,
        task_title,
    )?;

    // Handle terminal decisions with a closure
    if normalized_status == "accepted" || normalized_status == "rejected" {
        let closure_payload = json!({
            "task_id": task_id,
            "decision": normalized_status,
            "disposition": normalized_status,
            "captured_by": "orchestration review closure"
        });
        let mut closure_headers = std::collections::BTreeMap::new();
        closure_headers.insert(
            "task_id".to_string(),
            earmark_core::HeaderValue::String(task_id.clone()),
        );

        let closure_title = format!("Closure: {} for {}", normalized_status, task_id);
        let closure_ref = deposit_orchestration_object(
            ctx.store,
            index_ref,
            ctx.provider_registry,
            "closure",
            Some(closure_title),
            closure_payload,
            closure_headers,
        )?;

        // review -> closure (causes)
        create_orchestration_relation(
            ctx.store,
            index_ref,
            ctx.provider_registry,
            review_ref.id.clone(),
            closure_ref.id.clone(),
            "causes",
        )?;

        // closure -> task (closes)
        create_orchestration_relation(
            ctx.store,
            index_ref,
            ctx.provider_registry,
            closure_ref.id.clone(),
            task_oid.clone(),
            "closes",
        )?;
    }

    emit(
        as_json,
        json!({
            "kind": "orchestration_review_decision",
            "task_id": task_id,
            "task_object_id": task_oid.as_str(),
            "task_version_id": updated_task_ref.version_id.as_str(),
            "review_object_id": review_ref.id.as_str(),
            "review_version_id": review_ref.version_id.as_str(),
            "decision": normalized_status,
            "next_status": payload_status
        }),
    );

    Ok(())
}

fn next_task_status(decision: &str) -> (&'static str, Option<&'static str>, Option<&'static str>) {
    match decision {
        "accepted" => ("accepted", Some("closed"), Some("accepted")),
        "rejected" => ("rejected", Some("closed"), Some("rejected")),
        "needs_revision" => ("followup_required", Some("active"), Some("needs_revision")),
        _ => ("proposed", None, None),
    }
}
