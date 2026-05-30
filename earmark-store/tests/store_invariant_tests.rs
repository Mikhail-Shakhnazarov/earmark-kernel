use chrono::Utc;
use earmark_core::{ActorId, ObjectId, ObjectRecord, RelationId, RelationRecord, Standing};
use earmark_declarations::InProcessRegistry;
use earmark_index::sqlite_index::SqliteIndex;
use earmark_index::traits::DerivedIndex;
use earmark_store::file_store::FileStore;
use earmark_store::sanctioned::{deposit_object, DepositObjectInput};
use earmark_store::traits::CanonicalStore;
use tempfile::tempdir;

#[test]
fn test_store_consistency_audit() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store = FileStore::new(dir.path());
    store.init().unwrap();

    let registry = InProcessRegistry::new(); // Empty but valid for now

    // 1. Clean verify
    let violations = store.verify_consistency().unwrap();
    assert!(
        violations.is_empty(),
        "Clean store should have no violations"
    );

    // 2. Deposit something
    let actor = ActorId::generate();
    let obj_id = ObjectId::generate();
    let input = DepositObjectInput {
        id: Some(obj_id.clone()),
        class_id: None,
        payload: serde_json::json!({"test": "consistency"}),
        standing: Standing {
            dimensions: Vec::new(),
        },
        signal: None,
    };
    deposit_object(&store, &registry, actor.clone(), input).unwrap();

    let violations = store.verify_consistency().unwrap();
    assert!(
        violations.is_empty(),
        "Story with one object should be consistent"
    );

    // 3. Break version linkage
    let obj_path = dir.path().join(".earmark/objects").join(obj_id.as_str());
    let record_path = obj_path.join("record.json");
    let mut obj_record: ObjectRecord =
        serde_json::from_str(&std::fs::read_to_string(&record_path).unwrap()).unwrap();

    // Set to a non-existent version
    obj_record.latest_version_id = earmark_core::VersionId::parse("ver_broken").unwrap();
    std::fs::write(&record_path, serde_json::to_string(&obj_record).unwrap()).unwrap();

    let violations = store.verify_consistency().unwrap();
    assert!(!violations.is_empty(), "Should catch missing version");
    assert!(violations[0].contains("latest version ver_broken missing"));
}

#[test]
fn test_index_rebuild_from_store() {
    let dir = tempdir().expect("Failed to create temp dir");
    let store_dir = dir.path().join("store");
    let index_path = dir.path().join("index.sqlite");

    let store = FileStore::new(&store_dir);
    store.init().unwrap();

    let registry = InProcessRegistry::new();
    let actor = ActorId::generate();

    // Deposit an object
    let obj_id = ObjectId::generate();
    let input = DepositObjectInput {
        id: Some(obj_id.clone()),
        class_id: None,
        payload: serde_json::json!({"test": "index"}),
        standing: Standing {
            dimensions: Vec::new(),
        },
        signal: None,
    };
    deposit_object(&store, &registry, actor.clone(), input).unwrap();

    // Deposit a relation
    let target_id = ObjectId::generate();
    let input_target = DepositObjectInput {
        id: Some(target_id.clone()),
        class_id: None,
        payload: serde_json::json!({"test": "target"}),
        standing: Standing {
            dimensions: Vec::new(),
        },
        signal: None,
    };
    deposit_object(&store, &registry, actor.clone(), input_target).unwrap();

    store
        .create_relation(RelationRecord {
            id: RelationId::generate(),
            source_id: obj_id.clone(),
            target_id: target_id.clone(),
            relation_type: "test_rel".to_string(),
            created_at: Utc::now(),
            created_by: Some(actor.clone()),
        })
        .unwrap();

    // Initialize Index and Rebuild
    let mut index = SqliteIndex::open(&index_path).unwrap();
    index.rebuild_from_store(&store).unwrap();

    // Verify discovery via index
    let obj = index.get_object(&obj_id).unwrap();
    assert_eq!(obj.id, obj_id);

    let relations = index.find_relations_by_source(&obj_id).unwrap();
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].target_id, target_id);
}
