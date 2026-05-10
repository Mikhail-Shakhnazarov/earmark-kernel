use chrono::Utc;
use earmark_core::{
    ChangeSetId, HandoffManifest, Kind, ObjectId, ReviewStanding, Standing, StandingConstraint,
    WorkflowDefinition, WorkflowOperation,
};
use earmark_exec::handoff::reconstruct_successor_inputs_from_handoff;
use earmark_exec::persistence_helpers::{write_batch_and_index, write_object_and_index};
use earmark_index::DerivedIndex;
use earmark_store::{BatchWrite, CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_handoff_class_exclusion() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Create a root object of class "finding"
    let root_obj = StoredObject::new(
        Kind::Object,
        Some("finding".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"root".to_vec()),
        vec![],
    );
    let root_ref = write_object_and_index(&store, &index, &root_obj).unwrap();

    // 2. Create a related object of class "source_note" (not allowed)
    let note_obj = StoredObject::new(
        Kind::Object,
        Some("source_note".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"note".to_vec()),
        vec![],
    );
    let note_ref = write_object_and_index(&store, &index, &note_obj).unwrap();

    // Create relation
    let rel = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(
            serde_json::to_vec(&serde_json::json!({
                "source": earmark_core::ObjectRef {
                    id: root_ref.id.clone(),
                    version_id: root_ref.version_id.clone(),
                    kind: Kind::Object,
                    class: Some("finding".to_string()),
                },
                "target": earmark_core::ObjectRef {
                    id: note_ref.id.clone(),
                    version_id: note_ref.version_id.clone(),
                    kind: Kind::Object,
                    class: Some("source_note".to_string()),
                },
                "relation_type": "linked",
                "qualifiers": {},
                "scope": "test"
            }))
            .unwrap(),
        ),
        vec![],
    );
    write_object_and_index(&store, &index, &rel).unwrap();

    // 3. Create handoff allowing only "finding"
    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "run1".to_string(),
        from_transition_id: "t1".to_string(),
        to_transition_id: None,
        source_change_set_id: ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![root_ref.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec!["finding".to_string()],
        allowed_output_classes: vec![],
        allowed_relation_types: vec!["linked".to_string()],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };

    // 4. Reconstruct
    let inputs = reconstruct_successor_inputs_from_handoff(&store, &index, &handoff).unwrap();

    // Should only contain the finding, not the source_note
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].id, root_ref.id);
}

#[test]
fn test_handoff_standing_exclusion() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Create a root object (Accepted)
    let standing_accepted = Standing {
        review: ReviewStanding::Accepted,
        ..Default::default()
    };
    let root_obj = StoredObject::new(
        Kind::Object,
        Some("finding".to_string()),
        standing_accepted,
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"root".to_vec()),
        vec![],
    );
    let root_ref = write_object_and_index(&store, &index, &root_obj).unwrap();

    // 2. Create a related object (Rejected)
    let standing_rejected = Standing {
        review: ReviewStanding::Rejected,
        ..Default::default()
    };
    let rejected_obj = StoredObject::new(
        Kind::Object,
        Some("finding".to_string()),
        standing_rejected,
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"rejected".to_vec()),
        vec![],
    );
    let rejected_ref = write_object_and_index(&store, &index, &rejected_obj).unwrap();

    // Create relation
    let rel = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(
            serde_json::to_vec(&serde_json::json!({
                "source": earmark_core::ObjectRef {
                    id: root_ref.id.clone(),
                    version_id: root_ref.version_id.clone(),
                    kind: Kind::Object,
                    class: Some("finding".to_string()),
                },
                "target": earmark_core::ObjectRef {
                    id: rejected_ref.id.clone(),
                    version_id: rejected_ref.version_id.clone(),
                    kind: Kind::Object,
                    class: Some("finding".to_string()),
                },
                "relation_type": "linked",
                "qualifiers": {},
                "scope": "test"
            }))
            .unwrap(),
        ),
        vec![],
    );
    write_object_and_index(&store, &index, &rel).unwrap();

    // 3. Create handoff requiring review: accepted
    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "run1".to_string(),
        from_transition_id: "t1".to_string(),
        to_transition_id: None,
        source_change_set_id: ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![root_ref.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec![],
        allowed_output_classes: vec![],
        allowed_relation_types: vec!["linked".to_string()],
        standing_constraints: vec![StandingConstraint {
            constraint_type: "allowed_review".to_string(),
            requirements: vec!["accepted".to_string()],
        }],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };

    // 4. Reconstruct
    let inputs = reconstruct_successor_inputs_from_handoff(&store, &index, &handoff).unwrap();

    // Should only contain the accepted object
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].id, root_ref.id);
}

