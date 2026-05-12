use crate::error::ProviderFailure;
use crate::provider::{
    provider_record_from_response, provider_response_is_synthetic, MockAdapter, ProviderAdapter,
    ProviderExecutionOutcome, ProviderService,
};
use earmark_core::{
    Kind, ObjectId, ObjectRef, ProviderProfile, ProviderRequest, ProviderResponseContract,
    ScalarValue, VersionId, VersionRef, WorkflowDefinition, WorkflowOperation,
};
use earmark_index::*;
use earmark_store::GitCanonicalStore;
use earmark_store::*;
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_execution_ir_compilation() {
    let workflow = WorkflowDefinition {
        name: "test_flow".to_string(),
        version: "1".to_string(),
        description: None,
        operations: vec![WorkflowOperation {
            id: "op1".to_string(),
            kind: "transform".to_string(),
            input_contracts: vec!["note".to_string()],
            output_contracts: vec!["finding".to_string()],
            instruction: Some(VersionRef::new(
                ObjectId::parse("obj_00000000000000000000000000000001").unwrap(),
                VersionId::parse("ver_00000000000000000000000000000001").unwrap(),
            )),
            compiled_context: None,
            policy: None,
            provider_profile: None,
        }],
        edges: vec![],
        guards: vec![],
        output_contracts: vec![],
    };

    let ir = crate::helpers::compile_workflow(&workflow).unwrap();
    assert_eq!(ir.transitions.len(), 1);
    assert_eq!(ir.transitions[0].id, "op1");
    assert_eq!(ir.transitions[0].operation, "transform");
}

#[test]
fn test_engine_initialization() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let registry = crate::provider::ProviderRegistry::default();

    let _engine = crate::engine::ExecutionEngine::new(&store, &index, &registry);
}

struct NoopProviderService;

impl ProviderService for NoopProviderService {
    fn provide(
        &self,
        _profile: &earmark_core::ProviderProfile,
        _request: earmark_core::ProviderRequest,
        _transition_operation: &str,
    ) -> Result<ProviderExecutionOutcome, ProviderFailure> {
        Err(ProviderFailure::new(
            crate::error::ProviderFailureKind::ProviderUnavailable,
            "noop provider used only for seam substitution test",
        ))
    }
}

#[test]
fn test_engine_accepts_provider_service_test_double() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let provider = NoopProviderService;

    let _engine = crate::engine::ExecutionEngine::new(&store, &index, &provider);
}

#[test]
fn test_async_prep_boundaries_identify_provider_dispatch() {
    let boundaries = crate::async_prep::blocking_boundaries();
    assert!(boundaries
        .iter()
        .any(|b| b.id == "provider_dispatch" && b.future_async_candidate));
    assert!(boundaries
        .iter()
        .any(|b| b.id == "provider_http_client" && b.future_async_candidate));
}

#[test]
fn test_async_prep_sequence_starts_with_provider_boundary() {
    let sequence = crate::async_prep::recommended_async_migration_sequence();
    assert_eq!(
        sequence.first().copied(),
        Some("provider_service boundary (ProviderService + adapters)")
    );
}
struct BrokenProvider;
impl ProviderService for BrokenProvider {
    fn provide(
        &self,
        _profile: &earmark_core::ProviderProfile,
        _request: earmark_core::ProviderRequest,
        _transition_operation: &str,
    ) -> Result<ProviderExecutionOutcome, ProviderFailure> {
        Ok(ProviderExecutionOutcome {
            response: None,
            record: earmark_core::ProviderRecord {
                record_id: "prec_1".to_string(),
                request_id: "req_1".to_string(),
                run_id: "run_1".to_string(),
                work_packet: earmark_core::ObjectRef::new(
                    ObjectId::new(),
                    VersionId::new(),
                    earmark_core::Kind::WorkPacket,
                    None,
                ),
                provider_profile: VersionRef::new(ObjectId::new(), VersionId::new()),
                provider: "broken".to_string(),
                model: "broken".to_string(),
                status: "ok".to_string(),
                metadata: std::collections::BTreeMap::new(),
                advisory_warnings: vec![],
                usage: None,
                message: None,
                recorded_at: chrono::Utc::now(),
            },
        })
    }
}

