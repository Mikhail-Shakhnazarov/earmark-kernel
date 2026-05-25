use std::collections::BTreeMap;
use std::fs;

use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::{emit, mirror_surface};
use crate::cli::{IngestManifestArgs, IngestReportArgs, IngestTaskArgs};
use earmark_core::{ObjectId, RuntimeProvenance, VersionRef};
use earmark_runtime_tools::{DepositValidationContext, RuntimeToolSurface};
use earmark_store::{GitCanonicalStore, ObjectStore};
use serde_json::json;

use super::adapters;
use crate::app::commands::orchestration::common::*;

pub fn handle_ingest_manifest(
    ctx: &mut CommandContext,
    args: &IngestManifestArgs,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

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
            let fenced = parse_fenced_code_blocks(s)
                .into_iter()
                .flat_map(|block| {
                    block
                        .lines()
                        .map(|l| l.trim().to_string())
                        .collect::<Vec<_>>()
                })
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>();

            if !fenced.is_empty() {
                fenced
            } else {
                parse_bullet_list(s)
                    .into_iter()
                    .filter(|l| !l.trim().is_empty())
                    .collect::<Vec<_>>()
            }
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

    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required for ingest-manifest"))?;

    let context_oid = if let Some(ref c_id) = args.context_id {
        let (oid, class, _, _) = find_orchestration_task(index_ref, store, c_id)?
            .ok_or_else(|| CliError::not_found(format!("context {} not found", c_id)))?;
        if class != "context_packet" {
            return Err(CliError::argument(format!(
                "object {} is a {}, not a context_packet",
                c_id, class
            )));
        }
        Some(oid)
    } else {
        None
    };

    let mut payload = json!({
        "task_id": task_id,
        "attempt": attempt,
        "objective": objective,
        "local_gates": local_gates,
        "target_files": target_files,
        "raw_text": raw_text,
        "executor": executor,
        "branch": branch,
        "status": normalize_dispatch_status("queued"),
    });

    if let Some(ref c_oid) = context_oid {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("context_id".to_string(), json!(c_oid.as_str()));
        }
    }

    let (task_oid, _, _, _) = find_orchestration_task(index_ref, store, &task_id)?
        .ok_or_else(|| CliError::not_found(format!("parent work_item {} not found", task_id)))?;

    let prov = RuntimeProvenance {
        actor: "operator".to_string(),
        source_type: "cli".to_string(),
    };

    let mut headers = BTreeMap::new();
    headers.insert(
        "task_id".to_string(),
        earmark_core::HeaderValue::String(task_id.clone()),
    );

    let namespace = resolve_orchestration_namespace(index_ref, store);
    let mut runtime_surface = RuntimeToolSurface {
        store,
        index: index_ref,
        provider_service: ctx.provider_registry,
    };

    let object_ref = runtime_surface.deposit_object(
        "dispatch".to_string(),
        Some("object".to_string()),
        Some(format!("Dispatch for {}", task_id)),
        payload,
        prov,
        DepositValidationContext { namespace, headers },
    )?;

    create_orchestration_relation(
        ctx.store,
        index_ref,
        ctx.provider_registry,
        task_oid,
        object_ref.id.clone(),
        "dispatched_as",
    )?;

    if let Some(c_oid) = context_oid {
        create_orchestration_relation(
            ctx.store,
            index_ref,
            ctx.provider_registry,
            object_ref.id.clone(),
            c_oid,
            "used_context",
        )?;
    }

    let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
    let stored_object = ObjectStore::read_version(store, &vr)?;
    mirror_surface(store, &stored_object)?;

    emit(
        as_json,
        json!({
            "kind": "orchestration_dispatch_ingest",
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

pub fn handle_ingest_report(
    ctx: &mut CommandContext,
    args: &IngestReportArgs,
) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

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

    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required for ingest-report"))?;

    let manifest_ref: Option<String> = if let Some(m) = &args.manifest {
        Some(m.clone())
    } else {
        resolve_manifest_for_report(store, index_ref, &task_id, attempt)?
    };

    let namespace = resolve_orchestration_namespace(index_ref, store);
    let mut runtime_surface = RuntimeToolSurface {
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

    let mut headers = BTreeMap::new();
    headers.insert(
        "task_id".to_string(),
        earmark_core::HeaderValue::String(task_id.clone()),
    );

    let dispatch_oid = if let Some(ref m) = manifest_ref {
        let manifest_oid_str = m.split(':').next().unwrap_or(m);
        ObjectId::parse(manifest_oid_str.to_string()).ok()
    } else {
        None
    };

    let object_ref = runtime_surface.deposit_object(
        "evidence".to_string(),
        Some("object".to_string()),
        Some(format!("Evidence for {}", task_id)),
        payload,
        prov,
        DepositValidationContext { namespace, headers },
    )?;

    if let Some(d_oid) = dispatch_oid {
        create_orchestration_relation(
            ctx.store,
            runtime_surface.index,
            ctx.provider_registry,
            d_oid,
            object_ref.id.clone(),
            "produced_evidence",
        )?;
    }

    let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
    let stored_object = ObjectStore::read_version(store, &vr)?;
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
            "kind": "orchestration_evidence_ingest",
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

pub fn handle_ingest_task(ctx: &mut CommandContext, args: &IngestTaskArgs) -> Result<(), CliError> {
    let store = ctx.store;
    let as_json = ctx.as_json;

    require_initialized_workspace(store)?;

    let index_ref = ctx
        .index
        .as_mut()
        .ok_or_else(|| CliError::argument("index required for ingest-task"))?;

    let source = &args.source;

    let tasks = if let Some(title) = &args.title {
        vec![adapters::native_json::NativeTaskData {
            task_id: args.task_id.clone(),
            title: title.clone(),
            description: args.description.clone().unwrap_or_default(),
            priority: args
                .priority
                .clone()
                .unwrap_or_else(|| "medium".to_string()),
            status: args
                .status
                .clone()
                .unwrap_or_else(|| "proposed".to_string()),
            raw_text: String::new(),
        }]
    } else {
        match source.as_str() {
            "native-json" | "local-json" => adapters::native_json::ingest_from_json(&args.task_id)?,
            _ => {
                return Err(CliError::argument(format!(
                    "unsupported source: '{}'. Supported sources: 'native-json', 'local-json'",
                    source
                )));
            }
        }
    };

    if tasks.is_empty() {
        return Err(CliError::argument("No valid work items found in payload"));
    }

    let mut ingested = Vec::new();
    for task in tasks {
        let normalized_status = normalize_work_item_status(&task.status);
        let payload = json!({
            "task_id": task.task_id,
            "title": task.title,
            "description": task.description,
            "status": normalized_status,
            "priority": task.priority,
            "tags": [],
            "raw_text": task.raw_text,
        });

        let namespace = resolve_orchestration_namespace(index_ref, store);
        let mut runtime_surface = RuntimeToolSurface {
            store,
            index: index_ref,
            provider_service: ctx.provider_registry,
        };

        let prov = RuntimeProvenance {
            actor: "operator".to_string(),
            source_type: "cli".to_string(),
        };

        let mut headers = BTreeMap::new();
        headers.insert(
            "task_id".to_string(),
            earmark_core::HeaderValue::String(task.task_id.clone()),
        );

        let object_ref = runtime_surface.deposit_object(
            "work_item".to_string(),
            Some("object".to_string()),
            Some(task.title.clone()),
            payload,
            prov,
            DepositValidationContext { namespace, headers },
        )?;

        let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
        let stored_object = ObjectStore::read_version(store, &vr)?;
        mirror_surface(store, &stored_object)?;

        ingested.push(json!({
            "object_id": object_ref.id.as_str(),
            "version_id": object_ref.version_id.as_str(),
            "task_id": task.task_id,
            "title": task.title,
            "status": task.status,
            "priority": task.priority,
        }));
    }

    emit(
        as_json,
        json!({
            "kind": "orchestration_work_item_ingest",
            "source": source,
            "tasks": ingested,
        }),
    );

    Ok(())
}

fn resolve_manifest_for_report(
    store: &GitCanonicalStore,
    index: &earmark_index::DerivedIndex,
    task_id: &str,
    attempt: usize,
) -> Result<Option<String>, CliError> {
    use earmark_core::VersionId;
    use earmark_index::QueryFilter;

    let filter = QueryFilter {
        class: Some("dispatch".to_string()),
        ..Default::default()
    };
    let results = index.query_objects(&filter)?;
    for summary in results {
        let oid = ObjectId::parse(summary.object_id.clone())?;
        let vid = VersionId::parse(summary.version_id.clone())?;
        let vr = VersionRef::new(oid, vid);
        if let Ok(stored) = ObjectStore::read_version(store, &vr) {
            if let Ok(text) = stored.payload.as_utf8().map(|s| s.to_string()) {
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

// Private helpers (to be used locally in ingest.rs or moved to common if needed)
fn parse_manifest_sections(text: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current_section = "_preamble".to_string();
    let mut current_content = Vec::new();

    for line in text.lines() {
        if line.starts_with("# ") || line.starts_with("## ") {
            if !current_content.is_empty() {
                sections.insert(
                    current_section.clone(),
                    current_content.join("\n").trim().to_string(),
                );
                current_content.clear();
            }
            current_section = line.trim_start_matches('#').trim().to_lowercase();
        } else {
            current_content.push(line);
        }
    }
    if !current_content.is_empty() {
        sections.insert(
            current_section,
            current_content.join("\n").trim().to_string(),
        );
    }
    sections
}

fn parse_header_pairs(text: &str) -> BTreeMap<String, String> {
    let mut pairs = BTreeMap::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut in_yaml = false;
    for line in lines {
        if line.trim() == "---" {
            in_yaml = !in_yaml;
            continue;
        }
        if in_yaml {
            if let Some((k, v)) = line.split_once(':') {
                pairs.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }
    }
    pairs
}

fn parse_task_id_from_filename(filename: &str) -> Option<String> {
    filename.split('-').next().map(|s| s.to_string())
}

fn parse_attempt_from_filename(filename: &str) -> Option<usize> {
    filename.split('-').nth(1).and_then(|s| s.parse().ok())
}

fn parse_fenced_code_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        if line.starts_with("```") {
            if in_block {
                blocks.push(current_block.join("\n"));
                current_block.clear();
                in_block = false;
            } else {
                in_block = true;
            }
        } else if in_block {
            current_block.push(line);
        }
    }
    blocks
}

fn parse_bullet_list(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with("- ") || l.starts_with("* "))
        .map(|l| l[2..].trim().to_string())
        .collect()
}

fn parse_files_changed(sections: &BTreeMap<String, String>) -> Vec<String> {
    if let Some(content) = sections
        .get("files changed")
        .or_else(|| sections.get("affected files"))
    {
        parse_bullet_list(content)
    } else {
        Vec::new()
    }
}
