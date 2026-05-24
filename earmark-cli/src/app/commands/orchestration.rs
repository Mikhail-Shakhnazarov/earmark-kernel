pub mod adapters;

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use crate::app::common::{require_initialized_workspace, CliError, CommandContext};
use crate::app::{emit, mirror_surface, register_declaration_file};
use crate::cli::*;
use earmark_core::{
    DimensionId, ObjectId, RuntimeProvenance, Standing, TokenId, VersionId, VersionRef,
};
use earmark_declarations::activate_system_definition;
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_index::{DerivedIndex, ObjectSummary, QueryFilter};
use earmark_runtime_tools::{DepositValidationContext, RuntimeToolSurface};
use earmark_store::{ObjectStore, StoredObject, StoredPayload};
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
                    "class_count": 15,
                    "workflow_count": 1,
                }),
            );
            Ok(())
        }
        OrchestrationAction::CaptureGit(args) => {
            require_initialized_workspace(store)?;
            let index_ref = ctx
                .index
                .as_ref()
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
                let (oid, class, _, _) = find_orchestration_task(index_ref, store, d_id)?.ok_or_else(|| {
                    CliError::not_found(format!("dispatch {} not found", d_id))
                })?;
                if class != "dispatch" {
                    return Err(CliError::argument(format!("object {} is a {}, not a dispatch", d_id, class)));
                }
                Some(oid)
            } else {
                None
            };

            let commit = match &args.commit {
                Some(c) => c.clone(),
                None => run_git_cmd(&["rev-parse", "HEAD"])?,
            };

            let branch = run_git_cmd(&["branch", "--show-current"]).unwrap_or_default();

            let status_porcelain = run_git_cmd(&["status", "--porcelain"]).unwrap_or_default();
            let dirty = !status_porcelain.is_empty();

            let status_short = run_git_cmd(&["status", "--short"]).unwrap_or_default();

            let base = args.base.clone().unwrap_or_default();
            let head = args.head.clone().unwrap_or_else(|| commit.clone());

            let diff_stat = if args.include_diff_stat {
                if !base.is_empty() {
                    run_git_cmd(&["diff", "--stat", &format!("{}..{}", base, head)])
                        .unwrap_or_default()
                } else {
                    run_git_cmd(&["diff", "--stat"]).unwrap_or_default()
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
                "captured_by": "orchestration capture-git"
            });

            if let Some(ref d_oid) = dispatch_oid {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("dispatch_id".to_string(), json!(d_oid.as_str()));
                }
            }

            let title = format!("Git snapshot: {} for {}", args.phase, task_id);
            let obj_ref = deposit_orchestration_object(ctx, "git_snapshot", Some(title), payload)?;

            if let Some(d_oid) = dispatch_oid {
                create_orchestration_relation(
                    ctx,
                    d_oid,
                    obj_ref.id.clone(),
                    "anchored_by",
                )?;
            } else {
                create_orchestration_relation(
                    ctx,
                    task_oid.clone(),
                    obj_ref.id.clone(),
                    "has_git_snapshot",
                )?;
            }

            let vr = VersionRef::new(obj_ref.id.clone(), obj_ref.version_id.clone());
            if let Ok(stored_object) = store.read_version(&vr) {
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
                    "dirty": dirty
                }),
            );

            Ok(())
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

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for ingest-manifest"))?;

            let context_oid = if let Some(ref c_id) = args.context_id {
                let (oid, class, _, _) = find_orchestration_task(index_ref, store, c_id)?.ok_or_else(|| {
                    CliError::not_found(format!("context {} not found", c_id))
                })?;
                if class != "context_packet" {
                    return Err(CliError::argument(format!("object {} is a {}, not a context_packet", c_id, class)));
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

            let runtime_surface = RuntimeToolSurface {
                store,
                index: index_ref,
                provider_service: ctx.provider_registry,
            };

            let prov = RuntimeProvenance {
                actor: "operator".to_string(),
                source_type: "cli".to_string(),
            };

            let mut headers = BTreeMap::new();
            headers.insert("task_id".to_string(), earmark_core::HeaderValue::String(task_id.clone()));

            let (task_oid, _, _, _) = find_orchestration_task(index_ref, store, &task_id)?.ok_or_else(|| {
                CliError::not_found(format!("parent work_item {} not found", task_id))
            })?;

            let object_ref = runtime_surface.deposit_object(
                "dispatch".to_string(),
                Some("object".to_string()),
                Some(format!("Dispatch for {}", task_id)),
                payload,
                prov,
                DepositValidationContext {
                    namespace: Some("examples.earmark-dev".to_string()),
                    headers,
                },
            )?;

            create_orchestration_relation(
                ctx,
                task_oid,
                object_ref.id.clone(),
                "dispatched_as",
            )?;

            if let Some(c_oid) = context_oid {
                create_orchestration_relation(
                    ctx,
                    object_ref.id.clone(),
                    c_oid,
                    "used_context",
                )?;
            }

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

            let mut headers = BTreeMap::new();
            headers.insert("task_id".to_string(), earmark_core::HeaderValue::String(task_id.clone()));

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
                DepositValidationContext {
                    namespace: Some("examples.earmark-dev".to_string()),
                    headers,
                },
            )?;

            if let Some(d_oid) = dispatch_oid {
                create_orchestration_relation(
                    ctx,
                    d_oid,
                    object_ref.id.clone(),
                    "produced_evidence",
                )?;
            }

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

            let tasks = if let Some(title) = &args.title {
                vec![self::adapters::native_json::NativeTaskData {
                    task_id: args.task_id.clone(),
                    title: title.clone(),
                    description: args.description.clone().unwrap_or_default(),
                    priority: args.priority.clone().unwrap_or_else(|| "medium".to_string()),
                    status: args.status.clone().unwrap_or_else(|| "proposed".to_string()),
                    raw_text: String::new(),
                }]
            } else {
                match source.as_str() {
                    "native-json" | "local-json" => {
                        self::adapters::native_json::ingest_from_json(&args.task_id)?
                    }
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

                let runtime_surface = RuntimeToolSurface {
                    store,
                    index: index_ref,
                    provider_service: ctx.provider_registry,
                };

                let prov = RuntimeProvenance {
                    actor: "operator".to_string(),
                    source_type: "cli".to_string(),
                };

                let mut headers = BTreeMap::new();
                headers.insert("task_id".to_string(), earmark_core::HeaderValue::String(task.task_id.clone()));

                let object_ref = runtime_surface.deposit_object(
                    "work_item".to_string(),
                    Some("object".to_string()),
                    Some(task.title.clone()),
                    payload,
                    prov,
                    DepositValidationContext {
                        namespace: Some("examples.earmark-dev".to_string()),
                        headers,
                    },
                )?;

                index_ref.rebuild_from_store(store)?;

                let vr = VersionRef::new(object_ref.id.clone(), object_ref.version_id.clone());
                let stored_object = store.read_version(&vr)?;
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
                    "kind": "implementation_task_ingest",
                    "source": source,
                    "tasks": ingested,
                }),
            );

            Ok(())
        }
        OrchestrationAction::RecordContext(args) => {
            require_initialized_workspace(store)?;
            let index_ref = ctx
                .index
                .as_ref()
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

            if !args.path.exists() {
                return Err(CliError::not_found(format!(
                    "context file not found: {}",
                    args.path.display()
                )));
            }
            let raw_text = fs::read_to_string(&args.path)?;
            let context_data: serde_json::Value = serde_json::from_str(&raw_text)?;

            let payload = json!({
                "task_id": task_id,
                "task_object_id": task_oid.as_str(),
                "data": context_data,
                "recorded_by": "orchestration record-context"
            });

            let title = format!("Context packet for {}", task_id);
            let obj_ref = deposit_orchestration_object(ctx, "context_packet", Some(title), payload)?;

            create_orchestration_relation(
                ctx,
                task_oid,
                obj_ref.id.clone(),
                "has_context",
            )?;

            let vr = VersionRef::new(obj_ref.id.clone(), obj_ref.version_id.clone());
            if let Ok(stored_object) = store.read_version(&vr) {
                let _ = mirror_surface(store, &stored_object);
            }

            emit(
                as_json,
                json!({
                    "kind": "orchestration_context_packet",
                    "task_id": task_id,
                    "object_id": obj_ref.id.as_str(),
                    "version_id": obj_ref.version_id.as_str(),
                }),
            );

            Ok(())
        }
        OrchestrationAction::RecordGate(args) => {
            require_initialized_workspace(store)?;
            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required"))?;

            let (task_oid, _class, _task_summary, task_payload) =
                find_orchestration_task(index_ref, store, &args.task_id)?.ok_or_else(|| {
                    CliError::not_found(format!("task {} not found", args.task_id))
                })?;

            let dispatch_oid = if let Some(ref d_id) = args.dispatch_id {
                let (oid, class, _, _) = find_orchestration_task(index_ref, store, d_id)?.ok_or_else(|| {
                    CliError::not_found(format!("dispatch {} not found", d_id))
                })?;
                if class != "dispatch" {
                    return Err(CliError::argument(format!("object {} is a {}, not a dispatch", d_id, class)));
                }
                Some(oid)
            } else {
                None
            };

            let task_id = task_payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let normalized_status = normalize_gate_status(&args.status);

            let mut log_path = String::new();
            let mut log_excerpt = String::new();
            if let Some(ref path) = args.log {
                log_path = path.to_string_lossy().to_string();
                if !path.exists() {
                    return Err(CliError::not_found(format!(
                        "log file not found: {}",
                        path.display()
                    )));
                }
                let content = fs::read_to_string(path)?;
                let lines: Vec<&str> = content.lines().collect();
                let start_idx = if lines.len() > 120 {
                    lines.len() - 120
                } else {
                    0
                };
                let selected_lines = &lines[start_idx..];
                let mut excerpt = selected_lines.join("\n");
                if excerpt.len() > 12000 {
                    let char_start = excerpt.len() - 12000;
                    let mut byte_idx = char_start;
                    while byte_idx < excerpt.len() && !excerpt.is_char_boundary(byte_idx) {
                        byte_idx += 1;
                    }
                    excerpt = excerpt[byte_idx..].to_string();
                }
                log_excerpt = excerpt;
            }

            let mut payload = json!({
                "task_id": task_id,
                "task_object_id": task_oid.as_str(),
                "command": args.command,
                "status": normalized_status,
                "log_path": log_path,
                "log_excerpt": log_excerpt,
                "recorded_by": "orchestration record-gate"
            });

            if let Some(ref d_oid) = dispatch_oid {
                if let Some(obj) = payload.as_object_mut() {
                    obj.insert("dispatch_id".to_string(), json!(d_oid.as_str()));
                }
            }

            let title = format!("Gate result: {} for {}", args.command, task_id);
            let obj_ref = deposit_orchestration_object(ctx, "gate_result", Some(title), payload)?;

            if let Some(d_oid) = dispatch_oid {
                create_orchestration_relation(
                    ctx,
                    d_oid,
                    obj_ref.id.clone(),
                    "checked_by",
                )?;
            } else {
                create_orchestration_relation(
                    ctx,
                    task_oid.clone(),
                    obj_ref.id.clone(),
                    "has_gate_result",
                )?;
            }

            let vr = VersionRef::new(obj_ref.id.clone(), obj_ref.version_id.clone());
            if let Ok(stored_object) = store.read_version(&vr) {
                let _ = mirror_surface(store, &stored_object);
            }

            emit(
                as_json,
                json!({
                    "kind": "orchestration_gate_result",
                    "task_id": task_id,
                    "task_object_id": task_oid.as_str(),
                    "gate_result_object_id": obj_ref.id.as_str(),
                    "gate_result_version_id": obj_ref.version_id.as_str(),
                    "command": args.command,
                    "status": normalized_status
                }),
            );

            Ok(())
        }
        OrchestrationAction::Review(args) => {
            require_initialized_workspace(store)?;

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for review"))?;

            let runtime_surface = RuntimeToolSurface {
                store,
                index: index_ref,
                provider_service: ctx.provider_registry,
            };

            let task_arg = args.task_id.to_lowercase();

            let task_filter = QueryFilter {
                class: None, // Search all classes, find_orchestration_task already handles this better
                ..Default::default()
            };
            let task_results = index_ref.query_objects(&task_filter)?;

            let mut found_task: Option<(ObjectSummary, serde_json::Value)> = None;
            let mut candidates = Vec::new();
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
                    candidates.push((summary.clone(), payload));
                }
            }

            // Prioritize work_item or implementation_task
            for (summary, payload) in &candidates {
                let class = summary.class.as_deref().unwrap_or("");
                if class == "work_item" || class == "implementation_task" {
                    found_task = Some((summary.clone(), payload.clone()));
                    break;
                }
            }

            // Fallback to first candidate if no priority class found
            if found_task.is_none() {
                found_task = candidates.into_iter().next();
            }

            let (task_summary, task_payload) = found_task
                .ok_or_else(|| CliError::not_found(format!("task {} not found", args.task_id)))?;

            let task_id = task_payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let decision = normalize_review_status(&args.decision);

            let prov = RuntimeProvenance {
                actor: "operator".to_string(),
                source_type: "cli".to_string(),
            };

            let review_payload = json!({
                "task_id": task_id,
                "decision": decision,
                "comment": args.comment.clone().unwrap_or_default(),
            });

            let review_object_ref = runtime_surface.deposit_object(
                "review".to_string(),
                Some("object".to_string()),
                Some(format!("Review for {}", args.task_id)),
                review_payload,
                prov.clone(),
                DepositValidationContext {
                    namespace: Some("examples.earmark-dev".to_string()),
                    headers: BTreeMap::new(),
                },
            )?;

            let task_oid = ObjectId::parse(task_summary.object_id.clone())?;

            let closure_payload = json!({
                "task_id": task_id,
                "review_id": review_object_ref.id.as_str(),
                "outcome": decision,
            });

            let closure_object_ref = runtime_surface.deposit_object(
                "closure".to_string(),
                Some("object".to_string()),
                Some(format!("Closure for {}", args.task_id)),
                closure_payload,
                prov.clone(),
                DepositValidationContext {
                    namespace: Some("examples.earmark-dev".to_string()),
                    headers: BTreeMap::new(),
                },
            )?;

            create_orchestration_relation(
                ctx,
                review_object_ref.id.clone(),
                closure_object_ref.id.clone(),
                "causes",
            )?;

            create_orchestration_relation(
                ctx,
                closure_object_ref.id.clone(),
                task_oid.clone(),
                "closes",
            )?;
            let head_stored = store.read_head(&task_oid)?.ok_or_else(|| {
                CliError::not_found(format!("task {} head not found", args.task_id))
            })?;

            let (process_token, review_token) = match decision.as_str() {
                "accepted" => ("closed", "accepted"),
                "rejected" => ("closed", "rejected"),
                "needs_revision" => ("proposed", "needs_revision"),
                _ => unreachable!(),
            };

            let mut standing_values = BTreeMap::new();
            standing_values.insert(
                DimensionId::from_static("kernel:process"),
                TokenId::from_static(process_token),
            );
            standing_values.insert(
                DimensionId::from_static("kernel:review"),
                TokenId::from_static(review_token),
            );
            let standing = Standing {
                values: standing_values,
            };

            let mapped_status = match decision.as_str() {
                "accepted" => "implemented",
                "rejected" => "closed",
                "needs_revision" => "proposed",
                _ => unreachable!(),
            };

            let mut updated_payload = task_payload.clone();
            if let Some(obj) = updated_payload.as_object_mut() {
                obj.insert("status".to_string(), json!(mapped_status));
            }

            let new_task_object = StoredObject::with_parent(
                &head_stored,
                standing,
                head_stored.envelope.headers.clone(),
                StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&updated_payload)?),
            );

            let task_version_ref = write_object_and_index(store, index_ref, &new_task_object)?;

            create_orchestration_relation(
                ctx,
                task_oid.clone(),
                review_object_ref.id.clone(),
                "has_review",
            )?;

            index_ref.rebuild_from_store(store)?;

            let review_vr = VersionRef::new(
                review_object_ref.id.clone(),
                review_object_ref.version_id.clone(),
            );
            if let Ok(review_stored) = store.read_version(&review_vr) {
                let _ = mirror_surface(store, &review_stored);
            }

            let task_vr = VersionRef::new(task_oid.clone(), task_version_ref.version_id.clone());
            if let Ok(task_stored) = store.read_version(&task_vr) {
                let _ = mirror_surface(store, &task_stored);
            }

            emit(
                as_json,
                json!({
                    "kind": "orchestration_review_decision",
                    "task_id": task_id,
                    "decision": decision,
                    "comment": args.comment.clone().unwrap_or_default(),
                    "review_decision_object_id": review_object_ref.id.as_str(),
                    "task_object_id": task_oid.as_str(),
                    "task_new_version_id": task_version_ref.version_id.as_str(),
                }),
            );

            Ok(())
        }
        OrchestrationAction::Show(args) => {
            require_initialized_workspace(store)?;

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for show"))?;

            let task_arg = args.task_id.to_lowercase();

            let (task_oid, class, task_summary, task_payload) =
                find_orchestration_task(index_ref, store, &task_arg)?.ok_or_else(|| {
                    CliError::not_found(format!("task {} not found", args.task_id))
                })?;

            if class == "work_item" {
                let connected_objects = traverse_orchestration_graph(index_ref, store, &task_oid)?;

                let mut context_packets = Vec::new();
                let mut dispatches = Vec::new();
                let mut trace_events = Vec::new();
                let mut evidence = Vec::new();
                let mut reviews = Vec::new();
                let mut closures = Vec::new();
                let mut git_snapshots = Vec::new();
                let mut gate_results = Vec::new();

                for (_oid, summary, payload, _captured_at) in connected_objects {
                    match summary.class.as_deref().unwrap_or("") {
                        "context_packet" => context_packets.push(payload),
                        "dispatch" => dispatches.push(payload),
                        "trace_event" => trace_events.push(payload),
                        "evidence" => evidence.push(payload),
                        "review" => reviews.push(payload),
                        "closure" => closures.push(payload),
                        "git_snapshot" => git_snapshots.push(payload),
                        "gate_result" => gate_results.push(payload),
                        _ => {}
                    }
                }

                let title = task_payload
                    .get("title")
                    .and_then(|v| v.as_str())
                    .or(task_summary.title.as_deref())
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

                emit(
                    as_json,
                    json!({
                        "kind": "orchestration_work_item_show",
                        "work_item_id": task_oid.as_str(),
                        "title": title,
                        "description": description,
                        "status": status,
                        "priority": priority,
                        "context_packets": context_packets,
                        "dispatches": dispatches,
                        "trace_events": trace_events,
                        "evidence": evidence,
                        "reviews": reviews,
                        "closures": closures,
                        "git_snapshots": git_snapshots,
                        "gate_results": gate_results
                    }),
                );

                return Ok(());
            }

            // Fallback to legacy implementation_task Show command logic
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

            // Also traverse graph for implementation_task to find git_snapshots and gate_results
            let connected_objects = traverse_orchestration_graph(index_ref, store, &task_oid)?;
            let mut git_snapshots = Vec::new();
            let mut gate_results = Vec::new();
            for (_oid, summary, payload, _captured_at) in connected_objects {
                match summary.class.as_deref().unwrap_or("") {
                    "git_snapshot" => git_snapshots.push(payload),
                    "gate_result" => gate_results.push(payload),
                    _ => {}
                }
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
                    "git_snapshots": git_snapshots,
                    "gate_results": gate_results
                }),
            );

            Ok(())
        }
        OrchestrationAction::Timeline(args) => {
            require_initialized_workspace(store)?;

            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for timeline"))?;

            let task_arg = args.task_id.to_lowercase();
            let (task_oid, _class, _task_summary, _task_payload) =
                find_orchestration_task(index_ref, store, &task_arg)?.ok_or_else(|| {
                    CliError::not_found(format!("task {} not found", args.task_id))
                })?;

            let mut connected_objects = traverse_orchestration_graph(index_ref, store, &task_oid)?;

            // Sort chronologically by captured_at
            connected_objects.sort_by_key(|(_, _, _, captured_at)| *captured_at);

            let events: Vec<serde_json::Value> = connected_objects
                .into_iter()
                .map(|(oid, summary, payload, captured_at)| {
                    let class_name = summary.class.clone().unwrap_or_default();
                    let title = payload
                        .get("title")
                        .and_then(|v| v.as_str())
                        .or(summary.title.as_deref())
                        .unwrap_or("")
                        .to_string();
                    let summary_text = get_object_summary_text(&class_name, &payload);

                    json!({
                        "id": oid.as_str(),
                        "class": class_name,
                        "title": title,
                        "timestamp": captured_at.to_rfc3339(),
                        "summary": summary_text,
                    })
                })
                .collect();

            emit(
                as_json,
                json!({
                    "kind": "orchestration_timeline",
                    "work_item_id": task_oid.as_str(),
                    "task_id": task_oid.as_str(),
                    "events": events,
                }),
            );

            Ok(())
        }
        OrchestrationAction::List(args) => {
            require_initialized_workspace(store)?;
            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required"))?;

            let terminal_statuses: std::collections::HashSet<&str> = [
                "closed",
                "completed",
                "deferred",
                "rejected",
                "superseded",
                "cancelled",
            ]
            .iter()
            .cloned()
            .collect();

            let mut filtered_terminal_status = false;
            let target_status = args.status.as_ref().map(|s| s.to_lowercase());

            if let Some(ref req_status) = target_status {
                if !args.include_closed && terminal_statuses.contains(req_status.as_str()) {
                    filtered_terminal_status = true;
                }
            }

            let mut tasks = Vec::new();

            if !filtered_terminal_status {
                for class in &[
                    "work_item",
                    "implementation_task",
                    "dispatch",
                    "executor_manifest",
                    "executor_report",
                    "evidence",
                    "git_snapshot",
                    "gate_result",
                    "review",
                    "closure",
                ] {
                    let filter = QueryFilter {
                        class: Some(class.to_string()),
                        ..Default::default()
                    };
                    let results = index_ref.query_objects(&filter)?;
                    for summary in results {
                        let oid = match ObjectId::parse(summary.object_id.clone()) {
                            Ok(o) => o,
                            Err(_) => continue,
                        };
                        let vid = match VersionId::parse(summary.version_id.clone()) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let vr = VersionRef::new(oid.clone(), vid);
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

                        let status = payload
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_lowercase();

                        let is_terminal = terminal_statuses.contains(status.as_str());

                        if let Some(ref req_status) = target_status {
                            if status != *req_status {
                                continue;
                            }
                        } else if !args.include_closed && is_terminal {
                            continue;
                        }

                        let task_id = payload
                            .get("task_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let title = payload
                            .get("title")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let priority = payload
                            .get("priority")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        tasks.push((
                            oid,
                            summary.version_id.clone(),
                            class.to_string(),
                            task_id,
                            title,
                            status,
                            priority,
                            is_terminal,
                        ));
                    }
                }

                tasks.sort_by(|a, b| {
                    let a_term = a.7;
                    let b_term = b.7;
                    if a_term != b_term {
                        return a_term.cmp(&b_term);
                    }

                    let p_weight = |p: &str| match p.to_lowercase().as_str() {
                        "critical" => 0,
                        "high" => 1,
                        "medium" => 2,
                        "low" => 3,
                        _ => 4,
                    };
                    let a_p = p_weight(&a.6);
                    let b_p = p_weight(&b.6);
                    if a_p != b_p {
                        return a_p.cmp(&b_p);
                    }

                    let title_cmp = a.4.to_lowercase().cmp(&b.4.to_lowercase());
                    if title_cmp != std::cmp::Ordering::Equal {
                        return title_cmp;
                    }

                    a.0.as_str().cmp(b.0.as_str())
                });
            }

            let task_records: Vec<serde_json::Value> = tasks
                .into_iter()
                .map(|(oid, vid, class, task_id, title, status, priority, _)| {
                    json!({
                        "object_id": oid.as_str(),
                        "version_id": vid,
                        "class": class,
                        "task_id": task_id,
                        "title": title,
                        "status": status,
                        "priority": priority
                    })
                })
                .collect();

            emit(
                as_json,
                json!({
                    "kind": "orchestration_task_list",
                    "count": task_records.len(),
                    "include_closed": args.include_closed,
                    "status_filter": args.status,
                    "filtered_terminal_status": filtered_terminal_status,
                    "tasks": task_records
                }),
            );

            Ok(())
        }
        OrchestrationAction::ExplainDispatch(args) => {
            require_initialized_workspace(store)?;
            let index_ref = ctx
                .index
                .as_ref()
                .ok_or_else(|| CliError::argument("index required for explain-dispatch"))?;

            let dispatch_arg = args.dispatch_id.to_lowercase();
            let (dispatch_oid, _class, _summary, payload) =
                find_orchestration_task(index_ref, store, &dispatch_arg)?.ok_or_else(|| {
                    CliError::not_found(format!("dispatch {} not found", args.dispatch_id))
                })?;

            let mut connected_objects = traverse_orchestration_graph(index_ref, store, &dispatch_oid)?;
            connected_objects.sort_by_key(|(_, _, _, captured_at)| *captured_at);

            let events: Vec<serde_json::Value> = connected_objects
                .into_iter()
                .map(|(oid, summary, payload, captured_at)| {
                    let class_name = summary.class.clone().unwrap_or_default();
                    let summary_text = get_object_summary_text(&class_name, &payload);

                    json!({
                        "id": oid.as_str(),
                        "class": class_name,
                        "timestamp": captured_at.to_rfc3339(),
                        "summary": summary_text,
                    })
                })
                .collect();

            emit(
                as_json,
                json!({
                    "kind": "orchestration_dispatch_explanation",
                    "dispatch_id": dispatch_oid.as_str(),
                    "payload": payload,
                    "events": events,
                }),
            );

            Ok(())
        }
    }
}

