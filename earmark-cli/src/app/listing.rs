use crate::app::common::CliError;
use earmark_core::{Kind, ProviderRecord};
use earmark_store::CanonicalStore;
use serde_json::json;

pub(crate) fn list_run_records<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::RunRecord>, CliError> {
    let mut ledgers = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::RunRecord {
            continue;
        }
        let ledger: earmark_core::RunRecord = serde_json::from_slice(&object.payload.bytes)?;
        ledgers.push(ledger);
    }
    ledgers.sort_by(|a, b| {
        a.started_at
            .cmp(&b.started_at)
            .then_with(|| a.run_id.cmp(&b.run_id))
    });
    Ok(ledgers)
}

pub(crate) fn load_run_record_by_id<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<earmark_core::RunRecord, CliError> {
    let ledgers = list_run_records(store)?;
    if run_id == "latest" {
        return ledgers
            .last()
            .cloned()
            .ok_or_else(|| CliError::not_found("no runs found".to_string()));
    }
    for ledger in ledgers {
        if ledger.run_id == run_id {
            return Ok(ledger);
        }
    }
    Err(CliError::not_found(format!("run not found: {}", run_id)))
}

pub(crate) fn list_assignments<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::TransitionAssignment>, CliError> {
    let mut assignments = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransitionAssignment {
            continue;
        }
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id != object.envelope.version_id {
                continue;
            }
        }
        let assignment: earmark_core::TransitionAssignment =
            serde_json::from_slice(&object.payload.bytes)?;
        assignments.push(assignment);
    }
    Ok(assignments)
}

pub(crate) fn list_assignments_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<earmark_core::TransitionAssignment>, CliError> {
    Ok(list_assignments(store)?
        .into_iter()
        .filter(|assignment| assignment.run_id == run_id)
        .collect())
}

pub(crate) fn list_change_sets<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::ChangeSet>, CliError> {
    let mut change_sets = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::ChangeSet {
            continue;
        }
        let change_set: earmark_core::ChangeSet = serde_json::from_slice(&object.payload.bytes)?;
        change_sets.push(change_set);
    }
    Ok(change_sets)
}

pub(crate) fn list_change_sets_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<earmark_core::ChangeSet>, CliError> {
    Ok(list_change_sets(store)?
        .into_iter()
        .filter(|change_set| change_set.run_id == run_id)
        .collect())
}

pub(crate) fn list_handoffs<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<earmark_core::HandoffManifest>, CliError> {
    let mut handoffs = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::HandoffManifest {
            continue;
        }
        let handoff: earmark_core::HandoffManifest = serde_json::from_slice(&object.payload.bytes)?;
        handoffs.push(handoff);
    }
    Ok(handoffs)
}

pub(crate) fn list_handoffs_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<earmark_core::HandoffManifest>, CliError> {
    Ok(list_handoffs(store)?
        .into_iter()
        .filter(|handoff| handoff.run_id == run_id)
        .collect())
}

pub(crate) fn list_failure_objects<S: CanonicalStore>(
    store: &S,
) -> Result<Vec<(String, earmark_core::TransformationFailure)>, CliError> {
    let mut failures = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransformationFailure {
            continue;
        }
        let failure: earmark_core::TransformationFailure =
            serde_json::from_slice(&object.payload.bytes)?;
        failures.push((object.envelope.id.as_str().to_string(), failure));
    }
    Ok(failures)
}

pub(crate) fn list_failures_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<String>, CliError> {
    Ok(list_failure_objects(store)?
        .into_iter()
        .filter(|(_, failure)| failure.run_id == run_id)
        .map(|(id, _)| id)
        .collect())
}

pub(crate) fn list_failures<S: CanonicalStore>(
    store: &S,
    run_id: Option<&str>,
    transition_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, CliError> {
    let mut failures = Vec::new();
    for (failure_id, failure) in list_failure_objects(store)? {
        if let Some(run_id) = run_id {
            if failure.run_id != run_id {
                continue;
            }
        }
        if let Some(transition_id) = transition_id {
            if failure.transition_id != transition_id {
                continue;
            }
        }
        failures.push(json!({
            "failure_id": failure_id,
            "run_id": failure.run_id,
            "transition_id": failure.transition_id,
            "assignment_id": failure.assignment_id.as_str(),
            "error_type": failure.error_type,
            "message": failure.message,
            "created_at": failure.created_at,
        }));
    }
    Ok(failures)
}

pub(crate) fn run_related_artifacts<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<serde_json::Value, CliError> {
    let assignments = list_assignments_by_run(store, run_id)?
        .into_iter()
        .map(|assignment| assignment.id.as_str().to_string())
        .collect::<Vec<_>>();
    let change_sets_full = list_change_sets_by_run(store, run_id)?;
    let change_sets = change_sets_full
        .iter()
        .map(|change_set| change_set.id.as_str().to_string())
        .collect::<Vec<_>>();
    let mut synthetic_change_sets = Vec::new();
    for change_set in &change_sets_full {
        let (synthetic, synthetic_source) =
            crate::app::loaders::change_set_synthetic_marker(store, change_set)?;
        if synthetic {
            synthetic_change_sets.push(json!({
                "change_set_id": change_set.id.as_str(),
                "synthetic_source": synthetic_source,
            }));
        }
    }
    let handoffs = list_handoffs_by_run(store, run_id)?
        .into_iter()
        .map(|handoff| handoff.id.as_str().to_string())
        .collect::<Vec<_>>();
    let failures = list_failures_by_run(store, run_id)?
        .into_iter()
        .collect::<Vec<_>>();
    Ok(json!({
        "assignments": assignments,
        "change_sets": change_sets,
        "synthetic_change_sets": synthetic_change_sets,
        "handoffs": handoffs,
        "failures": failures,
    }))
}

pub(crate) fn list_provider_records_by_run<S: CanonicalStore>(
    store: &S,
    run_id: &str,
) -> Result<Vec<ProviderRecord>, CliError> {
    let mut records = Vec::new();
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::Event {
            continue;
        }
        if object.envelope.class.as_deref() != Some("provider_record") {
            continue;
        }
        let record: ProviderRecord = serde_json::from_slice(&object.payload.bytes)?;
        if record.run_id == run_id {
            records.push(record);
        }
    }
    records.sort_by(|a, b| a.recorded_at.cmp(&b.recorded_at));
    Ok(records)
}
