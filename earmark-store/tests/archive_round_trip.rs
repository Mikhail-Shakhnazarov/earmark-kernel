/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use earmark_core::*;
use earmark_store::file_store::FileStore;
use earmark_store::archive::{export_workspace, import_workspace};
use earmark_store::traits::CanonicalStore;
use tempfile::tempdir;
use chrono::Utc;

#[test]
fn test_archive_round_trip_v1_integrity() {
    let source_dir = tempdir().unwrap();
    let source_store = FileStore::new(source_dir.path());
    
    // 1. Populate source store
    let obj_id = ObjectId::generate();
    let ver_id = VersionId::generate();
    let obj = ObjectRecord {
        id: obj_id.clone(),
        class_id: Some(ClassId::parse("cls_test").unwrap()),
        latest_version_id: ver_id.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let ver = VersionRecord {
        version_id: ver_id,
        object_id: obj_id.clone(),
        payload: serde_json::json!({"test": "data"}),
        standing: Standing { dimensions: vec![] },
        signal: None,
        created_at: Utc::now(),
        created_by: None,
    };
    source_store.deposit_object(obj, ver).unwrap();
    
    let run_id = RunId::generate();
    let run = RunRecord {
        run_id: run_id.clone(),
        workflow_id: None,
        status: RunStatus::Scheduled,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    source_store.create_run(run).unwrap();
    
    // 2. Export
    let archive = export_workspace(&source_store, false).unwrap();
    
    // 3. Import to target store
    let target_dir = tempdir().unwrap();
    let target_store = FileStore::new(target_dir.path());
    target_store.init().unwrap(); // Initialize dirs
    
    import_workspace(&target_store, archive, true).unwrap();
    
    // 4. Verify integrity
    let fetched_obj = target_store.get_object(&obj_id).unwrap();
    assert_eq!(fetched_obj.id, obj_id);
    
    let fetched_run = target_store.get_run(&run_id).unwrap();
    assert_eq!(fetched_run.run_id, run_id);
    
    assert_eq!(target_store.list_objects().unwrap().len(), 1);
    assert_eq!(target_store.list_runs().unwrap().len(), 1);
}
