/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use earmark_core::*;
use proptest::prelude::*;
use std::str::FromStr;

proptest! {
    #[test]
    fn test_object_id_round_trip(s in "[a-zA-Z0-9_-]+") {
        let id_str = format!("obj_{}", s);
        if let Ok(id) = ObjectId::from_str(&id_str) {
            prop_assert_eq!(id.to_string(), id_str);
        }
    }

    #[test]
    fn test_run_id_round_trip(s in "[a-zA-Z0-9_-]+") {
        let id_str = format!("run_{}", s);
        if let Ok(id) = RunId::from_str(&id_str) {
            prop_assert_eq!(id.to_string(), id_str);
        }
    }

    #[test]
    fn test_packet_id_round_trip(s in "[a-zA-Z0-9_-]+") {
        let id_str = format!("pkt_{}", s);
        if let Ok(id) = PacketId::from_str(&id_str) {
            prop_assert_eq!(id.to_string(), id_str);
        }
    }

    #[test]
    fn test_class_id_round_trip(s in "[a-zA-Z0-9_-]+") {
        let id_str = format!("cls_{}", s);
        if let Ok(id) = ClassId::from_str(&id_str) {
            prop_assert_eq!(id.to_string(), id_str);
        }
    }

    #[test]
    fn test_actor_id_round_trip(s in "[a-zA-Z0-9_-]+") {
        let id_str = format!("act_{}", s);
        if let Ok(id) = ActorId::from_str(&id_str) {
            prop_assert_eq!(id.to_string(), id_str);
        }
    }

    #[test]
    fn test_arbitrary_string_does_not_panic(s in "\\PC*") {
        let _ = ObjectId::from_str(&s);
        let _ = RunId::from_str(&s);
        let _ = PacketId::from_str(&s);
        let _ = ClassId::from_str(&s);
        let _ = ActorId::from_str(&s);
    }
}

#[cfg(test)]
mod store_invariants {
    use super::*;
    use earmark_store::file_store::FileStore;
    use earmark_store::traits::CanonicalStore;
    use tempfile::tempdir;

    #[test]
    fn test_file_store_directory_bootstrap() {
        let dir = tempdir().unwrap();
        let store = FileStore::new(dir.path());
        
        let obj_id = ObjectId::generate();
        let ver_id = VersionId::generate();
        let obj = ObjectRecord {
            id: obj_id.clone(),
            class_id: Some(ClassId::parse("cls_test").unwrap()),
            latest_version_id: ver_id.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let ver = VersionRecord {
            version_id: ver_id,
            object_id: obj_id.clone(),
            payload: serde_json::Value::Null,
            standing: earmark_core::Standing { dimensions: vec![] },
            signal: None,
            created_at: chrono::Utc::now(),
            created_by: None,
        };
        
        store.deposit_object(obj.clone(), ver).unwrap();
        
        assert!(dir.path().join(".earmark/objects").exists());
        assert!(dir.path().join(".earmark/objects").join(obj.id.as_str()).join("record.json").exists());
    }

    #[test]
    fn test_file_store_record_round_trip() {
        let dir = tempdir().unwrap();
        let store = FileStore::new(dir.path());
        
        let run_id = RunId::generate();
        let run = RunRecord {
            run_id: run_id.clone(),
            workflow_id: None,
            status: earmark_core::RunStatus::Scheduled,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        store.create_run(run.clone()).unwrap();
        let fetched = store.get_run(&run_id).unwrap();
        
        assert_eq!(fetched.run_id, run.run_id);
        assert_eq!(fetched.status, run.status);
    }
}
