use chrono::Utc;
use earmark_core::Kind;
use earmark_index::{DerivedIndex, IndexDirtyMarker};
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_index_dirty_marker_lifecycle() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // Initial state: not dirty
    assert!(index.dirty_status().unwrap().is_none());

    // Mark dirty
    let marker = IndexDirtyMarker {
        schema_version: "1.0".to_string(),
        reason: "test".to_string(),
        timestamp: Utc::now(),
        operation: "test_op".to_string(),
        object_ids: vec!["obj1".to_string()],
        version_ids: vec!["ver1".to_string()],
    };
    index.mark_dirty(marker.clone()).unwrap();

    // Check dirty status
    let status = index.dirty_status().unwrap().unwrap();
    assert_eq!(status.reason, "test");
    assert_eq!(status.operation, "test_op");
    assert_eq!(status.object_ids, vec!["obj1".to_string()]);

    // Clear dirty
    index.clear_dirty().unwrap();
    assert!(index.dirty_status().unwrap().is_none());
}

#[test]
fn test_index_repair_recovery() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // Write an object to store only (simulating index failure)
    let obj = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let object_id = obj.envelope.id.clone();
    let version_ref = store.write_object(&obj).unwrap();

    // Mark dirty
    let marker = IndexDirtyMarker {
        schema_version: "1.0".to_string(),
        reason: "manual simulated failure".to_string(),
        timestamp: Utc::now(),
        operation: "write".to_string(),
        object_ids: vec![object_id.as_str().to_string()],
        version_ids: vec![version_ref.version_id.as_str().to_string()],
    };
    index.mark_dirty(marker).unwrap();

    // Verify index doesn't have it yet
    assert!(index.get_head(&object_id).unwrap().is_none());

    // Repair
    index.rebuild_from_store(&store).unwrap();
    index.clear_dirty().unwrap();

    // Verify index now has it
    assert!(index.get_head(&object_id).unwrap().is_some());
    assert!(index.dirty_status().unwrap().is_none());
}
