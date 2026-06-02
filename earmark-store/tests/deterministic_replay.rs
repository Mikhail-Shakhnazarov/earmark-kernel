/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use earmark_core::*;
use earmark_store::file_store::FileStore;
use earmark_store::traits::CanonicalStore;
use tempfile::tempdir;
use chrono::Utc;

#[test]
fn test_deterministic_object_versioning() {
    let dir = tempdir().unwrap();
    let store = FileStore::new(dir.path());
    
    let obj_id = ObjectId::generate();
    let v1_id = VersionId::generate();
    
    // 1. Initial Deposit
    let obj = ObjectRecord {
        id: obj_id.clone(),
        class_id: Some(ClassId::parse("cls_test").unwrap()),
        latest_version_id: v1_id.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let v1 = VersionRecord {
        version_id: v1_id.clone(),
        object_id: obj_id.clone(),
        payload: serde_json::json!({"ver": 1}),
        standing: Standing { dimensions: vec![] },
        signal: None,
        created_at: Utc::now(),
        created_by: None,
    };
    
    store.deposit_object(obj.clone(), v1.clone()).unwrap();
    
    // 2. New Version
    let v2_id = VersionId::generate();
    let mut obj_v2 = obj.clone();
    obj_v2.latest_version_id = v2_id.clone();
    obj_v2.updated_at = Utc::now();
    
    let v2 = VersionRecord {
        version_id: v2_id.clone(),
        object_id: obj_id.clone(),
        payload: serde_json::json!({"ver": 2}),
        standing: Standing { dimensions: vec![] },
        signal: None,
        created_at: Utc::now(),
        created_by: None,
    };
    
    store.deposit_object(obj_v2.clone(), v2.clone()).unwrap();
    
    // 3. Verification
    let fetched_obj = store.get_object(&obj_id).unwrap();
    assert_eq!(fetched_obj.latest_version_id, v2_id);
    
    let fetched_v1 = store.get_version(&obj_id, &v1_id).unwrap();
    assert_eq!(fetched_v1.payload, v1.payload);
    
    let fetched_v2 = store.get_version(&obj_id, &v2_id).unwrap();
    assert_eq!(fetched_v2.payload, v2.payload);
    
    // 4. Trace integrity
    let versions = store.list_versions(&obj_id).unwrap();
    assert_eq!(versions.len(), 2);
    assert!(versions.contains(&v1_id));
    assert!(versions.contains(&v2_id));
}

#[test]
fn test_complex_causality_linkage() {
    let dir = tempdir().unwrap();
    let store = FileStore::new(dir.path());
    
    let run_id = RunId::generate();
    let pkt_id = PacketId::generate();
    
    // Create Run
    let run = RunRecord {
        run_id: run_id.clone(),
        workflow_id: None,
        status: RunStatus::Scheduled,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    store.create_run(run).unwrap();
    
    // Create Packet linked to Run
    let packet = PacketRecord {
        packet_id: pkt_id.clone(),
        system_pack_ref: SystemPackId::generate(),
        system_ref: SystemId::generate(),
        run_id: run_id.clone(),
        workflow_ref: WorkflowId::generate(),
        transition_id: TransitionId::generate(),
        packet_template_ref: PacketTemplateId::generate(),
        root_object_ids: vec![],
        included_object_refs: vec![],
        excluded_object_refs: vec![],
        exclusion_reasons: vec![],
        relation_traversal_trace: vec![],
        standing_filter_trace: vec![],
        redaction_trace: vec![],
        provider_exposure_trace: vec![],
        instruction_ref: None,
        protocol_ref: RuntimeProtocolId::parse("rp_test").unwrap(),
        selection_ref: None,
        provider_profile_ref: None,
        output_contract_ref: ClassId::parse("cls_test").unwrap(),
        rendered_manifest: None,
        selection_trace: vec![],
        created_at: Utc::now(),
    };
    
    store.create_packet(packet).unwrap();
    
    let fetched_pkt = store.get_packet(&pkt_id).unwrap();
    assert_eq!(fetched_pkt.run_id, run_id);
}
