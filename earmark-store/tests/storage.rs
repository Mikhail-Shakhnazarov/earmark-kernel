use std::collections::BTreeMap;

use earmark_core::{HeaderValue, Kind, Provenance, Standing};
use earmark_store::{
    BatchWrite, CanonicalStore, GitCanonicalStore, ObjectStore, StoreScanner, StoreWriteLocking,
    StoredObject, StoredPayload, WorkspaceLayout,
};
use gix::bstr::ByteSlice;
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

#[test]
fn write_batch_rolls_back_on_mid_batch_failure() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let first = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("ok"),
        vec![],
    );
    let first_ref = first.envelope.version_ref();

    let mut second = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("bad"),
        vec![],
    );
    // Force deterministic write failure after first object writes.
    second.envelope.payload_ref = earmark_core::PayloadRef("sha256:deadbeef".to_string());

    let result = store.write_batch(&BatchWrite {
        message: "should fail".to_string(),
        objects: vec![first.clone(), second],
    });
    assert!(result.is_err());

    let first_version_dir = store.version_path(&first_ref);
    assert!(
        !first_version_dir.exists(),
        "first object artifacts should be rolled back on batch failure"
    );
    let head = store.read_head(&first.envelope.id).unwrap();
    assert!(head.is_none());
}

#[test]
fn write_batch_restores_overwritten_head_on_failure() {
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
    let first_ref = store.write_object(&first).unwrap();

    let second = StoredObject::with_parent(
        &first,
        Standing::default(),
        BTreeMap::new(),
        StoredPayload::from_markdown("v2"),
    );
    let second_ref = second.envelope.version_ref();

    let mut failing = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("bad"),
        vec![],
    );
    failing.envelope.payload_ref = earmark_core::PayloadRef("sha256:deadbeef".to_string());

    let result = store.write_batch(&BatchWrite {
        message: "overwrite then fail".to_string(),
        objects: vec![second, failing],
    });
    assert!(result.is_err());

    let head = store.read_head_ref(&first.envelope.id).unwrap().unwrap();
    assert_eq!(head.version_id, first_ref.version_id);
    assert_ne!(head.version_id, second_ref.version_id);

    let second_version_dir = store.version_path(&second_ref);
    assert!(
        !second_version_dir.exists(),
        "failed batch should remove second version artifacts"
    );
}

#[test]
fn commit_history_advances_and_records_message() {
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
    store
        .write_batch(&BatchWrite {
            message: "first commit".to_string(),
            objects: vec![first.clone()],
        })
        .unwrap();

    let second = StoredObject::with_parent(
        &first,
        Standing::default(),
        BTreeMap::new(),
        StoredPayload::from_markdown("v2"),
    );
    store
        .write_batch(&BatchWrite {
            message: "second commit".to_string(),
            objects: vec![second],
        })
        .unwrap();

    let repo = gix::open(store.canonical_dir()).unwrap();
    let head_id = repo.head_id().unwrap().detach();
    let head_commit = repo.find_commit(head_id).unwrap();
    assert_eq!(
        head_commit.message_raw_sloppy().to_string(),
        "second commit"
    );
    assert_eq!(head_commit.parent_ids().count(), 1);
}

#[test]
fn signature_precedence_env_over_config_then_fallback() {
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());

    let obj = || {
        StoredObject::new(
            Kind::Object,
            Some("note".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown("body"),
            vec![],
        )
    };

    store.init_layout().unwrap();
    std::fs::write(
        store.canonical_dir().join(".git/config"),
        "[user]\n\tname = cfg-name\n\temail = cfg@example.com\n",
    )
    .unwrap();

    unsafe {
        std::env::remove_var("EARMARK_GIT_NAME");
        std::env::remove_var("EARMARK_GIT_EMAIL");
    }
    store
        .write_batch(&BatchWrite {
            message: "cfg".into(),
            objects: vec![obj()],
        })
        .unwrap();
    let repo = gix::open(store.canonical_dir()).unwrap();
    let commit = repo.find_commit(repo.head_id().unwrap().detach()).unwrap();
    let sig = commit.author().unwrap();
    assert_eq!(sig.name.to_str_lossy().to_string(), "cfg-name");
    assert_eq!(sig.email.to_str_lossy().to_string(), "cfg@example.com");

    unsafe {
        std::env::set_var("EARMARK_GIT_NAME", "env-name");
        std::env::set_var("EARMARK_GIT_EMAIL", "env@example.com");
    }
    store
        .write_batch(&BatchWrite {
            message: "env".into(),
            objects: vec![obj()],
        })
        .unwrap();
    let repo = gix::open(store.canonical_dir()).unwrap();
    let commit = repo.find_commit(repo.head_id().unwrap().detach()).unwrap();
    let sig = commit.author().unwrap();
    assert_eq!(sig.name.to_str_lossy().to_string(), "env-name");
    assert_eq!(sig.email.to_str_lossy().to_string(), "env@example.com");

    unsafe {
        std::env::remove_var("EARMARK_GIT_NAME");
        std::env::remove_var("EARMARK_GIT_EMAIL");
    }
    std::fs::write(
        store.canonical_dir().join(".git/config"),
        "[user]\n\tname = \n\temail = \n",
    )
    .unwrap();
    store
        .write_batch(&BatchWrite {
            message: "fallback".into(),
            objects: vec![obj()],
        })
        .unwrap();
    let repo = gix::open(store.canonical_dir()).unwrap();
    let commit = repo.find_commit(repo.head_id().unwrap().detach()).unwrap();
    let sig = commit.author().unwrap();
    assert_eq!(sig.name.to_str_lossy().to_string(), "earmark");
    assert_eq!(sig.email.to_str_lossy().to_string(), "earmark@local");
}

#[test]
fn payload_utf8_accessors_support_borrowed_and_owned_views() {
    let payload = StoredPayload::from_markdown("hello");
    assert_eq!(payload.as_utf8_str().unwrap(), "hello");
    assert_eq!(payload.as_utf8().unwrap(), "hello".to_string());
}

#[test]
fn backend_contains_no_process_command_usage() {
    let src = include_str!("../src/backend.rs");
    assert!(!src.contains("std::process::Command"));
}
