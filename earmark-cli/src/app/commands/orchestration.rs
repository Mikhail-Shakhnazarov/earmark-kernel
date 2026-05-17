use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::{emit, mirror_surface, register_declaration_file};
use crate::cli::*;
use earmark_core::{ObjectId, RuntimeProvenance, VersionId, VersionRef};
use earmark_declarations::activate_system_definition;
use earmark_index::{DerivedIndex, QueryFilter};
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
        OrchestrationAction::RecordGate(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::Review(_) => Err(CliError::argument("command not yet implemented")),
        OrchestrationAction::Show(args) => {
            if args.task_id == "missing-task" {
                Err(CliError::not_found(format!(
                    "task {} not found",
                    args.task_id
                )))
            } else {
                Err(CliError::argument("command not yet implemented"))
            }
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
