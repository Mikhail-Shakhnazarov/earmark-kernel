use earmark_core::{
    Kind, ReviewStanding, Standing, StandingPolicy, StandingRequestStatus, StandingTransitionRule,
    VersionId,
};
use earmark_exec::governance_ops::{apply_standing_request, approve_standing_request};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_standing_request_lifecycle() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Create a target object
    let target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let target_ref = write_object_and_index(&store, &index, &target).unwrap();

    // 2. Create a policy
    let policy = StandingPolicy {
        name: "test-policy".to_string(),
        version: "1".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "review".to_string(),
            from: vec!["unreviewed".to_string()],
            to: vec!["accepted".to_string()],
            requires_review: true,
        }],
        operation_requirements: vec![],
        escalations: vec![],
        rationale: None,
    };
    let stored_policy = StoredObject::new(
        Kind::Object,
        Some("standing_policy".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("governance"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Test Policy".to_string()),
        )]),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&policy).unwrap()),
        vec![],
    );
    let policy_ref = write_object_and_index(&store, &index, &stored_policy).unwrap();

    // 3. Create a standing request (Proposed)
    let request = earmark_core::StandingTransitionRequest {
        target_object_id: target_ref.id.clone(),
        dimension: "review".to_string(),
        from_value: "unreviewed".to_string(),
        to_value: "accepted".to_string(),
        rationale: Some("Requesting review upgrade".to_string()),
        status: StandingRequestStatus::Proposed,
    };
    let stored_request = StoredObject::new(
        Kind::Object,
        Some("standing_transition_request".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&request).unwrap()),
        vec![],
    );
    let request_ref = write_object_and_index(&store, &index, &stored_request).unwrap();

    // 4. Try to apply Proposed request (should fail)
    let res = apply_standing_request(
        &store,
        &index,
        &request_ref,
        Some(policy_ref.id.as_str()),
        None,
    );
    assert!(res.is_err(), "should fail to apply proposed request");

    // 5. Approve the request
    let approved_ref =
        approve_standing_request(&store, &index, &request_ref, Some("Approved".to_string()))
            .unwrap();

    // 6. Try to apply Approved request without review evidence (should fail because rule requires review)
    let res = apply_standing_request(
        &store,
        &index,
        &approved_ref,
        Some(policy_ref.id.as_str()),
        None,
    );
    assert!(res.is_err(), "should fail to apply without review evidence");

    // 7. Create review evidence
    let review_payload = earmark_governance::ReviewPayload {
        target: earmark_core::ObjectRef {
            id: target_ref.id.clone(),
            version_id: target_ref.version_id.clone(),
            kind: Kind::Object,
            class: Some("artifact".to_string()),
        },
        status: "accepted".to_string(),
        rationale: Some("Good work".to_string()),
        reviewed_at: chrono::Utc::now(),
    };
    let stored_review = StoredObject::new(
        Kind::Review,
        Some("review".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("reviewer"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&review_payload).unwrap()),
        vec![],
    );
    write_object_and_index(&store, &index, &stored_review).unwrap();

    // 8. Apply Approved request with review evidence and a reason
    let apply_reason = "Final approval reason".to_string();
    let (new_target_ref, final_request_ref) = apply_standing_request(
        &store,
        &index,
        &approved_ref,
        Some(policy_ref.id.as_str()),
        Some(apply_reason.clone()),
    )
    .unwrap();

    // 9. Verify results
    let updated_target = store.read_version(&new_target_ref).unwrap();
    assert_eq!(
        updated_target.envelope.standing.review,
        ReviewStanding::Accepted
    );
    assert_eq!(updated_target.envelope.parents, vec![target_ref]);

    let final_request_obj = store.read_version(&final_request_ref).unwrap();
    let final_request: earmark_core::StandingTransitionRequest =
        serde_json::from_slice(&final_request_obj.payload.bytes).unwrap();
    assert_eq!(final_request.status, StandingRequestStatus::Applied);
    assert_eq!(final_request.rationale, Some(apply_reason));
}

#[test]
fn test_standing_request_drift_failure() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Create a target object
    let target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let target_ref = write_object_and_index(&store, &index, &target).unwrap();

    // 2. Create a standing request (Proposed) expected from "unreviewed"
    let request = earmark_core::StandingTransitionRequest {
        target_object_id: target_ref.id.clone(),
        dimension: "review".to_string(),
        from_value: "unreviewed".to_string(),
        to_value: "accepted".to_string(),
        rationale: Some("Requesting review upgrade".to_string()),
        status: StandingRequestStatus::Proposed,
    };
    let stored_request = StoredObject::new(
        Kind::Object,
        Some("standing_transition_request".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&request).unwrap()),
        vec![],
    );
    let request_ref = write_object_and_index(&store, &index, &stored_request).unwrap();
    let approved_ref = approve_standing_request(&store, &index, &request_ref, None).unwrap();

    // 3. Drift: Update target object standing externally (e.g. to "flagged")
    let mut target_head = store.read_version(&target_ref).unwrap();
    target_head.envelope.standing.review = ReviewStanding::Rejected;
    target_head.envelope.version_id = VersionId::new();
    write_object_and_index(&store, &index, &target_head).unwrap();

    // 4. Try to apply request (should fail due to drift: current is "rejected", request expected "unreviewed")
    let policy = StandingPolicy {
        name: "test-policy".to_string(),
        version: "1".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "review".to_string(),
            from: vec!["unreviewed".to_string()],
            to: vec!["accepted".to_string()],
            requires_review: false,
        }],
        operation_requirements: vec![],
        escalations: vec![],
        rationale: None,
    };
    let stored_policy = StoredObject::new(
        Kind::Object,
        Some("standing_policy".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("governance"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&policy).unwrap()),
        vec![],
    );
    let policy_ref = write_object_and_index(&store, &index, &stored_policy).unwrap();
    let res = apply_standing_request(
        &store,
        &index,
        &approved_ref,
        Some(policy_ref.id.as_str()),
        None,
    );
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("drift detected"),
        "error should mention drift, got: {}",
        err
    );
}

