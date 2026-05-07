use std::collections::BTreeMap;

use earmark_core::{HeaderValue, Kind, Provenance, Standing};
use earmark_store::{BatchWrite, CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use tempfile::tempdir;

#[test]
fn write_and_read_by_version() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    let mut headers = BTreeMap::new();
    headers.insert(
        "title".to_string(),
        HeaderValue::String("First note".to_string()),
    );
    let object = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        headers,
        StoredPayload::from_markdown("hello world"),
        vec![],
    );

    let version_ref = store.write_object(&object).unwrap();
    let loaded = store.read_version(&version_ref).unwrap();
    assert_eq!(loaded.envelope.id, object.envelope.id);
    assert_eq!(loaded.payload.as_utf8().unwrap(), "hello world");
}

#[test]
fn advance_head_with_new_version() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    let mut headers = BTreeMap::new();
    headers.insert(
        "title".to_string(),
        HeaderValue::String("First".to_string()),
    );

    let first = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        headers.clone(),
        StoredPayload::from_markdown("v1"),
        vec![],
    );
    store.write_object(&first).unwrap();

    headers.insert(
        "title".to_string(),
        HeaderValue::String("Second".to_string()),
    );
    let second = StoredObject::with_parent(
        &first,
        Standing::default(),
        headers,
        StoredPayload::from_markdown("v2"),
    );
    store.write_object(&second).unwrap();

    let head = store.read_head(&first.envelope.id).unwrap().unwrap();
    assert_eq!(head.envelope.version_id, second.envelope.version_id);
    assert_eq!(head.payload.as_utf8().unwrap(), "v2");
}

#[test]
fn list_versions_for_object() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let first = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("v1"),
        vec![],
    );
    store.write_object(&first).unwrap();

    let second = StoredObject::with_parent(
        &first,
        Standing::default(),
        BTreeMap::new(),
        StoredPayload::from_markdown("v2"),
    );
    store.write_object(&second).unwrap();

    let versions = store.list_versions(&first.envelope.id).unwrap();
    assert_eq!(versions.len(), 2);
}

#[test]
fn relation_object_storage_and_retrieval() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let source = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("source"),
        vec![],
    );
    let target = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("target"),
        vec![],
    );
    let refs = store
        .write_batch(&BatchWrite {
            message: "seed".to_string(),
            objects: vec![source.clone(), target.clone()],
        })
        .unwrap();

    let relation_json = serde_json::json!({
        "source": source.object_ref(),
        "target": target.object_ref(),
        "relation_type": "supports",
        "qualifiers": {},
        "scope": "test"
    });
    let relation = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&relation_json).unwrap()),
        refs,
    );
    let relation_ref = store.write_object(&relation).unwrap();
    let loaded = store.read_version(&relation_ref).unwrap();
    assert_eq!(loaded.envelope.kind, Kind::Relation);
}
