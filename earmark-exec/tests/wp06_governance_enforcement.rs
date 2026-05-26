use std::collections::BTreeMap;
use std::path::Path;

use earmark_core::{
    AssignmentStatus, ChangeSetDraft, DimensionId, Kind, RuntimeProfile, Standing, StandingPolicy,
    StandingRegistry, StandingRequestStatus, StandingTransitionRule, SystemDefinition, TokenId,
    TransitionAssignment, TransitionAssignmentId, WorkflowOperationKind,
};
use earmark_exec::governance_ops::{apply_standing_request, approve_standing_request};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_exec::validation::validate_transition_change_set;
use earmark_exec::ExecutionTransition;
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use tempfile::TempDir;

fn setup_store(root: &Path) -> (GitCanonicalStore, DerivedIndex) {
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();
    (store, index)
}

#[test]
fn test_transition_into_accepted_projection_fails_without_review_evidence() {
    let dir = TempDir::new().unwrap();
    let (store, mut index) = setup_store(dir.path());
    let registry = StandingRegistry::kernel_defaults();

    let target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let target_ref = write_object_and_index(&store, &mut index, &target).unwrap();

    let policy = StandingPolicy {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "kernel:review".to_string(),
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
    let policy_ref = write_object_and_index(&store, &mut index, &stored_policy).unwrap();

    let request = earmark_core::StandingTransitionRequest {
        target_object_id: target_ref.id.clone(),
        dimension: "kernel:review".to_string(),
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
    let request_ref = write_object_and_index(&store, &mut index, &stored_request).unwrap();
    let approved_ref = approve_standing_request(&store, &mut index, &request_ref, None).unwrap();

    let res = apply_standing_request(
        &store,
        &mut index,
        &approved_ref,
        Some(policy_ref.id.as_str()),
        None,
        &registry,
    );
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("requires accepted review"),
        "expected review-required error, got: {}",
        err
    );
}

#[test]
fn test_review_artifact_alone_does_not_mutate_standing() {
    let dir = TempDir::new().unwrap();
    let (store, mut index) = setup_store(dir.path());

    let target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let target_ref = write_object_and_index(&store, &mut index, &target).unwrap();
    let target_head = store.read_version(&target_ref).unwrap();

    assert_eq!(
        target_head
            .envelope
            .standing
            .get(&DimensionId::new("kernel:review"))
            .map(TokenId::as_str),
        Some("unreviewed")
    );

    let review_payload = earmark_governance::ReviewPayload {
        target: earmark_core::ObjectRef {
            id: target_ref.id.clone(),
            version_id: target_ref.version_id.clone(),
            kind: Kind::Object,
            class: Some("artifact".to_string()),
        },
        status: "accepted".to_string(),
        rationale: None,
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
    write_object_and_index(&store, &mut index, &stored_review).unwrap();

    let target_after = store.read_head(&target_ref.id).unwrap().unwrap();
    assert_eq!(
        target_after
            .envelope
            .standing
            .get(&DimensionId::new("kernel:review"))
            .map(TokenId::as_str),
        Some("unreviewed"),
        "review artifact should not mutate target standing"
    );
}

fn register_class(
    store: &GitCanonicalStore,
    index: &mut DerivedIndex,
    name: &str,
    relation_rules: Vec<earmark_core::RelationRule>,
) {
    let def = earmark_core::ClassDefinition {
        name: name.to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: earmark_core::JsonSchemaRef("inline:any".to_string()),
        standing_rules: earmark_core::ClassStandingRules::default(),
        relation_rules,
        validators: vec![],
    };
    let json = serde_json::to_string(&def).unwrap();
    let stored = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("system"),
        BTreeMap::new(),
        StoredPayload::from_markdown(&json),
        vec![],
    );
    let obj_ref = store.write_object(&stored).unwrap();
    index
        .upsert_head_object_from_store(store, &obj_ref.id)
        .unwrap();
}

#[test]
fn test_sealed_object_can_be_targeted_by_relation() {
    use earmark_core::{
        KernelProtocolId, ProtocolBinding, StandingDimensionDefinition, StandingTokenDefinition,
        SystemDefinition,
    };
    let dir = TempDir::new().unwrap();
    let (store, mut index) = setup_store(dir.path());

    register_class(
        &store,
        &mut index,
        "artifact",
        vec![earmark_core::RelationRule {
            relation_type: "references".to_string(),
            counterparty_classes: vec!["artifact".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        }],
    );

    let _sys = SystemDefinition {
        system_id: "test_seal_rel".to_string(),
        namespace: "test/seal_rel".to_string(),
        title: "SealRel".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![StandingDimensionDefinition {
            id: DimensionId::from_static("dim:immut"),
            default: TokenId::from_static("mutable_val"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::from_static("mutable_val"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::from_static("sealed_val"),
                    implements: vec![ProtocolBinding {
                        protocol: KernelProtocolId::from_static("kernel:immutability"),
                        state: Some("sealed".to_string()),
                        properties: BTreeMap::new(),
                    }],
                },
            ],
        }],
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "local".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "strict".to_string(),
        },
        activated_at: None,
    };

    let mut sealed_standing = Standing {
        values: BTreeMap::new(),
    };
    sealed_standing.values.insert(
        DimensionId::from_static("dim:immut"),
        TokenId::from_static("sealed_val"),
    );
    let sealed_target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        sealed_standing,
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let sealed_ref = write_object_and_index(&store, &mut index, &sealed_target).unwrap();

    let other = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let other_ref = write_object_and_index(&store, &mut index, &other).unwrap();

    let rel_payload = earmark_core::RelationPayload {
        source: earmark_core::ObjectRef {
            id: other_ref.id.clone(),
            version_id: other_ref.version_id.clone(),
            kind: Kind::Object,
            class: Some("artifact".to_string()),
        },
        target: earmark_core::ObjectRef {
            id: sealed_ref.id.clone(),
            version_id: sealed_ref.version_id.clone(),
            kind: Kind::Object,
            class: Some("artifact".to_string()),
        },
        relation_type: "references".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };
    let rel_result = earmark_exec::relation::persist_relation_canonical(
        &store,
        &mut index,
        rel_payload,
        earmark_core::Provenance::direct_input("operator"),
        earmark_core::RelationCreationMode::Declared,
        None,
    );
    assert!(
        rel_result.is_ok(),
        "sealed object should accept relation targeting: {:?}",
        rel_result
    );
}

#[test]
fn test_initial_accepted_standing_fails_without_review_or_trusted_provenance() {
    let dir = TempDir::new().unwrap();
    let (store, mut index) = setup_store(dir.path());

    let mut standing = Standing::default();
    standing
        .values
        .insert(DimensionId::new("kernel:review"), TokenId::new("accepted"));
    let target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        standing,
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let target_ref = write_object_and_index(&store, &mut index, &target).unwrap();

    let system = SystemDefinition {
        system_id: "test_sys".to_string(),
        namespace: "test/sys".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "local".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "strict".to_string(),
        },
        activated_at: None,
    };

    let transition = ExecutionTransition {
        id: earmark_core::TransitionId::parse("test").unwrap(),
        operation: WorkflowOperationKind::Review,
        input_contracts: vec![],
        output_contracts: vec![],
        instruction: None,
        compiled_context: None,
        policy: None,
        provider_profile: None,
    };

    let assignment = TransitionAssignment {
        id: TransitionAssignmentId::generate(),
        run_id: earmark_core::RunId::parse("test_run").unwrap(),
        transition_id: earmark_core::TransitionId::parse("test").unwrap(),
        assigned_to: "test".to_string(),
        status: AssignmentStatus::Assigned,
        input_object_ids: vec![],
        handoff_manifest_id: None,
        event_ids: vec![],
        blocked_reason: None,
        completion_change_set_id: None,
        assigned_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        expires_at: None,
        completed_at: None,
    };

    let draft = ChangeSetDraft {
        created_objects: vec![target_ref.id.clone()],
        created_relations: vec![],
        updated_objects: vec![],
        governance_events: vec![],
        standing_requests: vec![],
        blocked_operations: vec![],
        unresolved_ambiguities: vec![],
        rejected_candidates: vec![],
    };

    let (result, _requests) = validate_transition_change_set(
        &store,
        &mut index,
        &system,
        &transition,
        &assignment,
        &draft,
    )
    .expect("validation should not fail at transport level");

    assert!(!result.is_valid);
    assert!(
        result
            .failures
            .iter()
            .any(|f| f.contains("no same-change-set review evidence")),
        "expected initial-accepted failure, got failures: {:?}",
        result.failures
    );
}