#[test]
fn mock_adapter_provide_sets_synthetic_metadata() {
    let adapter = MockAdapter;
    let request = ProviderRequest {
        request_id: "req_test".to_string(),
        run_id: "run_test".to_string(),
        work_packet: ObjectRef::new(
            ObjectId::new(),
            VersionId::new(),
            earmark_core::Kind::WorkPacket,
            None,
        ),
        provider_profile: VersionRef::new(ObjectId::new(), VersionId::new()),
        instruction_text: "do work".to_string(),
        context_text: None,
        input_text: "do work".to_string(),
        work_surface_manifest: None,
        inputs: vec![],
        response_contract: ProviderResponseContract {
            format: "json".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        issued_at: chrono::Utc::now(),
    };
    let profile = ProviderProfile {
        name: "local_mock".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "echo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget {
            max_input_tokens: None,
            max_output_tokens: None,
            max_cost_usd: None,
            max_latency_ms: None,
        },
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: request.response_contract.clone(),
        http: None,
    };

    let response = adapter
        .provide(request, &profile, "transform")
        .expect("mock response");
    assert!(provider_response_is_synthetic(&response));
    assert_eq!(
        response.metadata.get("synthetic"),
        Some(&ScalarValue::Bool(true))
    );
    assert_eq!(
        response.metadata.get("synthetic_source"),
        Some(&ScalarValue::String("mock_provider".to_string()))
    );
    assert_eq!(
        response.metadata.get("production_eligible"),
        Some(&ScalarValue::Bool(false))
    );
}

