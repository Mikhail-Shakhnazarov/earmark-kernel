use crate::error::ExecError;
use earmark_core::{
    HeaderValue, Kind, ObjectRef, Provenance, RelationCreationMode, RelationPayload, Standing,
};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;

/// Persists a relation to the canonical store and updates the index.
/// This is the shared canonical path for all relation creation in the workspace.
pub fn persist_relation_canonical<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    payload: RelationPayload,
    provenance: Provenance,
    mode: RelationCreationMode,
    additional_headers: Option<BTreeMap<String, HeaderValue>>,
) -> Result<ObjectRef, ExecError> {
    if mode == RelationCreationMode::PrivilegedSystem
        && !earmark_core::is_privileged_relation(&payload.relation_type)
    {
        return Err(ExecError::InvalidRelationMode(format!(
            "relation type '{}' is not a privileged system relation",
            payload.relation_type
        )));
    }

    if mode == RelationCreationMode::Declared
        && earmark_core::is_privileged_relation(&payload.relation_type)
    {
        return Err(ExecError::InvalidRelationMode(format!(
            "relation type '{}' is a privileged system relation and cannot be created in 'declared' mode",
            payload.relation_type
        )));
    }

    let mut headers = additional_headers.unwrap_or_default();

    // Attach creation mode header
    headers.insert(
        "relation_creation_mode".to_string(),
        HeaderValue::String(match mode {
            RelationCreationMode::Declared => "declared".to_string(),
            RelationCreationMode::PrivilegedSystem => "privileged_system".to_string(),
        }),
    );

    let stored = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        provenance,
        headers,
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&payload)?),
        vec![],
    );

    use crate::persistence_helpers::write_object_and_index;
    let version_ref = write_object_and_index(store, index, &stored)?;

    Ok(ObjectRef::new(
        version_ref.id,
        version_ref.version_id,
        Kind::Relation,
        None,
    ))
}
