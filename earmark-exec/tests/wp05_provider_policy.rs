use earmark_connected_context::DEFAULT_COMPILED_CONTEXT_COMPILER;
use earmark_core::{
    Kind, ObjectId, ObjectRef, ProviderBudget, ProviderExposure, ProviderProfile, ProviderRequest,
    ProviderResponseContract, RunRecord, RunStatus, RuntimeProfile, ScalarValue, SystemDefinition,
    VersionId, VersionRef,
};
use earmark_exec::{
    default_provider_registry, engine::ExecutionEngine, ir::ExecutionIr, provide_with_registry,
    provider_record_from_response, state::ExecutionState, ExecutionTransition,
    ProviderExecutionOutcome, ProviderFailure, ProviderFailureKind, ProviderService,
    WorkflowRunRequest,
};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;
use tempfile::tempdir;

fn setup_env() -> (GitCanonicalStore, DerivedIndex) {
    let dir = tempdir().unwrap();
    let root = dir.keep();
    let store = GitCanonicalStore::new(root.clone());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(&root).unwrap();
    (store, index)
}

fn mock_profile(allowed_ops: Vec<&str>) -> ProviderProfile {
    ProviderProfile {
        name: "test-profile".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "echo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: ProviderBudget {
            max_input_tokens: None,
            max_output_tokens: None,
            max_cost_usd: None,
            max_latency_ms: None,
        },
        allowed_operations: allowed_ops.into_iter().map(|s| s.to_string()).collect(),
        exposure: ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: ProviderResponseContract {
            format: "markdown".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    }
}

fn mock_request() -> ProviderRequest {
    ProviderRequest {
        request_id: "req_1".to_string(),
        run_id: "run_1".to_string(),
        work_packet: ObjectRef::new(ObjectId::new(), VersionId::new(), Kind::WorkPacket, None),
        provider_profile: VersionRef::new(ObjectId::new(), VersionId::new()),
        instruction_text: "Do something".to_string(),
        context_text: None,
        input_text: "Do something".to_string(),
        work_surface_manifest: None,
        inputs: vec![],
        response_contract: ProviderResponseContract {
            format: "markdown".to_string(),
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        issued_at: chrono::Utc::now(),
    }
}

#[test]
fn test_allowed_operations_blocking() {
    let registry = default_provider_registry();
    let request = mock_request();

    // 1. Disallowed operation
    let profile = mock_profile(vec!["transform"]);
    let result = provide_with_registry(&registry, &profile, request.clone(), "export");
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        ProviderFailureKind::ForbiddenOperation
    );

    // 2. Allowed operation
    let result = provide_with_registry(&registry, &profile, request.clone(), "transform");
    assert!(result.is_ok());

    // 3. Empty allowed_operations (should block everything)
    let profile_empty = mock_profile(vec![]);
    let result = provide_with_registry(&registry, &profile_empty, request.clone(), "transform");
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        ProviderFailureKind::ForbiddenOperation
    );
}

#[test]
fn test_exposure_work_surface_only_enforcement() {
    let registry = default_provider_registry();
    let mut profile = mock_profile(vec!["transform"]);
    profile.exposure.allow_work_surface_only = true;

    // 1. Request without work surface manifest
    let request_no_surface = mock_request();
    let result = provide_with_registry(&registry, &profile, request_no_surface, "transform");
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind,
        ProviderFailureKind::ForbiddenOperation
    );

    // 2. Request with work surface manifest
    let mut request_with_surface = mock_request();
    request_with_surface.work_surface_manifest = Some(".earmark/manifest.json".to_string());
    let result = provide_with_registry(&registry, &profile, request_with_surface, "transform");
    assert!(result.is_ok());
}

#[test]
fn test_advisory_warnings_for_unmeasurable_fields() {
    let registry = default_provider_registry();
    let mut profile = mock_profile(vec!["transform"]);

    // Set some unmeasurable/unsupported fields
    profile.exposure.allow_prose_objects = false;
    profile.budget.max_input_tokens = Some(1000);

    let outcome = provide_with_registry(&registry, &profile, mock_request(), "transform").unwrap();

    let warnings = outcome.record.advisory_warnings;
    assert!(warnings
        .iter()
        .any(|w| w.contains("allow_prose_objects is false")));
    assert!(warnings
        .iter()
        .any(|w| w.contains("max_input_tokens budget is not yet enforced")));
}