#[test]
fn provider_record_from_response_preserves_synthetic_metadata() {
    let request = ProviderRequest {
        request_id: "req_test".to_string(),
        run_id: "run_test".to_string(),
        work_packet: ObjectRef::new(
            ObjectId::new(),
            VersionId::new(),
            earmark_core::Kind::WorkPacket,
            None,
        ),
        provider_profile: VersionRef::new(ObjectId::new(), VersionId::new()),
        instruction_text: "do work".to_string(),
        context_text: None,
        input_text: "do work".to_string(),
        work_surface_manifest: None,
        inputs: vec![],
        response_contract: ProviderResponseContract {
            format: "json".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        issued_at: chrono::Utc::now(),
    };
    let profile = ProviderProfile {
        name: "local_mock".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "echo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget {
            max_input_tokens: None,
            max_output_tokens: None,
            max_cost_usd: None,
            max_latency_ms: None,
        },
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: request.response_contract.clone(),
        http: None,
    };
    let response = earmark_core::ProviderResponse {
        request_id: request.request_id.clone(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
        status: "completed".to_string(),
        candidate_payload: "{}".to_string(),
        metadata: BTreeMap::new(),
        advisory_warnings: vec![],
        usage: None,
        received_at: chrono::Utc::now(),
    };
    let record = provider_record_from_response(&request, &profile, &response, None);
    assert_eq!(
        record.metadata.get("synthetic"),
        Some(&ScalarValue::Bool(true))
    );
}

#[test]
fn delegated_transform_output_sets_synthetic_headers() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();

    let input = StoredObject::new(
        earmark_core::Kind::Object,
        Some("note".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload::from_markdown("input"),
        vec![],
    );
    let input_ref = store.write_object(&input).unwrap();
    let input_obj_ref = earmark_core::ObjectRef::new(
        input_ref.id.clone(),
        input_ref.version_id.clone(),
        earmark_core::Kind::Object,
        Some("note".to_string()),
    );
    let instruction = earmark_core::InstructionPayload {
        name: "extract".to_string(),
        version: "1".to_string(),
        purpose: "extract".to_string(),
        input_classes: vec!["note".to_string()],
        output_classes: vec!["finding".to_string()],
        execution_policy: "delegated".to_string(),
        provider_profile: None,
        trace_policy: "summary".to_string(),
        register: "machined".to_string(),
        body: earmark_core::MarkdownBody::new("extract"),
    };
    let response = earmark_core::ProviderResponse {
        request_id: "req".to_string(),
        provider: "mock".to_string(),
        model: "echo".to_string(),
        status: "completed".to_string(),
        candidate_payload: "fixture".to_string(),
        metadata: std::collections::BTreeMap::from([
            ("synthetic".to_string(), ScalarValue::Bool(true)),
            (
                "synthetic_source".to_string(),
                ScalarValue::String("mock_provider".to_string()),
            ),
            ("production_eligible".to_string(), ScalarValue::Bool(false)),
        ]),
        advisory_warnings: vec![],
        usage: None,
        received_at: chrono::Utc::now(),
    };

    let instr_stored = StoredObject::new(
        Kind::Instruction,
        Some("instruction".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown("extract"),
        vec![],
    );
    let instr_ref = store.write_object(&instr_stored).unwrap();

    let index = DerivedIndex::open(dir.path()).unwrap();
    let artifacts = crate::persistence::create_delegated_transform_output(
        &store,
        &index,
        &instruction,
        "finding",
        &[input_obj_ref],
        &instr_ref,
        response,
    )
    .unwrap();
    let stored = store
        .read_version(&VersionRef::new(
            artifacts.output.id.clone(),
            artifacts.output.version_id.clone(),
        ))
        .unwrap();
    assert_eq!(
        stored.envelope.headers.get("synthetic"),
        Some(&earmark_core::HeaderValue::Bool(true))
    );
    assert_eq!(
        stored.envelope.headers.get("production_eligible"),
        Some(&earmark_core::HeaderValue::Bool(false))
    );
}

#[test]
fn test_delegated_outcome_with_none_response_returns_error_instead_of_panicking() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let engine = crate::engine::ExecutionEngine::new(&store, &index, &BrokenProvider);

    let prof = earmark_core::ProviderProfile {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "broken".to_string(),
        model: "broken".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: earmark_core::ProviderBudget {
            max_input_tokens: None,
            max_output_tokens: None,
            max_cost_usd: None,
            max_latency_ms: None,
        },
        allowed_operations: vec!["transform".to_string()],
        exposure: earmark_core::ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: true,
            allow_export_requests: false,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: "json".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: true,
        },
        http: None,
    };
    let prof_obj = StoredObject::new(
        earmark_core::Kind::ProviderProfile,
        Some("provider_profile".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&prof).unwrap()),
        vec![],
    );
    let prof_ref = engine.store.write_object(&prof_obj).unwrap();

    let instruction = earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        purpose: "test".to_string(),
        input_classes: vec!["note".to_string()],
        output_classes: vec!["summary".to_string()],
        execution_policy: "delegated".to_string(),
        provider_profile: Some(prof_ref.clone()),
        trace_policy: "full".to_string(),
        register: "user".to_string(),
        body: earmark_core::MarkdownBody::new("test body"),
    };

    let note = StoredObject::new(
        earmark_core::Kind::Object,
        Some("note".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload::from_markdown("note content"),
        vec![],
    );
    let note_ref = engine.store.write_object(&note).unwrap();

    let instr_text = format!(
        "---\n{}---\ntest body",
        earmark_core::to_yaml(&instruction).unwrap()
    );
    let instr_obj = StoredObject::new(
        earmark_core::Kind::Instruction,
        Some("instruction".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload::from_markdown(instr_text),
        vec![],
    );
    let instr_ref = engine.store.write_object(&instr_obj).unwrap();

    engine.index.rebuild_from_store(engine.store).unwrap();

    let ir = crate::ir::ExecutionIr {
        transitions: vec![crate::ir::ExecutionTransition {
            id: "trans_1".to_string(),
            operation: "transform".to_string(),
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: Some(instr_ref.clone()),
            compiled_context: None,
            policy: None,
            provider_profile: None,
        }],
        edges: vec![],
        guards: vec![],
    };

    let mut active_objects = vec![earmark_core::ObjectRef::new(
        note_ref.id.clone(),
        note_ref.version_id.clone(),
        earmark_core::Kind::Object,
        Some("note".to_string()),
    )];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "surf_1".to_string(),
        compiled_context: earmark_core::VersionRef::new(ObjectId::new(), VersionId::new()),
        work_packet: Some(earmark_core::ObjectRef::new(
            ObjectId::new(),
            VersionId::new(),
            earmark_core::Kind::WorkPacket,
            None,
        )),
        generated_at: chrono::Utc::now(),
        objects: vec![],
        boundary_relations: vec![],
        constraints: BTreeMap::new(),
        warnings: vec![],
    });

    let mut state = crate::state::ExecutionState {
        active_objects: &mut active_objects,
        emitted_packets: &mut emitted_packets,
        emitted_objects: &mut emitted_objects,
        governance_events: &mut governance_events,
        compiled_context: &mut compiled_context,
    };

    let sys_ref = VersionRef::new(ObjectId::new(), VersionId::new());
    let mut record = crate::helpers::new_run_record(
        "run_1".to_string(),
        sys_ref.clone(),
        VersionRef::new(ObjectId::new(), VersionId::new()),
        vec![],
    );

    let result = engine.execute_transition(
        &crate::ir::WorkflowRunRequest {
            run_id: "run_1".to_string(),
            system_definition: sys_ref.clone(),
            workflow: VersionRef::new(ObjectId::new(), VersionId::new()),
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: false,
            inputs: vec![],
        },
        &earmark_core::SystemDefinition {
            system_id: "test".to_string(),
            namespace: "test".to_string(),
            title: "test".to_string(),
            description: None,
            runtime_profile: earmark_core::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "manifest".to_string(),
            },
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![prof_ref],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![],
            activated_at: None,
        },
        &ir,
        &ir.transitions[0],
        &mut state,
        &mut record,
        &earmark_connected_context::DEFAULT_COMPILED_CONTEXT_COMPILER,
    );

    assert!(result.is_err(), "Expected error but got OK");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("delegated outcome did not contain a response"),
        "Error message was: {}",
        err_msg
    );
}

