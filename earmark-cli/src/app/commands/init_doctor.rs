use crate::app::common::CommandContext;
use crate::app::emit;
use crate::app::CliError;
use earmark_index::DerivedIndex;
use earmark_store::CanonicalStore;
use serde_json::json;

pub(crate) fn handle_init(ctx: &CommandContext) -> Result<(), CliError> {
    let store = ctx.store;
    let root = store.root();
    let canonical_dir = root.join(".earmark").join("canonical");
    let declarations_dir = store.declarations_dir();
    let work_surfaces_dir = root.join(".earmark").join("work_surfaces");
    let index_path = root.join(".earmark").join("derived").join("index.sqlite");
    emit(
        ctx.as_json,
        json!({
            "ok": true,
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

pub(crate) fn handle_doctor(ctx: &CommandContext) -> Result<(), CliError> {
    let store = ctx.store;
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
    let canonical_count = store_scan.as_ref().map(|v| v.len() as u64).unwrap_or(0);

    let index_path = store
        .root()
        .join(".earmark")
        .join("derived")
        .join("index.sqlite");
    let index_exists = index_path.exists();
    let mut warnings: Vec<String> = Vec::new();
    let mut all_ok = store_scan_ok;

    let (index_open_ok, indexed_count, indexed_head_count, counts_match) = if index_exists {
        match DerivedIndex::open_existing(store.root()) {
            Ok(idx) => {
                let obj_count = idx.object_count().unwrap_or(0);
                let head_count = idx.head_count().unwrap_or(0);
                let match_ok = obj_count == canonical_count;
                if !match_ok {
                    warnings.push(format!(
                        "store/index count mismatch: {} canonical objects vs {} indexed objects",
                        canonical_count, obj_count
                    ));
                }
                all_ok = all_ok && match_ok;
                (true, obj_count, head_count, match_ok)
            }
            Err(e) => {
                warnings.push(format!("index open failed: {}", e));
                all_ok = false;
                (false, 0, 0, false)
            }
        }
    } else {
        warnings.push(
            "derived index is missing; run a write command or system register to rebuild it"
                .to_string(),
        );
        all_ok = false;
        (false, 0, 0, false)
    };

    if !store_scan_ok {
        warnings.push(
            "canonical store scan failed; review .earmark/canonical for corruption".to_string(),
        );
    }

    emit(
        ctx.as_json,
        json!({
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
            "provider_capabilities": earmark_exec::compiled_provider_capabilities(),
            "warnings": warnings,
            "next_commands": if all_ok {
                vec!["em status", "em run list"]
            } else {
                let mut cmds = Vec::new();
                if !layout.is_initialized() {
                    cmds.push("em init");
                }
                if !index_exists || !index_open_ok || !counts_match {
                    cmds.push("em system register <manifest>");
                    cmds.push("no standalone index rebuild command exists yet; system registration triggers a full rebuild");
                }
                cmds.push("em status");
                cmds
            },
        }),
    );

    Ok(())
}