#[test]
fn test_handoff_depth_limit() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // Create a chain of 4 objects: A -> B -> C -> D
    // Depth limit is 2, so A (0), B (1), C (2) should be admitted, D (3) should NOT.

    let a = write_object_and_index(&store, &index, &simple_obj("a")).unwrap();
    let b = write_object_and_index(&store, &index, &simple_obj("b")).unwrap();
    let c = write_object_and_index(&store, &index, &simple_obj("c")).unwrap();
    let d = write_object_and_index(&store, &index, &simple_obj("d")).unwrap();

    write_rel(&store, &index, &a, &b, "linked");
    write_rel(&store, &index, &b, &c, "linked");
    write_rel(&store, &index, &c, &d, "linked");

    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "run1".to_string(),
        from_transition_id: "t1".to_string(),
        to_transition_id: None,
        source_change_set_id: ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![a.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec![],
        allowed_output_classes: vec![],
        allowed_relation_types: vec!["linked".to_string()],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };

    let inputs = reconstruct_successor_inputs_from_handoff(&store, &index, &handoff).unwrap();

    assert_eq!(inputs.len(), 3); // A, B, C
    let ids: std::collections::BTreeSet<_> =
        inputs.iter().map(|o| o.id.as_str().to_string()).collect();
    assert!(ids.contains(a.id.as_str()));
    assert!(ids.contains(b.id.as_str()));
    assert!(ids.contains(c.id.as_str()));
    assert!(!ids.contains(d.id.as_str()));
}

fn simple_obj(title: &str) -> StoredObject {
    StoredObject::new(
        Kind::Object,
        Some("finding".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(title.as_bytes().to_vec()),
        vec![],
    )
}

fn write_rel<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    source: &earmark_core::VersionRef,
    target: &earmark_core::VersionRef,
    rel_type: &str,
) {
    let rel = StoredObject::new(
        Kind::Relation,
        None,
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(
            serde_json::to_vec(&serde_json::json!({
                "source": earmark_core::ObjectRef {
                    id: source.id.clone(),
                    version_id: source.version_id.clone(),
                    kind: Kind::Object,
                    class: Some("finding".to_string()),
                },
                "target": earmark_core::ObjectRef {
                    id: target.id.clone(),
                    version_id: target.version_id.clone(),
                    kind: Kind::Object,
                    class: Some("finding".to_string()),
                },
                "relation_type": rel_type,
                "qualifiers": {},
                "scope": "test"
            }))
            .unwrap(),
        ),
        vec![],
    );
    write_object_and_index(store, index, &rel).unwrap();
}

#[test]
fn test_multi_output_transform_rejection() {
    let workflow = WorkflowDefinition {
        name: "bad_workflow".to_string(),
        version: "1".to_string(),
        description: None,
        operations: vec![WorkflowOperation {
            id: "op1".to_string(),
            kind: "transform".to_string(),
            input_contracts: vec!["input".to_string()],
            output_contracts: vec!["out1".to_string(), "out2".to_string()], // TWO OUTPUTS
            instruction: Some(earmark_core::VersionRef::new(
                ObjectId::parse("obj_00000000000000000000000000000001").unwrap(),
                earmark_core::VersionId::parse("ver_00000000000000000000000000000001").unwrap(),
            )),
            compiled_context: None,
            policy: None,
            provider_profile: None,
        }],
        edges: vec![],
        guards: vec![],
    };

    let res = earmark_declarations::validate_workflow_definition(&workflow);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("multi-output transform operations are not implemented"));
}