#[test]
fn test_synthetic_marking_integrity() {
    let registry = default_provider_registry();
    let profile = mock_profile(vec!["transform"]);

    // Even if a provider tries to claim production_eligible: true, the gate should force it to false for mock
    let outcome = provide_with_registry(&registry, &profile, mock_request(), "transform").unwrap();

    assert_eq!(
        outcome.record.metadata.get("synthetic"),
        Some(&ScalarValue::Bool(true))
    );
    assert_eq!(
        outcome.record.metadata.get("production_eligible"),
        Some(&ScalarValue::Bool(false))
    );
}

#[test]
fn test_work_packet_honest_constraints() {
    let (_store, _index) = setup_env();
    let request = WorkflowRunRequest {
        run_id: "run_1".to_string(),
        system_definition: VersionRef::new(ObjectId::new(), VersionId::new()),
        workflow: VersionRef::new(ObjectId::new(), VersionId::new()),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };
    let transition = ExecutionTransition {
        id: "t1".to_string(),
        operation: "transform".to_string(),
        input_contracts: vec![],
        output_contracts: vec![],
        instruction: None,
        compiled_context: None,
        policy: None,
        provider_profile: None,
    };
    let manifest = earmark_connected_context::WorkSurfaceManifest {
        surface_id: "s1".to_string(),
        compiled_context: VersionRef::new(ObjectId::new(), VersionId::new()),
        work_packet: None,
        generated_at: chrono::Utc::now(),
        objects: vec![],
        boundary_relations: vec![],
        constraints: BTreeMap::new(),
        warnings: vec![],
    };

    let work_packet = earmark_exec::helpers::work_packet_from_compiled_context(
        &request,
        &transition,
        &manifest,
        earmark_core::WorkPacketConstraints {
            standing_requirements: BTreeMap::new(),
            review_requirements: vec![],
            prohibited_operations: vec![],
            export_permitted: true, // This is what we pass, but we want to see it forced in some paths or at least know the engine sets it false by default elsewhere.
        },
        vec![],
    );

    // VERIFY: the helper preserves what it's given.
    // This test ensures the WorkPacket structure is correctly populated.
    assert!(work_packet.constraints.export_permitted);
    assert_eq!(work_packet.advisory_warnings.len(), 0); // No warnings added by default helper
}

#[test]
fn test_advisory_warnings_for_response_contract() {
    let registry = default_provider_registry();
    let mut profile = mock_profile(vec!["transform"]);

    // Set unsupported response contract flags
    profile.response_contract.must_include_lineage = true;
    profile.response_contract.must_return_candidate_only = false;

    let outcome = provide_with_registry(&registry, &profile, mock_request(), "transform").unwrap();

    let warnings = outcome.record.advisory_warnings;
    assert!(warnings
        .iter()
        .any(|w| w.contains("must_include_lineage is true")));
    assert!(warnings
        .iter()
        .any(|w| w.contains("must_return_candidate_only is false")));
}

