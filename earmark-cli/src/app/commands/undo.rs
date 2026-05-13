use crate::app::common::{CliError, CommandContext};
use crate::app::{emit, list_change_sets, list_change_sets_by_run, load_run_record_by_id};
use crate::cli::{UndoAction, UndoCommand};
use earmark_core::{
    ChangeSetId, Kind, ObjectId, Provenance, RunRecord, Standing, UndoRecord, UndoRecordId,
};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_store::{PayloadEncoding, StoredObject, StoredPayload};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct UndoImpact {
    pub run_id: String,
    pub change_set_ids: Vec<ChangeSetId>,
    pub created_object_ids: Vec<ObjectId>,
    pub created_relation_ids: Vec<ObjectId>,
    pub updated_object_ids: Vec<ObjectId>,
    pub blocking_reasons: Vec<String>,
}

pub fn handle(ctx: &CommandContext, command: &UndoCommand) -> Result<(), CliError> {
    match &command.action {
        UndoAction::Run { run_id, reason } => handle_undo_run(ctx, run_id, reason.as_deref()),
    }
}

fn handle_undo_run(
    ctx: &CommandContext,
    run_id: &str,
    reason: Option<&str>,
) -> Result<(), CliError> {
    let run = load_run_record_by_id(ctx.store, run_id)?;
    let impact = calculate_undo_impact(ctx, &run)?;

    if !impact.blocking_reasons.is_empty() {
        return Err(CliError::argument(format!(
            "Undo blocked for run {}: {}",
            run_id,
            impact.blocking_reasons.join(", ")
        )));
    }

    let undo_record = UndoRecord {
        id: UndoRecordId::new(),
        target_run_id: run.run_id.clone(),
        reverted_change_set_ids: impact.change_set_ids.clone(),
        created_object_ids: impact.created_object_ids.clone(),
        created_relation_ids: impact.created_relation_ids.clone(),
        restored_heads: Vec::new(), // Not used in v1
        reason: reason.map(|s| s.to_string()),
        created_at: chrono::Utc::now(),
    };

    let payload = StoredPayload::new(
        PayloadEncoding::Json,
        serde_json::to_vec(&undo_record).map_err(CliError::Json)?,
    );

    let object = StoredObject::new_with_id(
        undo_record.id.as_object_id(),
        Kind::UndoRecord,
        None,
        Standing::default(),
        Provenance::direct_input("system"),
        BTreeMap::new(),
        payload,
        Vec::new(),
    );

    write_object_and_index(ctx.store, ctx.index.as_ref().unwrap(), &object)?;

    emit(
        ctx.as_json,
        json!({
            "ok": true,
            "summary": format!("run {} undone", run_id),
            "undo_record_id": undo_record.id.as_str(),
            "impact": {
                "objects_hidden": impact.created_object_ids.len(),
                "relations_hidden": impact.created_relation_ids.len(),
            }
        }),
    );

    Ok(())
}

fn calculate_undo_impact(ctx: &CommandContext, run: &RunRecord) -> Result<UndoImpact, CliError> {
    let store = ctx.store;
    let index = ctx.index.as_ref().unwrap();

    let mut impact = UndoImpact {
        run_id: run.run_id.clone(),
        change_set_ids: Vec::new(),
        created_object_ids: Vec::new(),
        created_relation_ids: Vec::new(),
        updated_object_ids: Vec::new(),
        blocking_reasons: Vec::new(),
    };

    // 1. Check if already undone
    if let Some(undo_id) = index.is_run_undone(&run.run_id)? {
        impact.blocking_reasons.push(format!(
            "run {} is already undone by {}",
            run.run_id, undo_id
        ));
        return Ok(impact);
    }

    // 2. Collect artifacts from change sets
    let change_sets = list_change_sets_by_run(store, &run.run_id)?;
    for cs in change_sets {
        impact.change_set_ids.push(cs.id.clone());
        impact
            .created_object_ids
            .extend(cs.created_object_ids.clone());
        impact
            .created_relation_ids
            .extend(cs.created_relation_ids.clone());
        impact
            .updated_object_ids
            .extend(cs.updated_object_ids.clone());
    }

    // 3. Block if updated objects exist (v1 boundary)
    if !impact.updated_object_ids.is_empty() {
        impact.blocking_reasons.push(format!(
            "v1 undo does not support updated objects ({} updated)",
            impact.updated_object_ids.len()
        ));
    }

    // 4. Check downstream dependencies
    let all_change_sets = list_change_sets(store)?;

    // Sort change sets by creation time to find later ones
    let mut sorted_change_sets = all_change_sets.clone();
    sorted_change_sets.sort_by_key(|cs| cs.created_at);

    let our_first_cs_time = run.started_at;

    for cs in sorted_change_sets {
        if cs.run_id == run.run_id {
            continue;
        }
        if cs.created_at <= our_first_cs_time {
            continue;
        }

        // Check if the consuming change set is itself undone
        if index.is_run_undone(&cs.run_id)?.is_some() {
            continue;
        }

        for input_id in &cs.input_object_ids {
            if impact.created_object_ids.contains(input_id) {
                impact.blocking_reasons.push(format!(
                    "object {} created by this run is consumed by later run {} (change set {})",
                    input_id.as_str(),
                    cs.run_id,
                    cs.id.as_str()
                ));
            }
        }
    }

    Ok(impact)
}