fn parse_manifest_sections(text: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current_section = String::from("_preamble");
    let mut current_content = Vec::new();

    for line in text.lines() {
        if let Some(stripped) = line.strip_prefix("## ") {
            if !current_content.is_empty() {
                sections.insert(current_section, current_content.join("\n"));
            }
            current_section = stripped.trim().to_lowercase();
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
            t.strip_prefix("- ")
                .or_else(|| t.strip_prefix("* "))
                .map(|stripped| stripped.trim().to_string())
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
                let path = if let Some(stripped) = trimmed
                    .strip_prefix("- ")
                    .or_else(|| trimmed.strip_prefix("* "))
                {
                    stripped
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

#[allow(clippy::type_complexity)]
fn traverse_orchestration_graph(
    index: &DerivedIndex,
    store: &dyn ObjectStore,
    start_id: &ObjectId,
) -> Result<
    Vec<(
        ObjectId,
        ObjectSummary,
        serde_json::Value,
        chrono::DateTime<chrono::Utc>,
    )>,
    CliError,
> {
    use std::collections::HashSet;
    use std::collections::VecDeque;

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut results = Vec::new();

    queue.push_back(start_id.clone());
    visited.insert(start_id.clone());

    let orch_classes: HashSet<&str> = [
        "work_item",
        "implementation_task",
        "dispatch",
        "context_packet",
        "trace_event",
        "evidence",
        "review",
        "closure",
        "git_snapshot",
        "gate_result",
    ]
    .iter()
    .cloned()
    .collect();

    while let Some(current) = queue.pop_front() {
        let head = match store.read_head(&current) {
            Ok(Some(h)) => h,
            Ok(None) => continue,
            Err(_) => continue,
        };
        let vid = head.envelope.version_id.clone();
        let vr = VersionRef::new(current.clone(), vid.clone());
        let stored = match store.read_version(&vr) {
            Ok(s) => s,
            _ => continue,
        };
        let text = match stored.payload.as_utf8() {
            Ok(t) => t,
            _ => continue,
        };
        let payload: serde_json::Value = match serde_json::from_str(&text) {
            Ok(p) => p,
            _ => continue,
        };

        let filter = QueryFilter {
            object_id: Some(current.as_str().to_string()),
            ..Default::default()
        };
        let summaries = match index.query_objects(&filter) {
            Ok(s) => s,
            _ => continue,
        };
        let summary = match summaries.into_iter().next() {
            Some(s) => s,
            None => continue,
        };

        let class_name = summary.class.as_deref().unwrap_or("");
        if !orch_classes.contains(class_name) {
            continue;
        }

        let captured_at = head.envelope.provenance.captured_at;
        results.push((current.clone(), summary.clone(), payload, captured_at));

        let edges = match index.relation_adjacency(&current, false) {
            Ok(e) => e,
            _ => continue,
        };

        for edge in edges {
            let next_str = if edge.source_object_id == current.as_str() {
                edge.target_object_id
            } else {
                edge.source_object_id
            };

            if let Ok(next_oid) = ObjectId::parse(next_str) {
                if !visited.contains(&next_oid) {
                    visited.insert(next_oid.clone());
                    queue.push_back(next_oid);
                }
            }
        }
    }

    Ok(results)
}

fn get_object_summary_text(class: &str, payload: &serde_json::Value) -> String {
    match class {
        "work_item" => {
            let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("");
            format!("Work Item: '{}' [{}]", title, status)
        }
        "implementation_task" => {
            let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("");
            format!("Implementation Task: '{}' [{}]", title, status)
        }
        "dispatch" => {
            let attempt = payload.get("attempt").and_then(|v| v.as_u64()).unwrap_or(1);
            let executor = payload
                .get("executor")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Dispatch (Attempt #{}): executor={}", attempt, executor)
        }
        "context_packet" => {
            let task_id = payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Context Packet for task {}", task_id)
        }
        "trace_event" => {
            let msg = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Trace Event: {}", msg)
        }
        "evidence" => {
            let checksum = payload
                .get("checksum")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let desc = payload
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("Evidence: {} (checksum={})", desc, checksum)
        }
        "review" => {
            let decision = payload
                .get("decision")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let outcome = payload
                .get("outcome")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if outcome.is_empty() {
                format!("Review: decision={}", decision)
            } else {
                format!("Review: decision={} ({})", decision, outcome)
            }
        }
        "git_snapshot" => {
            let phase = payload.get("phase").and_then(|v| v.as_str()).unwrap_or("");
            let commit = payload.get("commit").and_then(|v| v.as_str()).unwrap_or("");
            format!("Git Snapshot: phase={} (commit={})", phase, commit)
        }
        "gate_result" => {
            let command = payload
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("");
            format!("Gate Result: '{}' -> {}", command, status)
        }
        _ => String::new(),
    }
}

fn find_orchestration_task(
    index: &DerivedIndex,
    store: &dyn ObjectStore,
    task_arg: &str,
) -> Result<Option<(ObjectId, String, ObjectSummary, serde_json::Value)>, CliError> {
    let task_arg = task_arg.to_lowercase();
    let mut candidates = Vec::new();

    for class in &[
        "work_item",
        "implementation_task",
        "dispatch",
        "executor_manifest",
        "executor_report",
        "evidence",
        "git_snapshot",
        "gate_result",
        "review",
        "closure",
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
            let vid = match VersionId::parse(summary.version_id.clone()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let vr = VersionRef::new(oid.clone(), vid);
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

            let oid_lower = summary.object_id.to_lowercase();
            let tid = payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            let is_work_item = *class == "work_item";

            let score = if oid_lower == task_arg {
                if is_work_item {
                    8
                } else {
                    7
                }
            } else if oid_lower.starts_with(&task_arg) {
                if is_work_item {
                    6
                } else {
                    5
                }
            } else if !tid.is_empty() && tid == task_arg {
                if is_work_item {
                    4
                } else {
                    3
                }
            } else if !tid.is_empty() && tid.starts_with(&task_arg) {
                if is_work_item {
                    2
                } else {
                    1
                }
            } else {
                0
            };

            if score > 0 {
                candidates.push((oid, class.to_string(), summary, payload, score));
            }
        }
    }

    candidates.sort_by_key(|c| c.4);
    if let Some(best) = candidates.pop() {
        Ok(Some((best.0, best.1, best.2, best.3)))
    } else {
        Ok(None)
    }
}

fn deposit_orchestration_object(
    ctx: &CommandContext,
    class: &str,
    title: Option<String>,
    payload: serde_json::Value,
) -> Result<earmark_core::ObjectRef, CliError> {
    let index_ref = ctx
        .index
        .as_ref()
        .ok_or_else(|| CliError::argument("index required"))?;
    let runtime_surface = RuntimeToolSurface {
        store: ctx.store,
        index: index_ref,
        provider_service: ctx.provider_registry,
    };
    let prov = RuntimeProvenance {
        actor: "operator".to_string(),
        source_type: "cli".to_string(),
    };
    let mut headers = BTreeMap::new();
    if let Some(task_id) = payload.get("task_id").and_then(|v| v.as_str()) {
        headers.insert("task_id".to_string(), earmark_core::HeaderValue::String(task_id.to_string()));
    }
    if let Some(command) = payload.get("command").and_then(|v| v.as_str()) {
        headers.insert("command".to_string(), earmark_core::HeaderValue::String(command.to_string()));
    }
    if let Some(phase) = payload.get("phase").and_then(|v| v.as_str()) {
        headers.insert("phase".to_string(), earmark_core::HeaderValue::String(phase.to_string()));
    }
    if let Some(commit) = payload.get("commit").and_then(|v| v.as_str()) {
        headers.insert("commit".to_string(), earmark_core::HeaderValue::String(commit.to_string()));
    }
    if let Some(branch) = payload.get("branch").and_then(|v| v.as_str()) {
        headers.insert("branch".to_string(), earmark_core::HeaderValue::String(branch.to_string()));
    }
    if let Some(status) = payload.get("status").and_then(|v| v.as_str()) {
        headers.insert("status".to_string(), earmark_core::HeaderValue::String(status.to_string()));
    }

    let obj_ref = runtime_surface.deposit_object(
        class.to_string(),
        Some("object".to_string()),
        title,
        payload,
        prov,
        DepositValidationContext {
            namespace: Some("examples.earmark-dev".to_string()),
            headers,
        },
    )?;
    index_ref.rebuild_from_store(ctx.store)?;
    Ok(obj_ref)
}

fn normalize_work_item_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "proposed" | "new" | "draft" => "proposed".to_string(),
        "ready" | "todo" | "backlog" => "ready".to_string(),
        "dispatched" | "running" | "started" | "in_progress" => "dispatched".to_string(),
        "under_review" | "review" | "qa" => "under_review".to_string(),
        "closed" | "done" | "completed" | "finished" => "closed".to_string(),
        "followup_required" | "followup" | "partial" => "followup_required".to_string(),
        "blocked" | "stuck" | "hold" => "blocked".to_string(),
        other => other.to_string(),
    }
}

fn normalize_dispatch_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "queued" | "pending" | "waiting" => "queued".to_string(),
        "running" | "in_progress" | "executing" => "running".to_string(),
        "succeeded" | "success" | "done" | "passed" | "ok" => "succeeded".to_string(),
        "failed" | "fail" | "error" | "err" => "failed".to_string(),
        "cancelled" | "cancel" | "abort" | "stopped" => "cancelled".to_string(),
        other => other.to_string(),
    }
}

fn normalize_gate_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "pass" | "passed" | "success" | "ok" => "pass".to_string(),
        "fail" | "failed" | "error" => "fail".to_string(),
        "skipped" | "skip" => "skipped".to_string(),
        other => other.to_string(),
    }
}