#[test]
fn test_privileged_relation_creation_and_validation() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

    let source = StoredObject::new(
        earmark_core::Kind::Object,
        Some("note".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload::from_markdown("source"),
        vec![],
    );
    let source_ref = store.write_object(&source).unwrap();
    index
        .upsert_head_object_from_store(&store, &source_ref.id)
        .unwrap();

    let target = StoredObject::new(
        earmark_core::Kind::Object,
        Some("finding".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        std::collections::BTreeMap::new(),
        StoredPayload::from_markdown("target"),
        vec![],
    );
    let target_ref = store.write_object(&target).unwrap();
    index
        .upsert_head_object_from_store(&store, &target_ref.id)
        .unwrap();

    let rel_payload = earmark_core::RelationPayload {
        source: source.object_ref(),
        target: target.object_ref(),
        relation_type: earmark_core::REL_TYPE_USED_INSTRUCTION.to_string(),
        qualifiers: std::collections::BTreeMap::new(),
        scope: None,
    };

    let rel_ref = crate::persist_relation_canonical(
        &store,
        &index,
        rel_payload,
        earmark_core::Provenance::direct_input("runtime"),
        earmark_core::RelationCreationMode::PrivilegedSystem,
        None,
    )
    .unwrap();

    let stored_rel = store.read_version(&rel_ref.version_ref()).unwrap();
    assert_eq!(
        stored_rel.envelope.headers.get("relation_creation_mode"),
        Some(&earmark_core::HeaderValue::String(
            "privileged_system".to_string()
        ))
    );

    // Validation should pass even if class rules don't exist for this relation type
    let system = earmark_core::SystemDefinition {
        system_id: "test".to_string(),
        namespace: "test".to_string(),
        title: "test".to_string(),
        description: None,
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "local".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "manifest".to_string(),
        },
        classes: vec![], // No classes defined, so no rules
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        activated_at: None,
    };

    let (result, _) = crate::validation::validate_transition_change_set(
        &store,
        &index,
        &system,
        &crate::ir::ExecutionTransition {
            id: "test".to_string(),
            operation: "transform".to_string(),
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: None,
            compiled_context: None,
            policy: None,
            provider_profile: None,
        },
        &earmark_core::TransitionAssignment {
            id: earmark_core::TransitionAssignmentId::new(),
            run_id: "run".to_string(),
            transition_id: "test".to_string(),
            assigned_to: "test".to_string(),
            status: earmark_core::AssignmentStatus::Assigned,
            input_object_ids: vec![],
            handoff_manifest_id: None,
            event_ids: vec![],
            blocked_reason: None,
            completion_change_set_id: None,
            assigned_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            expires_at: None,
            completed_at: None,
        },
        &earmark_core::ChangeSetDraft {
            created_objects: vec![],
            created_relations: vec![rel_ref.id],
            updated_objects: vec![],
            governance_events: vec![],
            standing_requests: vec![],
            blocked_operations: vec![],
            unresolved_ambiguities: vec![],
            rejected_candidates: vec![],
        },
    )
    .unwrap();

    assert!(
        result.is_valid,
        "Privileged relation should pass validation"
    );
}

#[test]
fn test_privileged_relation_enforcement_failure() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

    let payload = earmark_core::RelationPayload {
        source: ObjectRef::new(ObjectId::new(), VersionId::new(), Kind::Object, None),
        target: ObjectRef::new(ObjectId::new(), VersionId::new(), Kind::Object, None),
        relation_type: "some_ordinary_type".to_string(),
        qualifiers: BTreeMap::new(),
        scope: None,
    };

    let result = crate::persist_relation_canonical(
        &store,
        &index,
        payload,
        earmark_core::Provenance::direct_input("test"),
        earmark_core::RelationCreationMode::PrivilegedSystem,
        None,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err
        .to_string()
        .contains("is not a privileged system relation"));
}

#[test]
fn test_resolution_error_propagation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    let obj_id = ObjectId::new();
    let head_path = store
        .root()
        .join(".earmark/canonical/heads")
        .join(format!("{}.json", obj_id.as_str()));
    std::fs::create_dir_all(head_path.parent().unwrap()).unwrap();
    std::fs::write(&head_path, "invalid json").unwrap();

    let version_ref = VersionRef::new(
        obj_id,
        earmark_core::VersionId::parse("ver_00000000000000000000000000000000").unwrap(),
    ); // latest

    let res =
        crate::resolution::resolve_version_for_kind(&store, &index, &version_ref, Kind::Workflow);

    assert!(res.is_err());
    // It should NOT be IncompleteExecution (which would mean it fell back and failed),
    // it should be a Store error (JSON parse error from reading the head ref)
    match res.unwrap_err() {
        crate::error::ExecError::Store(_) => {} // OK
        e => panic!("Expected Store error, got {:?}", e),
    }
}