#[test]
fn test_handoff_cycle_handling() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // Create a cycle: A -> B -> A
    let a = write_object_and_index(&store, &index, &simple_obj("a")).unwrap();
    let b = write_object_and_index(&store, &index, &simple_obj("b")).unwrap();

    write_rel(&store, &index, &a, &b, "linked");
    write_rel(&store, &index, &b, &a, "linked");

    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "run1".to_string(),
        from_transition_id: "t1".to_string(),
        to_transition_id: None,
        source_change_set_id: ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![a.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec![],
        allowed_output_classes: vec![],
        allowed_relation_types: vec!["linked".to_string()],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };

    let inputs = reconstruct_successor_inputs_from_handoff(&store, &index, &handoff).unwrap();

    // Should contain A and B, and terminate.
    assert_eq!(inputs.len(), 2);
}

#[test]
fn test_handoff_object_limit_exhaustion() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // Use batch to speed up setup
    let root_obj = simple_obj("root");
    let mut batch = BatchWrite {
        message: "batch setup".to_string(),
        objects: vec![root_obj.clone()],
    };
    for i in 0..101 {
        batch.objects.push(simple_obj(&format!("child_{}", i)));
    }

    let refs = write_batch_and_index(&store, &index, &batch).unwrap();
    let root_ref = &refs[0];

    let mut rel_batch = BatchWrite {
        message: "rel setup".to_string(),
        objects: vec![],
    };
    for child_ref in refs.iter().take(102).skip(1) {
        let rel = StoredObject::new(
            Kind::Relation,
            None,
            Standing::default(),
            earmark_core::Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_json_bytes(
                serde_json::to_vec(&serde_json::json!({
                    "source": earmark_core::ObjectRef {
                        id: root_ref.id.clone(),
                        version_id: root_ref.version_id.clone(),
                        kind: Kind::Object,
                        class: Some("finding".to_string()),
                    },
                    "target": earmark_core::ObjectRef {
                        id: child_ref.id.clone(),
                        version_id: child_ref.version_id.clone(),
                        kind: Kind::Object,
                        class: Some("finding".to_string()),
                    },
                    "relation_type": "linked",
                    "qualifiers": {},
                    "scope": "test"
                }))
                .unwrap(),
            ),
            vec![],
        );
        rel_batch.objects.push(rel);
    }
    write_batch_and_index(&store, &index, &rel_batch).unwrap();

    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "run1".to_string(),
        from_transition_id: "t1".to_string(),
        to_transition_id: None,
        source_change_set_id: ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![root_ref.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec![],
        allowed_output_classes: vec![],
        allowed_relation_types: vec!["linked".to_string()],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };

    let res = reconstruct_successor_inputs_from_handoff(&store, &index, &handoff);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("expansion object limit reached (100)"));
}

#[test]
fn test_handoff_relation_limit_exhaustion() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    let target = simple_obj("target");
    let source = simple_obj("source");
    let refs = write_batch_and_index(
        &store,
        &index,
        &BatchWrite {
            message: "setup".to_string(),
            objects: vec![target.clone(), source.clone()],
        },
    )
    .unwrap();
    let target_ref = &refs[0];
    let source_ref = &refs[1];

    let mut rel_batch = BatchWrite {
        message: "setup".to_string(),
        objects: vec![],
    };
    for i in 0..501 {
        let rel = StoredObject::new(
            Kind::Relation,
            None,
            Standing::default(),
            earmark_core::Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_json_bytes(
                serde_json::to_vec(&serde_json::json!({
                    "source": earmark_core::ObjectRef {
                        id: source_ref.id.clone(),
                        version_id: source_ref.version_id.clone(),
                        kind: Kind::Object,
                        class: Some("finding".to_string()),
                    },
                    "target": earmark_core::ObjectRef {
                        id: target_ref.id.clone(),
                        version_id: target_ref.version_id.clone(),
                        kind: Kind::Object,
                        class: Some("finding".to_string()),
                    },
                    "relation_type": "linked",
                    "qualifiers": { "id": i },
                    "scope": "test"
                }))
                .unwrap(),
            ),
            vec![],
        );
        rel_batch.objects.push(rel);
    }
    write_batch_and_index(&store, &index, &rel_batch).unwrap();

    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "run1".to_string(),
        from_transition_id: "t1".to_string(),
        to_transition_id: None,
        source_change_set_id: ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![source_ref.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec![],
        allowed_output_classes: vec![],
        allowed_relation_types: vec!["linked".to_string()],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };

    let res = reconstruct_successor_inputs_from_handoff(&store, &index, &handoff);
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("expansion relation limit reached (500)"));
}