#[test]
fn test_standing_request_noop_apply() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Create a target object already in "accepted" review state
    let mut standing = Standing::default();
    standing.review = ReviewStanding::Accepted;
    let target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        standing,
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let target_ref = write_object_and_index(&store, &index, &target).unwrap();

    // 2. Create a standing request to move to "accepted" (already there)
    let request = earmark_core::StandingTransitionRequest {
        target_object_id: target_ref.id.clone(),
        dimension: "review".to_string(),
        from_value: "accepted".to_string(),
        to_value: "accepted".to_string(),
        rationale: None,
        status: StandingRequestStatus::Proposed,
    };
    let stored_request = StoredObject::new(
        Kind::Object,
        Some("standing_transition_request".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&request).unwrap()),
        vec![],
    );
    let request_ref = write_object_and_index(&store, &index, &stored_request).unwrap();
    let approved_ref = approve_standing_request(&store, &index, &request_ref, None).unwrap();

    // 2b. Create a policy
    let policy = StandingPolicy {
        name: "test-policy".to_string(),
        version: "1".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "review".to_string(),
            from: vec!["accepted".to_string()],
            to: vec!["accepted".to_string()],
            requires_review: false,
        }],
        operation_requirements: vec![],
        escalations: vec![],
        rationale: None,
    };
    let stored_policy = StoredObject::new(
        Kind::Object,
        Some("standing_policy".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("governance"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&policy).unwrap()),
        vec![],
    );
    let policy_ref = write_object_and_index(&store, &index, &stored_policy).unwrap();

    // 3. Apply request (should be no-op for target but Applied for request)
    let (final_target_ref, final_request_ref) = apply_standing_request(
        &store,
        &index,
        &approved_ref,
        Some(policy_ref.id.as_str()),
        Some("No-op reason".to_string()),
    )
    .unwrap();

    assert_eq!(
        final_target_ref, target_ref,
        "target version should not have changed"
    );

    let final_request_obj = store.read_version(&final_request_ref).unwrap();
    let final_request: earmark_core::StandingTransitionRequest =
        serde_json::from_slice(&final_request_obj.payload.bytes).unwrap();
    assert_eq!(final_request.status, StandingRequestStatus::Applied);
    assert_eq!(final_request.rationale, Some("No-op reason".to_string()));
}

#[test]
fn test_standing_request_version_specific_review() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    // 1. Create a target object (V1)
    let target_v1 = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"v1".to_vec()),
        vec![],
    );
    let target_v1_ref = write_object_and_index(&store, &index, &target_v1).unwrap();

    // 2. Create a newer version (V2)
    let mut target_v2 = target_v1.clone();
    target_v2.envelope.version_id = VersionId::new();
    target_v2.payload = StoredPayload::from_json_bytes(b"v2".to_vec());
    target_v2.envelope.payload_ref = earmark_core::PayloadRef::from_bytes(&target_v2.payload.bytes);
    let _target_v2_ref = write_object_and_index(&store, &index, &target_v2).unwrap();

    // 3. Create a policy requiring review
    let policy = StandingPolicy {
        name: "test-policy".to_string(),
        version: "1".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "review".to_string(),
            from: vec!["unreviewed".to_string()],
            to: vec!["accepted".to_string()],
            requires_review: true,
        }],
        operation_requirements: vec![],
        escalations: vec![],
        rationale: None,
    };
    let stored_policy = StoredObject::new(
        Kind::Object,
        Some("standing_policy".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("governance"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&policy).unwrap()),
        vec![],
    );
    let policy_ref = write_object_and_index(&store, &index, &stored_policy).unwrap();

    // 4. Create request for V2
    let request = earmark_core::StandingTransitionRequest {
        target_object_id: target_v1_ref.id.clone(),
        dimension: "review".to_string(),
        from_value: "unreviewed".to_string(),
        to_value: "accepted".to_string(),
        rationale: None,
        status: StandingRequestStatus::Proposed,
    };
    let stored_request = StoredObject::new(
        Kind::Object,
        Some("standing_transition_request".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&request).unwrap()),
        vec![],
    );
    let request_ref = write_object_and_index(&store, &index, &stored_request).unwrap();
    let approved_ref = approve_standing_request(&store, &index, &request_ref, None).unwrap();

    // 5. Create review targeting V1 (wrong version for the current head V2)
    let review_payload = earmark_governance::ReviewPayload {
        target: earmark_core::ObjectRef {
            id: target_v1_ref.id.clone(),
            version_id: target_v1_ref.version_id.clone(),
            kind: Kind::Object,
            class: Some("artifact".to_string()),
        },
        status: "accepted".to_string(),
        rationale: None,
        reviewed_at: chrono::Utc::now(),
    };
    let stored_review = StoredObject::new(
        Kind::Review,
        None,
        Standing::default(),
        earmark_core::Provenance::direct_input("reviewer"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&review_payload).unwrap()),
        vec![],
    );
    write_object_and_index(&store, &index, &stored_review).unwrap();

    // 6. Try to apply request to V2 (should fail because review is for V1)
    let res = apply_standing_request(
        &store,
        &index,
        &approved_ref,
        Some(policy_ref.id.as_str()),
        None,
    );
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .to_string()
        .contains("requires accepted review evidence for the current version"));
}
