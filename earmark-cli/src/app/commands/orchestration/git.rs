use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::{emit, mirror_surface};
use crate::cli::CaptureGitArgs;
use earmark_core::VersionRef;
use earmark_store::{ObjectStore, WorkspaceLayout};
use serde_json::json;

use super::common::*;

pub fn handle_capture_git(ctx: &mut CommandContext, args: &CaptureGitArgs) -> Result<(), CliError> {
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

    let repo_path = resolve_git_repo(ctx.store.root(), args.repo.as_ref())?;
    let git_toplevel = run_git_cmd(&repo_path, &["rev-parse", "--show-toplevel"]).unwrap_or_else(|_| repo_path.display().to_string());

    let commit = match &args.commit {
        Some(c) => c.clone(),
        None => run_git_cmd(&repo_path, &["rev-parse", "HEAD"])?,
    };

    let branch = run_git_cmd(&repo_path, &["branch", "--show-current"]).unwrap_or_default();

    let status_porcelain = run_git_cmd(&repo_path, &["status", "--porcelain"]).unwrap_or_default();
    let dirty = !status_porcelain.is_empty();

    let status_short = run_git_cmd(&repo_path, &["status", "--short"]).unwrap_or_default();

    let base = args.base.clone().unwrap_or_default();
    let head = args.head.clone().unwrap_or_else(|| commit.clone());

    let diff_stat = if args.include_diff_stat {
        if !base.is_empty() {
            run_git_cmd(&repo_path, &["diff", "--stat", &format!("{}..{}", base, head)])
                .unwrap_or_default()
        } else {
            run_git_cmd(&repo_path, &["diff", "--stat"]).unwrap_or_default()
        }
    } else {
        String::new()
    };

    let mut payload = json!({
        "task_id": task_id,
        "task_object_id": task_oid.as_str(),
        "phase": args.phase,
        "commit": commit,
        "base": base,
        "head": head,
        "branch": branch,
        "dirty": dirty,
        "status_short": status_short,
        "diff_stat": diff_stat,
        "repo_path": repo_path.display().to_string(),
        "git_toplevel": git_toplevel,
        "captured_by": "orchestration capture-git"
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
        "phase".to_string(),
        earmark_core::HeaderValue::String(args.phase.clone()),
    );
    headers.insert(
        "commit".to_string(),
        earmark_core::HeaderValue::String(args.commit.clone().unwrap_or_default()),
    );
    headers.insert(
        "branch".to_string(),
        earmark_core::HeaderValue::String(args.head.clone().unwrap_or_else(|| "unknown".to_string())),
    );

    let title = format!("Git snapshot: {} for {}", args.phase, task_id);
    let obj_ref = deposit_orchestration_object(ctx.store, index_ref, ctx.provider_registry, "git_snapshot", Some(title), payload, headers)?;

    if let Some(d_oid) = dispatch_oid {
        create_orchestration_relation(ctx.store, index_ref, ctx.provider_registry, d_oid, obj_ref.id.clone(), "anchored_by")?;
    } else {
        create_orchestration_relation(
            ctx.store,
            index_ref,
            ctx.provider_registry,
            task_oid.clone(),
            obj_ref.id.clone(),
            "has_git_snapshot",
        )?;
    }

    let vr = VersionRef::new(obj_ref.id.clone(), obj_ref.version_id.clone());
    if let Ok(stored_object) = ObjectStore::read_version(store, &vr) {
        let _ = mirror_surface(store, &stored_object);
    }

    emit(
        as_json,
        json!({
            "kind": "orchestration_git_snapshot",
            "task_id": task_id,
            "task_object_id": task_oid.as_str(),
            "snapshot_object_id": obj_ref.id.as_str(),
            "snapshot_version_id": obj_ref.version_id.as_str(),
            "phase": args.phase,
            "commit": commit,
            "dirty": dirty,
            "repo_path": repo_path.display().to_string(),
        }),
    );

    Ok(())
}
