use earmark_core::{Kind, Provenance, Standing};
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_git_index_restoration_on_failure() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();

    // 1. Success write
    let obj1 = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        Default::default(),
        StoredPayload::from_markdown("hello".to_string()),
        vec![],
    );
    store.write_object(&obj1).unwrap();

    // Verify it's there
    let head1 = store.read_head(&obj1.envelope.id).unwrap().unwrap();
    assert_eq!(head1.envelope.id, obj1.envelope.id);

    // 2. Force failure by creating a directory where a file should be, 
    // or making a file unreadable in the corpus/objects path.
    
    // We'll create a file in the canonical dir that we then make unreadable.
    // Wait, GixBackend walks .earmark and corpus.
    let malicious_path = dir.path().join("corpus").join("malicious.md");
    fs::create_dir_all(malicious_path.parent().unwrap()).unwrap();
    fs::write(&malicious_path, "can't read me").unwrap();
    
    // On Windows, making it unreadable is tricky with just fs::set_permissions.
    // We'll try to use a directory with the same name as a file GixBackend expects to write.
    
    let obj2 = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        Default::default(),
        StoredPayload::from_markdown("fail me".to_string()),
        vec![],
    );
    
    // We'll make the version path a directory to force failure during fs::write or during gix walk.
    // Force failure by locking the git index
    let lock_path = dir.path().join(".git").join("index.lock");
    let _lock_file = fs::File::create(&lock_path).unwrap();

    println!("Attempting write while git index is locked");
    let result = store.write_object(&obj2);
    if let Err(ref e) = result {
        println!("Got expected error: {}", e);
    } else {
        println!("Unexpected success!");
    }
    assert!(result.is_err());

    // 3. Verify store is still consistent (obj1 is still there, obj2 is NOT there)
    // First, remove the lock so we can read properly if needed (though read_head doesn't need it)
    drop(_lock_file);
    let _ = fs::remove_file(&lock_path);

    let head_check = store.read_head(&obj1.envelope.id).unwrap().unwrap();
    assert_eq!(head_check.envelope.id, obj1.envelope.id);
    
    let version_path2 = store.version_path(&obj2.envelope.version_ref());
    assert!(!version_path2.exists(), "Version file should have been cleaned up");
    
    let head2_check = store.read_head(&obj2.envelope.id).unwrap();
    assert!(head2_check.is_none(), "Head file should have been cleaned up");

    // 4. Verify we can write again successfully
    let obj3 = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        Default::default(),
        StoredPayload::from_markdown("i am third".to_string()),
        vec![],
    );
    store.write_object(&obj3).expect("Should be able to write again after failure");
    let head3 = store.read_head(&obj3.envelope.id).unwrap().expect("Obj3 should be there");
    assert_eq!(head3.envelope.id, obj3.envelope.id);
}
