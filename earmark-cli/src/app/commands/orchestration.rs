use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::{emit, mirror_surface, register_declaration_file};
use crate::cli::*;
use earmark_core::{ObjectId, RuntimeProvenance, VersionId, VersionRef};
use earmark_declarations::activate_system_definition;
use earmark_index::{DerivedIndex, ObjectSummary, QueryFilter};
use earmark_runtime_tools::{DepositValidationContext, RuntimeToolSurface};
use earmark_store::ObjectStore;
use serde_json::json;

pub fn handle(ctx: &CommandContext, command: &OrchestrationCommand) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;
    let actor = ctx.actor;

    match &command.action {
        OrchestrationAction::InitExample => {
            require_initialized_workspace(store)?;

            let version_ref = register_declaration_file(
                store,
                None,
                DeclarationKind::System,
                &PathBuf::from("examples/earmark-dev-orchestration/declarations/system.yaml"),
                None,
                actor,
            )?;

            let index = ctx
                .index
                .as_ref()
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
                    "class_count": 8,
                    "workflow_count": 1,
                }),
            );
            Ok(())
        }
        OrchestrationAction::CaptureGit(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::IngestManifest(args) => {
            require_initialized_workspace(store)?;

            let manifest_path = &args.path;
            if !manifest_path.exists() {
                return Err(CliError::not_found(format!(
                    "manifest file not found: {}",
                    manifest_path.display()
                )));
            }
            let raw_text = fs::read_to_string(manifest_path)?;

            let sections = parse_manifest_sections(&raw_text);
            let preamble = sections.get("_preamble").map(|s| s.as_str()).unwrap_or("");
            let header_pairs = parse_header_pairs(preamble);

            let task_id = match &args.task_id {
                Some(tid) => tid.clone(),
                None => {
                    let from_header = header_pairs
                        .get("task_uuid")
                        .or_else(|| header_pairs.get("task_id"))
                        .cloned();
                    match from_header {
                        Some(tid) => tid,
                        None => manifest_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .and_then(parse_task_id_from_filename)
                            .unwrap_or_else(|| "unknown".to_string()),
                    }
                }
            };

            let attempt: usize = match args.attempt {
                Some(a) => a,
                None => {
                    let from_header = header_pairs
                        .get("attempt_number")
                        .and_then(|s| s.parse::<usize>().ok());
                    match from_header {
                        Some(a) => a,
                        None => manifest_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .and_then(parse_attempt_from_filename)
                            .unwrap_or(1),
                    }
                }
            };

            let objective = sections.get("objective").cloned().unwrap_or_default();
            let local_gates = sections
                .get("local gates")
                .or_else(|| sections.get("acceptance gates"))
                .map(|s| {
                    parse_fenced_code_blocks(s)
                        .into_iter()
                        .flat_map(|block| {
                            block
                                .lines()
                                .map(|l| l.trim().to_string())
                                .collect::<Vec<_>>()
                        })
                        .filter(|l| !l.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let target_files = sections
                .get("target files")
                .map(|s| parse_bullet_list(s))
                .unwrap_or_default();
            let executor = args
                .executor
                .clone()
                .unwrap_or_else(|| "opencode".to_string());
            let branch = args.branch.clone().unwrap_or_default();

            let payload = json!({
                "task_id": task_id,
                "attempt": attempt,
                "objective": objective,
                "local_gates": local_gates,
                "target_files": target_files,
                "raw_text": raw_text,
                "executor": executor,
                "branch": branch,
            });

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for ingest-manifest"))?;
            let runtime_surface = RuntimeToolSurface {
                store,
                index: index_ref,
                provider_service: ctx.provider_registry,
            };

            let prov = RuntimeProvenance {
                actor: "operator".to_string(),
                source_type: "cli".to_string(),
            };

            let object_ref = runtime_surface.deposit_object(
                "executor_manifest".to_string(),
                Some("object".to_string()),
                Some(format!("Manifest for {}", task_id)),
                payload,
                prov,
                DepositValidationContext::default(),
            )?;

            index_ref.rebuild_from_store(store)?;

            let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
            let stored_object = store.read_version(&vr)?;
            mirror_surface(store, &stored_object)?;

            emit(
                as_json,
                json!({
                    "kind": "executor_manifest_ingest",
                    "object_id": object_ref.id.as_str(),
                    "version_id": object_ref.version_id.as_str(),
                    "task_id": task_id,
                    "attempt": attempt,
                    "objective": objective,
                    "local_gates": local_gates,
                    "target_files": target_files,
                }),
            );

            Ok(())
        }
        OrchestrationAction::IngestReport(args) => {
            require_initialized_workspace(store)?;

            let report_path = &args.path;
            if !report_path.exists() {
                return Err(CliError::not_found(format!(
                    "report file not found: {}",
                    report_path.display()
                )));
            }
            let raw_text = fs::read_to_string(report_path)?;

            let sections = parse_manifest_sections(&raw_text);
            let preamble = sections.get("_preamble").map(|s| s.as_str()).unwrap_or("");
            let header_pairs = parse_header_pairs(preamble);

            let task_id = match &args.task_id {
                Some(tid) => tid.clone(),
                None => {
                    let from_header = header_pairs
                        .get("task_uuid")
                        .or_else(|| header_pairs.get("task_id"))
                        .cloned();
                    match from_header {
                        Some(tid) => tid,
                        None => report_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .and_then(parse_task_id_from_filename)
                            .unwrap_or_else(|| "unknown".to_string()),
                    }
                }
            };

            let attempt: usize = match args.attempt {
                Some(a) => a,
                None => {
                    let from_header = header_pairs
                        .get("attempt_number")
                        .or_else(|| header_pairs.get("attempt"))
                        .and_then(|s| s.parse::<usize>().ok());
                    match from_header {
                        Some(a) => a,
                        None => report_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .and_then(parse_attempt_from_filename)
                            .unwrap_or(1),
                    }
                }
            };

            let files_changed = parse_files_changed(&sections);

            let manifest_ref: Option<String> = if let Some(m) = &args.manifest {
                Some(m.clone())
            } else {
                let index_ref = ctx
                    .index
                    .as_ref()
                    .ok_or_else(|| CliError::argument("index required for ingest-report"))?;
                resolve_manifest_for_report(store, index_ref, &task_id, attempt)?
            };

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for ingest-report"))?;
            let runtime_surface = RuntimeToolSurface {
                store,
                index: index_ref,
                provider_service: ctx.provider_registry,
            };

            let prov = RuntimeProvenance {
                actor: "operator".to_string(),
                source_type: "cli".to_string(),
            };

            let payload = json!({
                "task_id": task_id,
                "attempt": attempt,
                "files_changed": files_changed,
                "raw_text": raw_text,
                "manifest": manifest_ref,
            });

            let object_ref = runtime_surface.deposit_object(
                "executor_report".to_string(),
                Some("object".to_string()),
                Some(format!("Report for {}", task_id)),
                payload,
                prov,
                DepositValidationContext::default(),
            )?;

            index_ref.rebuild_from_store(store)?;

            let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
            let stored_object = store.read_version(&vr)?;
            mirror_surface(store, &stored_object)?;

            if let Some(ref m) = manifest_ref {
                let manifest_oid_str = m.split(':').next().unwrap_or(m);
                if let Ok(target_id) = ObjectId::parse(manifest_oid_str.to_string()) {
                    let rel_prov = RuntimeProvenance {
                        actor: "operator".to_string(),
                        source_type: "cli".to_string(),
                    };
                    let _ = runtime_surface.create_relation(
                        object_ref.id.clone(),
                        target_id,
                        "implements_manifest".to_string(),
                        json!({}),
                        rel_prov,
                    );
                }
            }

            emit(
                as_json,
                json!({
                    "kind": "executor_report_ingest",
                    "object_id": object_ref.id.as_str(),
                    "version_id": object_ref.version_id.as_str(),
                    "task_id": task_id,
                    "attempt": attempt,
                    "files_changed": files_changed,
                    "manifest": manifest_ref,
                }),
            );

            Ok(())
        }
        OrchestrationAction::IngestTask(args) => {
            require_initialized_workspace(store)?;

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for ingest-task"))?;

            let source = &args.source;
            if source != "engram" {
                return Err(CliError::argument(format!(
                    "unsupported source: {}. only 'engram' is supported",
                    source
                )));
            }

            let output = Command::new("cargo")
                .current_dir("/home/m/GITHUB/engram/engram")
                .args([
                    "run",
                    "--manifest-path",
                    "/home/m/GITHUB/engram/engram/Cargo.toml",
                    "--quiet",
                    "--bin",
                    "engram",
                    "--",
                    "task",
                    "show",
                    &args.task_id,
                ])
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stderr_trimmed = stderr.trim();
                if stderr_trimmed.contains("Not found") {
                    return Err(CliError::not_found(format!(
                        "task {} not found",
                        args.task_id
                    )));
                }
                return Err(CliError::argument(format!(
                    "engram command failed: {}",
                    stderr_trimmed
                )));
            }

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let fields = parse_engram_fields(&stdout);

            let task_id = fields
                .get("ID")
                .cloned()
                .unwrap_or_else(|| args.task_id.clone());
            let title = fields.get("Title").cloned().unwrap_or_default();
            let description = fields.get("Description").cloned().unwrap_or_default();
            let priority = fields.get("Priority").cloned().unwrap_or_default();
            let status_raw = fields.get("Status").cloned().unwrap_or_default();
            let status = map_engram_status(&status_raw);

            let payload = json!({
                "task_id": task_id,
                "title": title,
                "description": description,
                "status": status,
                "priority": priority,
                "tags": [],
                "raw_text": stdout,
            });

            let runtime_surface = RuntimeToolSurface {
                store,
                index: index_ref,
                provider_service: ctx.provider_registry,
            };

            let prov = RuntimeProvenance {
                actor: "operator".to_string(),
                source_type: "cli".to_string(),
            };

            let object_ref = runtime_surface.deposit_object(
                "implementation_task".to_string(),
                Some("object".to_string()),
                Some(title.clone()),
                payload,
                prov,
                DepositValidationContext::default(),
            )?;

            index_ref.rebuild_from_store(store)?;

            let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
            let stored_object = store.read_version(&vr)?;
            mirror_surface(store, &stored_object)?;

            emit(
                as_json,
                json!({
                    "kind": "implementation_task_ingest",
                    "object_id": object_ref.id.as_str(),
                    "version_id": object_ref.version_id.as_str(),
                    "task_id": task_id,
                    "title": title,
                    "status": status,
                    "priority": priority,
                }),
            );

            Ok(())
        }
        OrchestrationAction::RecordGate(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::Review(_) => Err(CliError::argument("command not yet implemented")),
        OrchestrationAction::Show(args) => {
            require_initialized_workspace(store)?;

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for show"))?;

            let task_arg = args.task_id.to_lowercase();

            let task_filter = QueryFilter {
                class: Some("implementation_task".to_string()),
                ..Default::default()
            };
            let task_results = index_ref.query_objects(&task_filter)?;

            let mut found_task: Option<(ObjectSummary, serde_json::Value)> = None;
            for summary in &task_results {
                let oid = match ObjectId::parse(summary.object_id.clone()) {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                let vid = match VersionId::parse(summary.version_id.clone()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let vr = VersionRef::new(oid, vid);
                let stored = match store.read_version(&vr) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let text = match stored.payload.as_utf8() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let payload: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let tid = match payload.get("task_id").and_then(|v| v.as_str()) {
                    Some(t) => t.to_lowercase(),
                    None => continue,
                };
                let oid_lower = summary.object_id.to_lowercase();
                if oid_lower == task_arg
                    || oid_lower.starts_with(&task_arg)
                    || tid.starts_with(&task_arg)
                {
                    found_task = Some((summary.clone(), payload));
                    break;
                }
            }

            let (task_summary, task_payload) = found_task
                .ok_or_else(|| CliError::not_found(format!("task {} not found", args.task_id)))?;

            let task_id = task_payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = task_payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let description = task_payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let status = task_payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let priority = task_payload
                .get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let manifest_filter = QueryFilter {
                class: Some("executor_manifest".to_string()),
                ..Default::default()
            };
            let manifest_results = index_ref.query_objects(&manifest_filter)?;

            let mut manifests: Vec<(ObjectSummary, serde_json::Value)> = Vec::new();
            for summary in &manifest_results {
                let oid = match ObjectId::parse(summary.object_id.clone()) {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                let vid = match VersionId::parse(summary.version_id.clone()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let vr = VersionRef::new(oid, vid);
                let stored = match store.read_version(&vr) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let text = match stored.payload.as_utf8() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let payload: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let tid = match payload.get("task_id").and_then(|v| v.as_str()) {
                    Some(t) => t.to_lowercase(),
                    None => continue,
                };
                let oid_lower = summary.object_id.to_lowercase();
                if tid.starts_with(&task_arg) || oid_lower.starts_with(&task_arg) {
                    manifests.push((summary.clone(), payload));
                }
            }

            let report_filter = QueryFilter {
                class: Some("executor_report".to_string()),
                ..Default::default()
            };
            let report_results = index_ref.query_objects(&report_filter)?;

            let mut reports: Vec<(ObjectSummary, serde_json::Value)> = Vec::new();
            for summary in &report_results {
                let oid = match ObjectId::parse(summary.object_id.clone()) {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                let vid = match VersionId::parse(summary.version_id.clone()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let vr = VersionRef::new(oid, vid);
                let stored = match store.read_version(&vr) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let text = match stored.payload.as_utf8() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let payload: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let tid = match payload.get("task_id").and_then(|v| v.as_str()) {
                    Some(t) => t.to_lowercase(),
                    None => continue,
                };
                let oid_lower = summary.object_id.to_lowercase();
                if tid.starts_with(&task_arg) || oid_lower.starts_with(&task_arg) {
                    reports.push((summary.clone(), payload));
                }
            }

            let task_oid = ObjectId::parse(task_summary.object_id.clone())?;
            let head_stored = store.read_head(&task_oid)?;
            let standing = head_stored
                .as_ref()
                .map(|s| {
                    let mut map = serde_json::Map::new();
                    for (dim, token) in s.envelope.standing.iter() {
                        map.insert(
                            dim.as_str().to_string(),
                            serde_json::Value::String(token.as_str().to_string()),
                        );
                    }
                    serde_json::Value::Object(map)
                })
                .unwrap_or(serde_json::Value::Null);

            let mut attempts: Vec<serde_json::Value> = Vec::new();
            for (manifest_summary, manifest_payload) in &manifests {
                let attempt = manifest_payload
                    .get("attempt")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1);

                let report_entry = reports
                    .iter()
                    .find(|(_, rp)| rp.get("attempt").and_then(|v| v.as_u64()) == Some(attempt));

                let objective = manifest_payload
                    .get("objective")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let target_files = manifest_payload
                    .get("target_files")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let local_gates = manifest_payload
                    .get("local_gates")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .map(|v| v.as_str().unwrap_or("").to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let (report_version_id, files_changed) = match report_entry {
                    Some((rep_summary, rep_payload)) => {
                        let files = rep_payload
                            .get("files_changed")
                            .and_then(|v| v.as_array())
                            .map(|a| {
                                a.iter()
                                    .map(|v| v.as_str().unwrap_or("").to_string())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        (rep_summary.version_id.clone(), files)
                    }
                    None => (String::new(), Vec::new()),
                };

                attempts.push(json!({
                    "attempt": attempt,
                    "manifest_version_id": manifest_summary.version_id,
                    "objective": objective,
                    "target_files": target_files,
                    "local_gates": local_gates,
                    "report_version_id": report_version_id,
                    "files_changed": files_changed,
                }));
            }

            emit(
                as_json,
                json!({
                    "kind": "orchestration_task_details",
                    "task_id": task_id,
                    "title": title,
                    "description": description,
                    "status": status,
                    "priority": priority,
                    "standing": standing,
                    "attempts": attempts,
                }),
            );

            Ok(())
        }
        OrchestrationAction::List(_) => Err(CliError::argument("command not yet implemented")),
    }
}

fn parse_manifest_sections(text: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current_section = String::from("_preamble");
    let mut current_content = Vec::new();

    for line in text.lines() {
        if line.starts_with("## ") {
            if !current_content.is_empty() {
                sections.insert(current_section, current_content.join("\n"));
            }
            current_section = line[3..].trim().to_lowercase();
            current_content.clear();
        } else {
            current_content.push(line);
        }
    }
    if !current_content.is_empty() {
        sections.insert(current_section, current_content.join("\n"));
    }
    sections
}

fn parse_header_pairs(text: &str) -> HashMap<String, String> {
    let mut pairs = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            pairs.insert(key, value);
        }
    }
    pairs
}

fn parse_bullet_list(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.starts_with("- ") {
                Some(t[2..].trim().to_string())
            } else if t.starts_with("* ") {
                Some(t[2..].trim().to_string())
            } else {
                None
            }
        })
        .collect()
}

fn parse_fenced_code_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current = Vec::new();

    for line in text.lines() {
        if line.trim().starts_with("```") {
            if in_block {
                if !current.is_empty() {
                    blocks.push(current.join("\n"));
                }
                current.clear();
                in_block = false;
            } else {
                in_block = true;
            }
        } else if in_block {
            current.push(line);
        }
    }
    if in_block && !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

fn parse_task_id_from_filename(filename: &str) -> Option<String> {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    if let Some(pos) = stem.find("--") {
        return Some(stem[..pos].to_string());
    }
    None
}

fn parse_attempt_from_filename(filename: &str) -> Option<usize> {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);
    if let Some(pos) = stem.find("--") {
        let after = &stem[pos + 2..];
        if let Some(dash) = after.find('-') {
            return after[..dash].parse::<usize>().ok();
        }
        return after.parse::<usize>().ok();
    }
    None
}

fn parse_files_changed(sections: &HashMap<String, String>) -> Vec<String> {
    let mut files = Vec::new();
    for (key, content) in sections {
        let k = key.trim().to_lowercase();
        if k == "changed files" || k == "files changed" {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let path = if trimmed.starts_with("- ") {
                    &trimmed[2..]
                } else if trimmed.starts_with("* ") {
                    &trimmed[2..]
                } else if trimmed.len() > 2 && trimmed.as_bytes()[1] == b' ' {
                    let prefix = trimmed[..2].to_uppercase();
                    if prefix == "M "
                        || prefix == "A "
                        || prefix == "D "
                        || prefix == "R "
                        || prefix == "??"
                    {
                        &trimmed[2..]
                    } else {
                        trimmed
                    }
                } else {
                    trimmed
                };
                let p = path.trim().to_string();
                if !p.is_empty() {
                    files.push(p);
                }
            }
        }
    }
    files
}

fn parse_engram_fields(text: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in text.lines() {
        if !line.starts_with("  ") {
            continue;
        }
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            if !key.is_empty()
                && !value.is_empty()
                && key.chars().all(|c| c.is_alphanumeric() || c == ' ')
            {
                fields.insert(key, value);
            }
        }
    }
    fields
}

fn map_engram_status(status: &str) -> &'static str {
    match status {
        "Todo" => "proposed",
        "InProgress" => "dispatched",
        "Done" => "implemented",
        "Blocked" => "proposed",
        "Cancelled" => "closed",
        _ => "proposed",
    }
}

fn resolve_manifest_for_report(
    store: &impl ObjectStore,
    index: &DerivedIndex,
    task_id: &str,
    attempt: usize,
) -> Result<Option<String>, CliError> {
    let filter = QueryFilter {
        class: Some("executor_manifest".to_string()),
        ..Default::default()
    };
    let results = index.query_objects(&filter)?;
    for summary in results {
        let oid = ObjectId::parse(summary.object_id.clone())?;
        let vid = VersionId::parse(summary.version_id.clone())?;
        let vr = VersionRef::new(oid, vid);
        if let Ok(stored) = store.read_version(&vr) {
            if let Ok(text) = stored.payload.as_utf8() {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&text) {
                    let tid = payload.get("task_id").and_then(|v| v.as_str());
                    let att = payload.get("attempt").and_then(|v| v.as_u64());
                    if tid == Some(task_id) && att == Some(attempt as u64) {
                        return Ok(Some(summary.object_id));
                    }
                }
            }
        }
    }
    Ok(None)
}
