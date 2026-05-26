use std::collections::BTreeMap;
use std::path::Path;

use crate::app::common::CliError;
use earmark_core::{ObjectId, RuntimeProvenance, VersionId, VersionRef};
use earmark_index::{DerivedIndex, ObjectSummary, QueryFilter};
use earmark_runtime_tools::{DepositValidationContext, RuntimeToolSurface};
use earmark_store::{GitCanonicalStore, ObjectStore};
use serde_json::json;

/// Backward compatibility helper for orchestration classes.
#[allow(dead_code)]
pub fn canonical_orchestration_class(class: &str) -> &str {
    match class {
        "executor_manifest" => "dispatch",
        "executor_report" => "evidence",
        "implementation_task" => "work_item",
        _ => class,
    }
}

pub fn normalize_work_item_status(status: &str) -> String {
    let s = status.to_lowercase();
    match s.as_str() {
        "todo" | "pending" | "proposed" => "proposed".to_string(),
        "doing" | "in_progress" | "active" | "dispatched" | "running" | "started" => {
            "active".to_string()
        }
        "review" | "qa" | "under_review" => "under_review".to_string(),
        "followup" | "partial" | "needs_revision" | "followup_required" => {
            "followup_required".to_string()
        }
        "blocked" | "stuck" | "hold" => "blocked".to_string(),
        "done" | "completed" | "succeeded" | "success" | "closed" => "completed".to_string(),
        "accepted" | "approved" | "implemented" | "finalized" => "accepted".to_string(),
        "rejected" | "denied" => "rejected".to_string(),
        _ => s,
    }
}

pub fn is_terminal_work_item_status(status: &str) -> bool {
    matches!(
        normalize_work_item_status(status).as_str(),
        "completed" | "accepted" | "rejected"
    )
}

pub fn normalize_dispatch_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "queued" | "pending" | "waiting" => "queued".to_string(),
        "running" | "in_progress" | "executing" => "running".to_string(),
        "succeeded" | "success" | "done" | "passed" | "ok" => "succeeded".to_string(),
        "failed" | "fail" | "error" | "err" => "failed".to_string(),
        "cancelled" | "cancel" | "abort" | "stopped" => "cancelled".to_string(),
        other => other.to_string(),
    }
}

pub fn normalize_gate_status(status: &str) -> Result<String, CliError> {
    match status.to_lowercase().as_str() {
        "pass" | "passed" | "success" | "ok" => Ok("pass".to_string()),
        "fail" | "failed" | "error" => Ok("fail".to_string()),
        "skipped" | "skip" => Ok("skipped".to_string()),
        other => Err(CliError::argument(format!(
            "invalid gate status: {}",
            other
        ))),
    }
}

pub fn normalize_review_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "unreviewed" | "pending" | "none" | "draft" | "proposed" => "unreviewed".to_string(),
        "accepted" | "approve" | "approved" | "pass" | "ok" => "accepted".to_string(),
        "rejected" | "reject" | "deny" | "denied" | "fail" => "rejected".to_string(),
        other => other.to_string(),
    }
}

pub fn resolve_orchestration_namespace(
    index: &DerivedIndex,
    _store: &GitCanonicalStore,
) -> Option<String> {
    if let Ok(Some(active)) = index.get_active_system("examples.earmark-dev") {
        return Some(active.namespace);
    }
    None
}

pub fn find_orchestration_task(
    index: &DerivedIndex,
    store: &GitCanonicalStore,
    task_arg: &str,
) -> Result<Option<(ObjectId, String, ObjectSummary, serde_json::Value)>, CliError> {
    let task_arg = task_arg.to_lowercase();
    let mut candidates = Vec::new();

    for class in &[
        "work_item",
        "dispatch",
        "evidence",
        "git_snapshot",
        "gate_result",
        "review",
        "closure",
        "context_packet",
        "followup_task",
        "trace_event",
    ] {
        let filter = QueryFilter {
            class: Some(class.to_string()),
            ..Default::default()
        };
        let results = index.query_objects(&filter)?;
        for summary in results {
            let oid = match ObjectId::parse(summary.object_id.clone()) {
                Ok(o) => o,
                Err(_) => continue,
            };

            // Check OID prefix or full OID
            if summary.object_id.to_lowercase().starts_with(&task_arg) {
                candidates.push((oid, class.to_string(), summary));
                continue;
            }

            // Check VersionId prefix
            if summary.version_id.to_lowercase().starts_with(&task_arg) {
                candidates.push((oid, class.to_string(), summary));
                continue;
            }

            // Check payload task_id
            let vid = match VersionId::parse(summary.version_id.clone()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let vr = VersionRef::new(oid.clone(), vid);
            if let Ok(stored) = store.read_version(&vr) {
                if let Ok(text) = stored.payload.as_utf8() {
                    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(tid) = payload.get("task_id").and_then(|v| v.as_str()) {
                            if tid.to_lowercase() == task_arg {
                                candidates.push((oid, class.to_string(), summary));
                            }
                        }
                    }
                }
            }
        }
    }

    if candidates.is_empty() {
        return Ok(None);
    }

    // Prefer work_item if multiple candidates
    candidates.sort_by_key(|(_, class, _)| if class == "work_item" { 0 } else { 1 });

    let (oid, class, summary) = candidates.remove(0);
    let vid = VersionId::parse(summary.version_id.clone())?;
    let vr = VersionRef::new(oid.clone(), vid);
    let stored = store.read_version(&vr)?;
    let text = stored.payload.as_utf8()?;
    let payload = serde_json::from_str(&text).map_err(|e| CliError::argument(e.to_string()))?;

    Ok(Some((oid, class, summary, payload)))
}

