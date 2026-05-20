use chrono::Utc;
use earmark_core::{Kind, ObjectId, PayloadRef, VersionRef};
use earmark_index::{DerivedIndex, IndexDirtyMarker};
use earmark_store::{
    BatchWrite, CanonicalStore, GitCanonicalStore, ObjectStore as StoreObjectStore,
    StoreDiagnostics, StoreError, StoreScanner, StoreWriteLocking, StoredObject, StoredPayload,
    WorkspaceLayout as StoreWorkspaceLayout, WorkspaceWriteGuard,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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

    // Verify index now has it
    assert!(index.get_head(&object_id).unwrap().is_some());
    assert!(index.dirty_status().unwrap().is_none());
}

#[test]
fn test_rebuild_successful_clears_dirty_marker() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    let obj = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    store.write_object(&obj).unwrap();

    let report = index.rebuild_from_store(&store).unwrap();
    assert_eq!(report.indexed_objects, 1);
    assert!(report.skipped_entries.is_empty());
    assert!(index.dirty_status().unwrap().is_none());
}

#[test]
fn test_rebuild_skips_and_reports_corrupted_envelope() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    let obj = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let version_ref = store.write_object(&obj).unwrap();

    // Corrupt the envelope.json file
    let objects_dir = root.join(".earmark").join("canonical").join("objects");
    let env_path = objects_dir
        .join(version_ref.id.as_str())
        .join(version_ref.version_id.as_str())
        .join("envelope.json");
    std::fs::write(&env_path, b"{corrupted json").unwrap();

    let report = index.rebuild_from_store(&store).unwrap();
    assert_eq!(report.indexed_objects, 0);
    assert_eq!(report.skipped_entries.len(), 1);
    assert!(report.skipped_entries[0]
        .reason
        .contains("corrupted envelope JSON"));
}

#[test]
fn test_rebuild_skips_and_reports_missing_payload() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    let obj = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let version_ref = store.write_object(&obj).unwrap();

    // Remove the payload file from version dir
    let objects_dir = root.join(".earmark").join("canonical").join("objects");
    let payload_path = objects_dir
        .join(version_ref.id.as_str())
        .join(version_ref.version_id.as_str())
        .join("payload.json");
    std::fs::remove_file(&payload_path).unwrap();

    let report = index.rebuild_from_store(&store).unwrap();
    assert_eq!(report.indexed_objects, 0);
    assert_eq!(report.skipped_entries.len(), 1);
    assert!(report.skipped_entries[0]
        .reason
        .contains("missing payload file"));
}

#[test]
fn test_rebuild_skips_and_reports_payload_ref_mismatch() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    let obj = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let version_ref = store.write_object(&obj).unwrap();

    // Modify payload content directly in payload file, which makes envelope.payload_ref mismatched
    let objects_dir = root.join(".earmark").join("canonical").join("objects");
    let payload_path = objects_dir
        .join(version_ref.id.as_str())
        .join(version_ref.version_id.as_str())
        .join("payload.json");
    std::fs::write(&payload_path, b"{\"changed\": true}").unwrap();

    let report = index.rebuild_from_store(&store).unwrap();
    assert_eq!(report.indexed_objects, 0);
    assert_eq!(report.skipped_entries.len(), 1);
    assert!(report.skipped_entries[0]
        .reason
        .contains("integrity mismatch"));
}

struct FailingStore {
    inner: GitCanonicalStore,
    should_fail: std::sync::atomic::AtomicBool,
}

impl StoreWorkspaceLayout for FailingStore {
    fn root(&self) -> &Path {
        self.inner.root()
    }
    fn init_layout(&self) -> Result<(), StoreError> {
        self.inner.init_layout()
    }
    fn version_path(&self, version: &VersionRef) -> PathBuf {
        self.inner.version_path(version)
    }
}

impl StoreObjectStore for FailingStore {
    fn write_object(&self, object: &StoredObject) -> Result<VersionRef, StoreError> {
        self.inner.write_object(object)
    }
    fn write_batch(&self, batch: &BatchWrite) -> Result<Vec<VersionRef>, StoreError> {
        self.inner.write_batch(batch)
    }
    fn read_version(&self, version: &VersionRef) -> Result<StoredObject, StoreError> {
        self.inner.read_version(version)
    }
    fn read_head(&self, object_id: &ObjectId) -> Result<Option<StoredObject>, StoreError> {
        self.inner.read_head(object_id)
    }
    fn read_head_ref(&self, object_id: &ObjectId) -> Result<Option<VersionRef>, StoreError> {
        self.inner.read_head_ref(object_id)
    }
    fn list_versions(&self, object_id: &ObjectId) -> Result<Vec<VersionRef>, StoreError> {
        self.inner.list_versions(object_id)
    }
    fn resolve_payload(&self, payload_ref: &PayloadRef) -> Result<StoredPayload, StoreError> {
        self.inner.resolve_payload(payload_ref)
    }
}

impl StoreScanner for FailingStore {
    fn scan_objects(&self) -> Result<StoreDiagnostics, StoreError> {
        if self.should_fail.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(StoreError::Io(std::io::Error::other(
                "injected scan failure",
            )));
        }
        self.inner.scan_objects()
    }
}

impl StoreWriteLocking for FailingStore {
    fn acquire_write_lock(&self) -> Result<WorkspaceWriteGuard, StoreError> {
        self.inner.acquire_write_lock()
    }
    fn write_batch_locked(
        &self,
        guard: &WorkspaceWriteGuard,
        batch: &BatchWrite,
    ) -> Result<Vec<VersionRef>, StoreError> {
        self.inner.write_batch_locked(guard, batch)
    }
}

impl CanonicalStore for FailingStore {}

#[test]
fn test_rebuild_transactional_safety_on_injected_failure() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let inner_store = GitCanonicalStore::new(root);
    inner_store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Write an initial valid object
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
    inner_store.write_object(&obj).unwrap();

    // Rebuild so index has this object
    index.rebuild_from_store(&inner_store).unwrap();
    assert!(index.get_head(&object_id).unwrap().is_some());

    // 2. Wrap store with a failing one
    let store = FailingStore {
        inner: inner_store,
        should_fail: std::sync::atomic::AtomicBool::new(true),
    };

    // Rebuild should fail due to injected scan error
    let res = index.rebuild_from_store(&store);
    assert!(res.is_err());

    // Verify index still has the object and was NOT cleared (transaction rolled back)
    assert!(index.get_head(&object_id).unwrap().is_some());
}

#[test]
fn test_rebuild_relation_rows_projected_only_for_valid_relations() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Write a valid Relation
    let source_ref = earmark_core::ObjectRef::new(
        ObjectId::new(),
        earmark_core::VersionId::new(),
        Kind::Object,
        None,
    );
    let target_ref = earmark_core::ObjectRef::new(
        ObjectId::new(),
        earmark_core::VersionId::new(),
        Kind::Object,
        None,
    );
    let payload = earmark_core::RelationPayload {
        source: source_ref,
        target: target_ref,
        relation_type: "test_relation".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };
    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let obj = StoredObject::new(
        Kind::Relation,
        None,
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(payload_bytes),
        vec![],
    );
    store.write_object(&obj).unwrap();

    // 2. Write a normal object (non-relation)
    let obj2 = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    store.write_object(&obj2).unwrap();

    // Rebuild
    index.rebuild_from_store(&store).unwrap();

    // Check count of relations
    assert_eq!(index.relation_count().unwrap(), 1);
}
