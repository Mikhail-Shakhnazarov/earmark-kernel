use earmark_core::{HeaderValue, Kind, Provenance, Standing};
use earmark_store::{
    BatchWrite, CanonicalStore, GitCanonicalStore, StoreError, StoredObject, StoredPayload,
};
use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_payload_integrity_verification() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("payload_integrity");
    let store = GitCanonicalStore::new(&root);

    let object = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("hello world"),
        vec![],
    );

    let version_ref = store.write_object(&object).unwrap();

    // Read should work
    let loaded = store.read_version(&version_ref).unwrap();
    assert_eq!(loaded.payload.as_utf8().unwrap(), "hello world");

    // Manually corrupt the payload file in the version directory
    let version_dir = store.version_path(&version_ref);
    let payload_file = fs::read_dir(&version_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("payload.")
        })
        .expect("Payload file should exist");

    // On Windows, gix/git or system indexers might still have a handle.
    // Retry for 5 seconds.
    let mut success = false;
    for _ in 0..50 {
        if let Ok(_) = fs::write(&payload_file, "corrupted world") {
            success = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        success,
        "Failed to corrupt payload file after 5 seconds due to locking"
    );

    // Read should now fail with PayloadIntegrityMismatch
    let result = store.read_version(&version_ref);
    match result {
        Err(StoreError::PayloadIntegrityMismatch { .. }) => {}
        other => panic!("Expected PayloadIntegrityMismatch, got {:?}", other),
    }

    // Scan should also fail
    let scan_result = store.scan_objects();
    match scan_result {
        Err(StoreError::PayloadIntegrityMismatch { .. }) => {}
        other => panic!("Expected PayloadIntegrityMismatch in scan, got {:?}", other),
    }
}

#[test]
fn test_rollback_cleanup_intermediate_dirs() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("rollback_cleanup");
    let store = GitCanonicalStore::new(&root);

    let ok_object = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("ok"),
        vec![],
    );

    let mut bad_object = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("bad"),
        vec![],
    );
    // Force failure via invalid payload ref in write_batch (which triggers rollback)
    bad_object.envelope.payload_ref = earmark_core::PayloadRef("sha256:invalid".to_string());

    let result = store.write_batch(&BatchWrite {
        message: "fail".to_string(),
        objects: vec![ok_object.clone(), bad_object],
    });
    assert!(result.is_err());

    // Check that intermediate directories are gone for ok_object
    let obj_id_str = ok_object.envelope.id.as_str();
    let obj_dir = root
        .join(".earmark")
        .join("canonical")
        .join("objects")
        .join(obj_id_str);
    assert!(
        !obj_dir.exists(),
        "Object directory should be cleaned up if empty during rollback"
    );
}

#[test]
fn test_write_serialization_locking() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("locking");
    let store = GitCanonicalStore::new(&root);

    // Acquire lock manually
    let _guard = store.acquire_write_lock().unwrap();

    let store_clone = store.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let _guard2 = store_clone.acquire_write_lock().unwrap();
        tx.send(start.elapsed()).unwrap();
    });

    std::thread::sleep(std::time::Duration::from_millis(200));
    drop(_guard);

    let elapsed = rx.recv().unwrap();
    assert!(
        elapsed >= std::time::Duration::from_millis(200),
        "Second lock acquisition should have been blocked"
    );
}

#[test]
fn test_lock_alone() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("lock_alone");
    let store = GitCanonicalStore::new(&root);
    let _guard = store.acquire_write_lock().unwrap();
}
