use crate::app::common::CommandContext;
use crate::app::emit;
use crate::app::CliError;
use crate::cli::DoctorArgs;
use earmark_index::DerivedIndex;
use earmark_store::{StoreScanner, WorkspaceLayout};
use serde_json::json;

pub(crate) fn handle_init(ctx: &mut CommandContext) -> Result<(), CliError> {
    let store = ctx.store;
    let root = store.root();
    let canonical_dir = root.join(".earmark").join("canonical");
    let declarations_dir = store.declarations_dir();
    let work_surfaces_dir = root.join(".earmark").join("work_surfaces");
    let index_path = root.join(".earmark").join("derived").join("index.sqlite");
    emit(
        ctx.as_json,
        json!({
            "summary": "workspace initialized",
            "root": root.display().to_string(),
            "paths": {
                "canonical_dir": canonical_dir.display().to_string(),
                "declarations_dir": declarations_dir.display().to_string(),
                "work_surfaces_dir": work_surfaces_dir.display().to_string(),
                "index_path": index_path.display().to_string(),
            },
            "next_commands": [
                "em doctor",
                "em status",
                "em declare list-examples"
            ],
        }),
    );
    Ok(())
}

pub(crate) fn handle_doctor(ctx: &mut CommandContext, args: &DoctorArgs) -> Result<(), CliError> {
    let store = ctx.store;

    if args.repair_index {
        let index = ctx
            .index
            .as_mut()
            .ok_or_else(|| CliError::not_found("index available for workspace command"))?;
        let report = index.rebuild_from_store(store)?;
        let partial = !report.skipped_entries.is_empty();
        if !partial {
            index.clear_dirty()?;
        }
        emit(
            ctx.as_json,
            json!({
                "kind": "doctor",
                "ok": !partial,
                "summary": if partial {
                    "index repaired partially; canonical store still has skipped entries"
                } else {
                    "index repaired successfully from canonical store"
                },
                "indexed_object_count": report.indexed_objects,
                "skipped_canonical_entries": report.skipped_entries.iter().map(|entry| {
                    json!({
                        "path": entry.path.display().to_string(),
                        "reason": entry.reason,
                    })
                }).collect::<Vec<_>>(),
                "next_commands": if partial {
                    vec![
                        "repair canonical store entries listed in skipped_canonical_entries",
                        "em doctor --repair-index",
                        "em doctor",
                    ]
                } else {
                    vec!["em doctor", "em status"]
                },
            }),
        );
        return Ok(());
    }

    let layout = store.layout_status();
    if !layout.is_initialized() {
        emit(
            ctx.as_json,
            json!({
                "ok": false,
                "summary": "workspace is not initialized",
                "root": store.root().display().to_string(),
                "layout": layout,
                "warnings": ["workspace layout is incomplete"],
                "next_commands": ["em init"],
            }),
        );
        return Ok(());
    }

    let store_scan = store.scan_objects();
    let store_scan_ok = store_scan.is_ok();

    let mut warnings: Vec<String> = Vec::new();
    let mut all_ok = store_scan_ok;

    let canonical_count = if let Ok(diag) = &store_scan {
        if !diag.skipped_entries.is_empty() {
            all_ok = false;
            for skipped in &diag.skipped_entries {
                warnings.push(format!(
                    "skipped corrupted or incomplete object at {}: {}",
                    skipped.path.display(),
                    skipped.reason
                ));
            }
        }
        diag.scanned_objects.len() as u64
    } else {
        0
    };

    let index_path = store
        .root()
        .join(".earmark")
        .join("derived")
        .join("index.sqlite");
    let index_exists = index_path.exists();
    let mut dirty_marker = None;

    let (index_open_ok, indexed_count, indexed_head_count, counts_match, index_stale) =
        if index_exists {
            match DerivedIndex::open_existing(store.root()) {
                Ok(idx) => {
                    dirty_marker = idx.dirty_status().unwrap_or(None);
                    if dirty_marker.is_some() {
                        all_ok = false;
                        warnings.push("index is marked dirty; repair required".to_string());
                    }

                    let obj_count = idx.object_count().unwrap_or(0);
                    let head_count = idx.head_count().unwrap_or(0);
                    let match_ok = obj_count == canonical_count;
                    if !match_ok {
                        warnings.push(format!(
                        "store/index count mismatch: {} canonical objects vs {} indexed objects",
                        canonical_count, obj_count
                    ));
                        all_ok = false;
                    }

                    // Check for staleness
                    let mut stale = false;
                    if let Ok(index_meta) = std::fs::metadata(&index_path) {
                        if let Ok(index_mtime) = index_meta.modified() {
                            // Find newest envelope.json mtime
                            let mut max_mtime = None;
                            for entry in walkdir::WalkDir::new(
                                store
                                    .root()
                                    .join(".earmark")
                                    .join("canonical")
                                    .join("objects"),
                            )
                            .into_iter()
                            .filter_map(|e| e.ok())
                            {
                                if entry.file_name() == "envelope.json" {
                                    if let Ok(meta) = entry.metadata() {
                                        if let Ok(mtime) = meta.modified() {
                                            if max_mtime.map_or(true, |max| mtime > max) {
                                                max_mtime = Some(mtime);
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(max) = max_mtime {
                                if max > index_mtime {
                                    stale = true;
                                    warnings.push("index may be stale; canonical objects have been modified since last index update".to_string());
                                    all_ok = false;
                                }
                            }
                        }
                    }

                    all_ok = all_ok && match_ok && !stale;
                    (true, obj_count, head_count, match_ok, stale)
                }
                Err(e) => {
                    warnings.push(format!("index open failed: {}", e));
                    all_ok = false;
                    (false, 0, 0, false, false)
                }
            }
        } else {
            warnings.push(
                "derived index is missing; run a write command or system register to rebuild it"
                    .to_string(),
            );
            all_ok = false;
            (false, 0, 0, false, false)
        };

    if let Err(e) = &store_scan {
        warnings.push(format!(
            "canonical store scan failed; review .earmark/canonical for corruption: {}",
            e
        ));
    }

    emit(
        ctx.as_json,
        json!({
            "kind": "doctor",
            "id": "workspace",
            "ok": all_ok,
            "summary": if all_ok { "workspace health checks passed" } else { "workspace health checks reported issues" },
            "root": store.root().display().to_string(),
            "layout": layout,
            "store_scan_ok": store_scan_ok,
            "canonical_object_count": canonical_count,
            "index_exists": index_exists,
            "index_open_ok": index_open_ok,
            "indexed_object_count": indexed_count,
            "indexed_head_count": indexed_head_count,
            "counts_match": counts_match,
            "index_is_dirty": dirty_marker.is_some(),
            "index_dirty_marker": dirty_marker,
            "index_is_stale": index_stale,
            "provider_capabilities": earmark_exec::compiled_provider_capabilities(),
            "warnings": warnings,
            "next_commands": if all_ok {
                vec!["em status", "em run list"]
            } else {
                let mut cmds = Vec::new();
                if !layout.is_initialized() {
                    cmds.push("em init");
                }
                if !index_exists || !index_open_ok || !counts_match || dirty_marker.is_some() || index_stale {
                    cmds.push("em doctor --repair-index");
                }
                cmds.push("em status");
                cmds
            },
        }),
    );

    Ok(())
}
