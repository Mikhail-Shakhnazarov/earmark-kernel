use crate::app::common::CliError;
use earmark_core::{HeaderValue, Kind};
use earmark_store::{CanonicalStore, StoredObject};

pub(crate) fn load_current_assignment_by_id<S: CanonicalStore>(
    store: &S,
    assignment_id: &str,
) -> Result<earmark_core::TransitionAssignment, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransitionAssignment {
            continue;
        }
        let assignment: earmark_core::TransitionAssignment =
            serde_json::from_slice(&object.payload.bytes)?;
        if assignment.id.as_str() != assignment_id {
            continue;
        }
        if let Some(head_ref) = store.read_head_ref(&object.envelope.id)? {
            if head_ref.version_id == object.envelope.version_id {
                return Ok(assignment);
            }
        }
    }
    Err(CliError::not_found(format!(
        "assignment not found: {}",
        assignment_id
    )))
}

pub(crate) fn load_change_set_by_id<S: CanonicalStore>(
    store: &S,
    change_set_id: &str,
) -> Result<earmark_core::ChangeSet, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::ChangeSet {
            continue;
        }
        let change_set: earmark_core::ChangeSet = serde_json::from_slice(&object.payload.bytes)?;
        if change_set.id.as_str() == change_set_id {
            return Ok(change_set);
        }
    }
    Err(CliError::not_found(format!(
        "change set not found: {}",
        change_set_id
    )))
}

pub(crate) fn change_set_synthetic_marker<S: CanonicalStore>(
    store: &S,
    change_set: &earmark_core::ChangeSet,
) -> Result<(bool, Option<String>), CliError> {
    for object_id in &change_set.created_object_ids {
        let Some(stored) = store.read_head(object_id)? else {
            continue;
        };
        let synthetic = matches!(
            stored.envelope.headers.get("synthetic"),
            Some(HeaderValue::Bool(true))
        );
        if !synthetic {
            continue;
        }
        let source = match stored.envelope.headers.get("synthetic_source") {
            Some(HeaderValue::String(value)) => Some(value.clone()),
            _ => None,
        };
        return Ok((true, source));
    }
    Ok((false, None))
}

pub(crate) fn load_handoff_by_id<S: CanonicalStore>(
    store: &S,
    handoff_id: &str,
) -> Result<earmark_core::HandoffManifest, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::HandoffManifest {
            continue;
        }
        let handoff: earmark_core::HandoffManifest = serde_json::from_slice(&object.payload.bytes)?;
        if handoff.id.as_str() == handoff_id {
            return Ok(handoff);
        }
    }
    Err(CliError::not_found(format!(
        "handoff not found: {}",
        handoff_id
    )))
}

pub(crate) fn load_failure_by_id<S: CanonicalStore>(
    store: &S,
    failure_id: &str,
) -> Result<earmark_core::TransformationFailure, CliError> {
    for object in store.scan_objects()?.scanned_objects {
        if object.envelope.kind != Kind::TransformationFailure {
            continue;
        }
        if object.envelope.id.as_str() == failure_id {
            let failure: earmark_core::TransformationFailure =
                serde_json::from_slice(&object.payload.bytes)?;
            return Ok(failure);
        }
    }
    Err(CliError::not_found(format!(
        "failure not found: {}",
        failure_id
    )))
}

pub(crate) fn load_relation_object_by_id<S: CanonicalStore>(
    store: &S,
    relation_id: &str,
) -> Result<StoredObject, CliError> {
    let id = earmark_core::ObjectId::parse(relation_id)
        .map_err(|_| CliError::argument(format!("invalid relation ID: {}", relation_id)))?;
    let found = store
        .read_head(&id)?
        .ok_or_else(|| CliError::not_found(format!("relation not found: {}", relation_id)))?;
    if found.envelope.kind != Kind::Relation {
        return Err(CliError::argument(format!(
            "object {} is not a relation",
            relation_id
        )));
    }
    Ok(found)
}