#[test]
fn test_sealed_object_rejects_standing_transition() {
    use earmark_core::{
        KernelProtocolId, ProtocolBinding, StandingDimensionDefinition, StandingTokenDefinition,
    };

    let dir = TempDir::new().unwrap();
    let (store, mut index) = setup_store(dir.path());

    let system = SystemDefinition {
        system_id: "test_sys".to_string(),
        namespace: "test/sys".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![StandingDimensionDefinition {
            id: DimensionId::from_static("dim:immut"),
            default: TokenId::from_static("mutable"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::from_static("mutable"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::from_static("sealed"),
                    implements: vec![ProtocolBinding {
                        protocol: KernelProtocolId::from_static("kernel:immutability"),
                        state: Some("sealed".to_string()),
                        properties: BTreeMap::new(),
                    }],
                },
            ],
        }],
        runtime_profile: RuntimeProfile {
            execution_surface: "local".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "strict".to_string(),
        },
        activated_at: None,
    };
    let reg = StandingRegistry::from_system_definition(&system).expect("registry should be valid");

    let mut sealed_standing = Standing {
        values: BTreeMap::new(),
    };
    sealed_standing.values.insert(
        DimensionId::from_static("dim:immut"),
        TokenId::from_static("sealed"),
    );

    let target = StoredObject::new(
        Kind::Object,
        Some("artifact".to_string()),
        sealed_standing,
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(b"{}".to_vec()),
        vec![],
    );
    let target_ref = write_object_and_index(&store, &mut index, &target).unwrap();

    let policy = StandingPolicy {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "kernel:epistemic".to_string(),
            from: vec!["working".to_string()],
            to: vec!["supported".to_string()],
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
    let policy_ref = write_object_and_index(&store, &mut index, &stored_policy).unwrap();

    let request = earmark_core::StandingTransitionRequest {
        target_object_id: target_ref.id.clone(),
        dimension: "kernel:epistemic".to_string(),
        from_value: "working".to_string(),
        to_value: "supported".to_string(),
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
    let request_ref = write_object_and_index(&store, &mut index, &stored_request).unwrap();
    let approved_ref = approve_standing_request(&store, &mut index, &request_ref, None).unwrap();

    let res = apply_standing_request(
        &store,
        &mut index,
        &approved_ref,
        Some(policy_ref.id.as_str()),
        None,
        &reg,
    );
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("immutability"),
        "expected immutability violation, got: {}",
        err
    );
}