fn normalize_review_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "unreviewed" | "pending" | "none" | "draft" | "proposed" => "unreviewed".to_string(),
        "accepted" | "approve" | "approved" | "pass" | "ok" => "accepted".to_string(),
        "rejected" | "reject" | "deny" | "denied" | "fail" => "rejected".to_string(),
        other => other.to_string(),
    }
}

fn create_orchestration_relation(
    ctx: &CommandContext,
    source: ObjectId,
    target: ObjectId,
    relation_type: &str,
) -> Result<(), CliError> {
    let index_ref = ctx
        .index
        .as_ref()
        .ok_or_else(|| CliError::argument("index required"))?;
    let runtime_surface = RuntimeToolSurface {
        store: ctx.store,
        index: index_ref,
        provider_service: ctx.provider_registry,
    };
    let prov = RuntimeProvenance {
        actor: "operator".to_string(),
        source_type: "cli".to_string(),
    };
    runtime_surface.create_relation(source, target, relation_type.to_string(), json!({}), prov)?;
    index_ref.rebuild_from_store(ctx.store)?;
    Ok(())
}

fn run_git_cmd(args: &[&str]) -> Result<String, CliError> {
    let output = std::process::Command::new("git").args(args).output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Err(CliError::argument(format!(
                    "git command failed: git {}. Error: {}",
                    args.join(" "),
                    err
                )))
            }
        }
        Err(e) => Err(CliError::argument(format!("failed to execute git: {}", e))),
    }
}
