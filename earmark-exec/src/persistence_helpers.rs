use crate::error::ExecError;
use earmark_core::VersionRef;
use earmark_index::DerivedIndex;
use earmark_store::{BatchWrite, CanonicalStore, StoredObject};

pub fn write_object_and_index<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    object: &StoredObject,
) -> Result<VersionRef, ExecError> {
    let guard = store.acquire_write_lock()?;
    let version_ref = store.write_object_locked(&guard, object)?;
    if let Err(e) = index.upsert_head_object_from_store(store, &object.envelope.id) {
        return Err(ExecError::IndexUpdateFailure {
            object_id: object.envelope.id.as_str().to_string(),
            version_id: object.envelope.version_id.as_str().to_string(),
            error: e.to_string(),
        });
    }
    Ok(version_ref)
}

pub fn write_batch_and_index<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    batch: &BatchWrite,
) -> Result<Vec<VersionRef>, ExecError> {
    let guard = store.acquire_write_lock()?;
    let version_refs = store.write_batch_locked(&guard, batch)?;
    for object in &batch.objects {
        if let Err(e) = index.upsert_head_object_from_store(store, &object.envelope.id) {
            return Err(ExecError::IndexUpdateFailure {
                object_id: object.envelope.id.as_str().to_string(),
                version_id: object.envelope.version_id.as_str().to_string(),
                error: e.to_string(),
            });
        }
    }
    Ok(version_refs)
}