#[test]
fn test_transition_enforces_honest_work_packet_defaults() {
    let (store, index) = setup_env();
    let registry = default_provider_registry();
    let engine = ExecutionEngine::new(&store, &index, &registry);

    // 1. Setup minimal objects
    let class_def = earmark_core::ClassDefinition {
        name: "candidate_output".to_string(),
        version: "1".to_string(),
        kind: "class".to_string(),
        required_headers: vec![],
        payload_schema: earmark_core::JsonSchemaRef("{}".to_string()),
        standing_rules: earmark_core::ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let class_obj = StoredObject::new(
        Kind::Object,
        Some("candidate_output".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&class_def).unwrap()),
        vec![],
    );
    let class_ref = store.write_object(&class_obj).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let system = SystemDefinition {
        system_id: "sys".to_string(),
        namespace: "ns".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![class_ref],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "test".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized".to_string(),
        },
        activated_at: None,
    };

    let ir = ExecutionIr {
        transitions: vec![ExecutionTransition {
            id: "t1".to_string(),
            operation: "compile_context".to_string(),
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: None,
            compiled_context: Some(VersionRef::new(
                earmark_core::ObjectId::parse("obj_00000000000000000000000000000001").unwrap(),
                earmark_core::VersionId::parse("ver_00000000000000000000000000000001").unwrap(),
            )),
            policy: None,
            provider_profile: None,
        }],
        guards: vec![],
        edges: vec![],
    };

    // Store a dummy template for resolution
    let template = earmark_core::CompiledContextTemplate {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        select: earmark_core::CompiledContextSelect {
            classes: vec![],
            standing: BTreeMap::new(),
            relations: vec![],
            time_range: None,
            expansion: earmark_core::CompiledContextExpansion {
                object_filter: earmark_core::ExpansionObjectFilter::Inherit,
                include_boundary_relations: false,
            },
        },
        group_by: vec![],
        render: earmark_core::CompiledContextRender {
            mode: "work_surface_materialization".to_string(),
            manifest_format: Some("json".to_string()),
            prose_template: None,
        },
        visibility: earmark_core::CompiledContextVisibility {
            include_lineage: true,
            include_constraints: true,
            include_provenance: true,
        },
    };
    let template_obj = StoredObject::new(
        Kind::CompiledContextTemplate,
        Some("test_template".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&template).unwrap()),
        vec![],
    );
    let template_ref = store.write_object(&template_obj).unwrap();

    // Update IR with real ref
    let mut ir_fixed = ir.clone();
    ir_fixed.transitions[0].compiled_context = Some(template_ref.clone());
    index.rebuild_from_store(&store).unwrap();

    let request = WorkflowRunRequest {
        run_id: "run_1".to_string(),
        system_definition: VersionRef::new(
            earmark_core::ObjectId::parse("obj_00000000000000000000000000000002").unwrap(),
            earmark_core::VersionId::parse("ver_00000000000000000000000000000002").unwrap(),
        ),
        workflow: VersionRef::new(
            earmark_core::ObjectId::parse("obj_00000000000000000000000000000003").unwrap(),
            earmark_core::VersionId::parse("ver_00000000000000000000000000000003").unwrap(),
        ),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let mut record = RunRecord {
        run_id: request.run_id.clone(),
        system_definition: request.system_definition.clone(),
        workflow: request.workflow.clone(),
        status: RunStatus::Running,
        started_at: chrono::Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        work_packets: vec![],
        governance_events: vec![],
        assignments: vec![],
        change_sets: vec![],
        manifests: vec![],
    };

    let mut active_objects = vec![];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = None;

    let mut state = ExecutionState {
        active_objects: &mut active_objects,
        emitted_packets: &mut emitted_packets,
        emitted_objects: &mut emitted_objects,
        governance_events: &mut governance_events,
        compiled_context: &mut compiled_context,
    };

    // 2. Execute transition
    engine
        .execute_transition(
            &request,
            &system,
            &ir_fixed,
            &ir_fixed.transitions[0],
            &mut state,
            &mut record,
            &DEFAULT_COMPILED_CONTEXT_COMPILER,
        )
        .unwrap();

    // 3. VERIFY: emitted work packet must have export_permitted: false
    assert_eq!(emitted_packets.len(), 1);
    let wp_ref = &emitted_packets[0];
    let wp_obj = store.read_version(&wp_ref.version_ref()).unwrap();
    let wp: earmark_core::WorkPacket = serde_json::from_slice(&wp_obj.payload.bytes).unwrap();

    // SUBSTANTIVE ASSERTION: transition.rs MUST have hardcoded false
    assert!(
        !wp.constraints.export_permitted,
        "Transition engine must enforce export_permitted: false by default"
    );
}

