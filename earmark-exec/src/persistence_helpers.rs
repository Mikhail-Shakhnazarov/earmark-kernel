use crate::error::ExecError;
use chrono::Utc;
use earmark_core::VersionRef;
use earmark_index::{DerivedIndex, IndexDirtyMarker};
use earmark_store::{BatchWrite, CanonicalStore, StoredObject};

pub fn write_object_and_index<S: CanonicalStore>(
    store: &S,
    index: &mut DerivedIndex,
    object: &StoredObject,
) -> Result<VersionRef, ExecError> {
    let marker = IndexDirtyMarker {
        schema_version: "1.0".to_string(),
        reason: "writing object".to_string(),
        timestamp: Utc::now(),
        operation: "write_object".to_string(),
        object_ids: vec![object.envelope.id.as_str().to_string()],
        version_ids: vec![object.envelope.version_id.as_str().to_string()],
    };
    index.mark_dirty(marker)?;

    let guard = store.acquire_write_lock().map_err(|e| {
        let _ = index.clear_dirty();
        ExecError::Store(e)
    })?;
    let version_ref = store.write_object_locked(&guard, object).map_err(|e| {
        let _ = index.clear_dirty();
        ExecError::Store(e)
    })?;
    if let Err(e) = index.upsert_head_object_from_store(store, &object.envelope.id) {
        return Err(ExecError::IndexUpdateFailure {
            object_id: object.envelope.id.as_str().to_string(),
            version_id: object.envelope.version_id.as_str().to_string(),
            error: e.to_string(),
        });
    }

    index.clear_dirty()?;
    Ok(version_ref)
}

/// Batch-write objects to the store and update the derived index for each.
///
/// This is part of the execution persistence API: batch writes ensure store/index
/// coherence across multiple object writes in a single lock acquisition. It remains
/// public because integration tests and external consumers legitimately need
/// transactional batch-write semantics when setting up or extending store content.
///
/// If a future refactor consolidates index updates into the store write path,
/// this function may become an internal detail.
pub fn write_batch_and_index<S: CanonicalStore>(
    store: &S,
    index: &mut DerivedIndex,
    batch: &BatchWrite,
) -> Result<Vec<VersionRef>, ExecError> {
    let marker = IndexDirtyMarker {
        schema_version: "1.0".to_string(),
        reason: "writing batch".to_string(),
        timestamp: Utc::now(),
        operation: "write_batch".to_string(),
        object_ids: batch
            .objects
            .iter()
            .map(|o| o.envelope.id.as_str().to_string())
            .collect(),
        version_ids: batch
            .objects
            .iter()
            .map(|o| o.envelope.version_id.as_str().to_string())
            .collect(),
    };
    index.mark_dirty(marker)?;

    let guard = store.acquire_write_lock().map_err(|e| {
        let _ = index.clear_dirty();
        ExecError::Store(e)
    })?;
    let version_refs = store.write_batch_locked(&guard, batch).map_err(|e| {
        let _ = index.clear_dirty();
        ExecError::Store(e)
    })?;
    for object in &batch.objects {
        if let Err(e) = index.upsert_head_object_from_store(store, &object.envelope.id) {
            return Err(ExecError::IndexUpdateFailure {
                object_id: object.envelope.id.as_str().to_string(),
                version_id: object.envelope.version_id.as_str().to_string(),
                error: e.to_string(),
            });
        }
    }

    index.clear_dirty()?;
    Ok(version_refs)
}