pub fn deposit_orchestration_object(
    store: &GitCanonicalStore,
    index: &mut DerivedIndex,
    provider_registry: &dyn earmark_exec::ProviderService,
    class: &str,
    title: Option<String>,
    payload: serde_json::Value,
    headers: BTreeMap<String, earmark_core::HeaderValue>,
) -> Result<VersionRef, CliError> {
    let namespace = resolve_orchestration_namespace(index, store);
    let mut runtime_surface = RuntimeToolSurface {
        store,
        index,
        provider_service: provider_registry,
    };
    let prov = RuntimeProvenance {
        actor: "operator".to_string(),
        source_type: "cli".to_string(),
    };

    let object_ref = runtime_surface.deposit_object(
        class.to_string(),
        Some("object".to_string()),
        title,
        payload,
        prov,
        DepositValidationContext { namespace, headers },
    )?;

    let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
    Ok(vr)
}

pub fn update_orchestration_object_head(
    store: &GitCanonicalStore,
    index: &mut DerivedIndex,
    object_id: ObjectId,
    updated_payload: serde_json::Value,
    headers: BTreeMap<String, earmark_core::HeaderValue>,
    title: Option<String>,
) -> Result<VersionRef, CliError> {
    use earmark_core::{HeaderValue, Kind, Provenance};
    use earmark_store::{StoredObject, StoredPayload};

    let current_head = store.read_head(&object_id)?.ok_or_else(|| {
        CliError::not_found(format!("object head not found: {}", object_id.as_str()))
    })?;

    let mut merged_headers = current_head.envelope.headers.clone();
    for (key, value) in headers {
        merged_headers.insert(key, value);
    }

    if let Some(title_val) = title {
        merged_headers.insert("title".to_string(), HeaderValue::String(title_val));
    }

    let payload_bytes = serde_json::to_vec_pretty(&updated_payload)
        .map_err(|e| CliError::argument(format!("failed to serialize payload: {}", e)))?;

    let object = StoredObject::new_with_id(
        object_id,
        Kind::Object,
        current_head.envelope.class.clone(),
        current_head.envelope.standing.clone(),
        Provenance::direct_input("operator"),
        merged_headers,
        StoredPayload::from_json_bytes(payload_bytes),
        vec![current_head.envelope.version_ref()],
    );

    let vr = earmark_exec::persistence_helpers::write_object_and_index(store, index, &object)
        .map_err(|e| CliError::argument(format!("failed to write object update: {}", e)))?;

    Ok(vr)
}

pub fn create_orchestration_relation(
    store: &GitCanonicalStore,
    index: &mut DerivedIndex,
    provider_registry: &dyn earmark_exec::ProviderService,
    source: ObjectId,
    target: ObjectId,
    relation_type: &str,
) -> Result<(), CliError> {
    let mut runtime_surface = RuntimeToolSurface {
        store,
        index,
        provider_service: provider_registry,
    };
    let prov = RuntimeProvenance {
        actor: "operator".to_string(),
        source_type: "cli".to_string(),
    };
    runtime_surface.create_relation(source, target, relation_type.to_string(), json!({}), prov)?;
    Ok(())
}

pub fn run_git_cmd(repo: &Path, args: &[&str]) -> Result<String, CliError> {
    let output = std::process::Command::new("git")
        .current_dir(repo)
        .args(args)
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Err(CliError::argument(format!(
                    "git command failed at {}: git {}. Error: {}",
                    repo.display(),
                    args.join(" "),
                    err
                )))
            }
        }
        Err(e) => Err(CliError::argument(format!(
            "failed to execute git at {}: {}",
            repo.display(),
            e
        ))),
    }
}

pub fn resolve_git_repo(
    store_root: &Path,
    explicit_repo: Option<&std::path::PathBuf>,
) -> Result<std::path::PathBuf, CliError> {
    if let Some(repo) = explicit_repo {
        if repo.exists() {
            return Ok(repo.clone());
        } else {
            return Err(CliError::argument(format!(
                "explicit repository path does not exist: {}",
                repo.display()
            )));
        }
    }

    // Try workspace root
    if store_root.join(".git").exists() {
        return Ok(store_root.to_path_buf());
    }

    // Try to find if we are in a worktree
    let output = std::process::Command::new("git")
        .current_dir(store_root)
        .args(["rev-parse", "--show-toplevel"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            return Ok(std::path::PathBuf::from(
                String::from_utf8_lossy(&out.stdout).trim().to_string(),
            ));
        }
    }

    Err(CliError::argument(
        "no Git repository found. Please specify --repo <path>".to_string(),
    ))
}