#[test]
fn test_transition_preserves_provider_record_warnings() {
    let (store, index) = setup_env();

    struct WarningProvider;
    impl ProviderService for WarningProvider {
        fn provide(
            &self,
            _profile: &ProviderProfile,
            request: ProviderRequest,
            _transition_operation: &str,
        ) -> Result<ProviderExecutionOutcome, ProviderFailure> {
            let response = earmark_core::ProviderResponse {
                request_id: request.request_id.clone(),
                provider: "mock".to_string(),
                model: "warn".to_string(),
                status: "ok".to_string(),
                candidate_payload: "{}".to_string(),
                metadata: BTreeMap::new(),
                advisory_warnings: vec!["Provider-level warning".to_string()],
                usage: None,
                received_at: chrono::Utc::now(),
            };
            // Mock the validation gate warning as well
            let mut record =
                provider_record_from_response(&request, &mock_profile(vec![]), &response, None);
            record.advisory_warnings = vec![
                "Policy-level warning".to_string(),
                "Provider-level warning".to_string(),
            ];
            Ok(ProviderExecutionOutcome {
                response: Some(response),
                record,
            })
        }
    }

    let engine = ExecutionEngine::new(&store, &index, &WarningProvider);

    // Setup minimal environment for transform
    let class_def_2 = earmark_core::ClassDefinition {
        name: "candidate_output".to_string(),
        version: "1".to_string(),
        kind: "class".to_string(),
        required_headers: vec![],
        payload_schema: earmark_core::JsonSchemaRef("{}".to_string()),
        standing_rules: earmark_core::ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let class_obj_2 = StoredObject::new(
        Kind::Object,
        Some("candidate_output".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&class_def_2).unwrap()),
        vec![],
    );
    let class_ref_2 = store.write_object(&class_obj_2).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let system = SystemDefinition {
        system_id: "sys".to_string(),
        namespace: "ns".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![class_ref_2],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "test".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized".to_string(),
        },
        activated_at: None,
    };

    let instruction = earmark_core::InstructionPayload {
        name: "test".to_string(),
        version: "1".to_string(),
        purpose: "test".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        execution_policy: "runtime_permitted".to_string(),
        provider_profile: None,
        trace_policy: "full".to_string(),
        register: "machined".to_string(),
        body: earmark_core::MarkdownBody::new("test".to_string()),
    };
    let instr_obj = StoredObject::new(
        Kind::Instruction,
        Some("test_instr".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown(instruction.to_markdown().unwrap()),
        vec![],
    );
    let instr_ref = store.write_object(&instr_obj).unwrap();

    let profile = ProviderProfile {
        name: "mock".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "warn".to_string(),
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
            allow_export_requests: true,
        },
        response_contract: earmark_core::ProviderResponseContract {
            format: "json".to_string(),
            must_return_candidate_only: false,
            must_include_lineage: false,
        },
        http: None,
    };
    let profile_obj = StoredObject::new(
        Kind::ProviderProfile,
        Some("mock_profile".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&profile).unwrap()),
        vec![],
    );
    let profile_ref = store.write_object(&profile_obj).unwrap();
    index.rebuild_from_store(&store).unwrap();

    let ir = ExecutionIr {
        transitions: vec![ExecutionTransition {
            id: "t1".to_string(),
            operation: "transform".to_string(),
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: Some(instr_ref),
            compiled_context: None,
            policy: None,
            provider_profile: Some(profile_ref),
        }],
        guards: vec![],
        edges: vec![],
    };

    let request = WorkflowRunRequest {
        run_id: "run_1".to_string(),
        system_definition: VersionRef::new(
            earmark_core::ObjectId::parse("obj_00000000000000000000000000000004").unwrap(),
            earmark_core::VersionId::parse("ver_00000000000000000000000000000004").unwrap(),
        ),
        workflow: VersionRef::new(
            earmark_core::ObjectId::parse("obj_00000000000000000000000000000005").unwrap(),
            earmark_core::VersionId::parse("ver_00000000000000000000000000000005").unwrap(),
        ),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let mut record = RunRecord {
        run_id: request.run_id.clone(),
        system_definition: request.system_definition.clone(),
        workflow: request.workflow.clone(),
        status: RunStatus::Running,
        started_at: chrono::Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        work_packets: vec![],
        governance_events: vec![],
        assignments: vec![],
        change_sets: vec![],
        manifests: vec![],
    };

    let mut active_objects = vec![];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "test".to_string(),
        compiled_context: VersionRef::new(
            earmark_core::ObjectId::parse("obj_00000000000000000000000000000006").unwrap(),
            earmark_core::VersionId::parse("ver_00000000000000000000000000000006").unwrap(),
        ),
        work_packet: None,
        generated_at: chrono::Utc::now(),
        objects: vec![],
        boundary_relations: vec![],
        constraints: BTreeMap::new(),
        warnings: vec![],
    });

    let mut state = ExecutionState {
        active_objects: &mut active_objects,
        emitted_packets: &mut emitted_packets,
        emitted_objects: &mut emitted_objects,
        governance_events: &mut governance_events,
        compiled_context: &mut compiled_context,
    };

    // 2. Execute transition
    engine
        .execute_transition(
            &request,
            &system,
            &ir,
            &ir.transitions[0],
            &mut state,
            &mut record,
            &DEFAULT_COMPILED_CONTEXT_COMPILER,
        )
        .unwrap();

    // 3. VERIFY: governance event for provider record MUST have all warnings
    assert!(!governance_events.is_empty());
    let event_ref = &governance_events[0];
    let event_obj = store.read_version(&event_ref.version_ref()).unwrap();
    let pr: earmark_core::ProviderRecord =
        serde_json::from_slice(&event_obj.payload.bytes).unwrap();

    // SUBSTANTIVE ASSERTIONS: verify both warnings are preserved
    assert!(pr
        .advisory_warnings
        .contains(&"Policy-level warning".to_string()));
    assert!(pr
        .advisory_warnings
        .contains(&"Provider-level warning".to_string()));
}
