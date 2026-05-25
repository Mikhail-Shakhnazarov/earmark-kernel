use chrono::Utc;
use earmark_connected_context::DEFAULT_COMPILED_CONTEXT_COMPILER;
use earmark_core::{
    to_yaml, Kind, ObjectId, ProviderBudget, ProviderExposure, ProviderProfile, ProviderRequest,
    ProviderResponse, ProviderResponseContract, ProviderUsage, RunRecord, SystemDefinition,
    VersionId, VersionRef, WorkflowOperationKind,
};
use earmark_exec::{
    engine::ExecutionEngine, ir::ExecutionIr, provider_record_from_response, state::ExecutionState,
    ExecutionTransition, ProviderExecutionOutcome, ProviderFailureKind, ProviderService,
    WorkflowRunRequest,
};
use earmark_index::DerivedIndex;
use earmark_store::{
    GitCanonicalStore, ObjectStore, StoreScanner, StoredObject, StoredPayload, WorkspaceLayout,
};
use std::collections::BTreeMap;
use tempfile::tempdir;

struct BudgetTestProvider {
    output_text: String,
    cost_usd: Option<f32>,
}

impl ProviderService for BudgetTestProvider {
    fn provide(
        &self,
        profile: &ProviderProfile,
        request: ProviderRequest,
        _op: &str,
    ) -> Result<ProviderExecutionOutcome, earmark_exec::error::ProviderFailure> {
        let response = ProviderResponse {
            request_id: request.request_id.clone(),
            provider: profile.provider.clone(),
            model: profile.model.clone(),
            status: earmark_core::ProviderResponseStatus::Completed,
            candidate_payload: self.output_text.clone(),
            usage: Some(ProviderUsage {
                input_tokens: Some(0),
                output_tokens: Some(0),
                estimated_cost_usd: self.cost_usd,
                latency_ms: Some(0),
            }),
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
            received_at: chrono::Utc::now(),
        };
        let record = provider_record_from_response(&request, profile, &response, None);
        Ok(ProviderExecutionOutcome {
            response: Some(response),
            record,
        })
    }
}

fn setup_env() -> (GitCanonicalStore, DerivedIndex) {
    let dir = tempdir().unwrap();
    let root = dir.keep();
    let store = GitCanonicalStore::new(root.clone());
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(&root).unwrap();
    (store, index)
}

fn mock_profile(budget: ProviderBudget) -> ProviderProfile {
    ProviderProfile {
        name: "test-profile".to_string(),
        version: "1".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "echo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget,
        allowed_operations: vec!["transform".to_string()],
        exposure: ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: ProviderResponseContract {
            format: earmark_core::ProviderResponseFormat::Markdown,
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    }
}

#[test]
fn test_input_budget_enforcement() {
    let (store, mut index) = setup_env();
    let provider = BudgetTestProvider {
        output_text: "ok".to_string(),
        cost_usd: None,
    };
    let mut engine = ExecutionEngine::new(&store, &mut index, &provider);

    let budget = ProviderBudget {
        max_input_tokens: Some(2),
        max_output_tokens: None,
        max_cost_usd: None,
        max_latency_ms: None,
    };
    let profile = mock_profile(budget);
    let profile_ref = store
        .write_object(&StoredObject::new(
            Kind::ProviderProfile,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&profile).unwrap()),
            vec![],
        ))
        .unwrap();

    let system = SystemDefinition {
        system_id: "s".to_string(),
        namespace: "n".to_string(),
        title: "t".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: Some(profile_ref.clone()),
        standing_dimensions: vec![],
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "t".to_string(),
            machine_output_default: "t".to_string(),
            work_surface_mode: "t".to_string(),
        },
        activated_at: None,
    };

    let instr_text = r#"---
name: i
version: 1
purpose: p
input_classes: []
output_classes: []
execution_policy: default
trace_policy: default
register: ""
---
Over budget input text"#;

    let instr_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr_text),
            vec![],
        ))
        .unwrap();

    let transition = ExecutionTransition {
        id: earmark_core::TransitionId::parse("t1").unwrap(),
        operation: WorkflowOperationKind::Transform,
        input_contracts: vec![],
        output_contracts: vec!["out".to_string()],
        instruction: Some(instr_ref),
        compiled_context: None,
        policy: None,
        provider_profile: Some(profile_ref),
    };

    let mut record = RunRecord {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        status: earmark_core::RunStatus::Running,
        started_at: Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        assignments: vec![],
        change_sets: vec![],
        work_packets: vec![],
        governance_events: vec![],
        manifests: vec![],
    };

    let mut active_objects = vec![];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "test".to_string(),
        compiled_context: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        work_packet: None,
        generated_at: Utc::now(),
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

    let request = WorkflowRunRequest {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let result = engine.execute_transition(
        &request,
        &system,
        &ExecutionIr {
            transitions: vec![transition.clone()],
            guards: vec![],
            edges: vec![],
        },
        &transition,
        &mut state,
        &mut record,
        &DEFAULT_COMPILED_CONTEXT_COMPILER,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        earmark_exec::error::ExecError::Provider(f) => {
            assert_eq!(f.kind, ProviderFailureKind::BudgetExceeded);
            assert!(f.message.contains("estimated input tokens"));
        }
        _ => panic!("Expected Provider error, got {:?}", err),
    }
}

#[test]
fn test_output_budget_enforcement() {
    let (store, mut index) = setup_env();
    let provider = BudgetTestProvider {
        output_text: "This is a very long output that should exceed the budget".to_string(),
        cost_usd: None,
    };
    let mut engine = ExecutionEngine::new(&store, &mut index, &provider);

    let budget = ProviderBudget {
        max_input_tokens: None,
        max_output_tokens: Some(5),
        max_cost_usd: None,
        max_latency_ms: None,
    };
    let profile = mock_profile(budget);
    let profile_ref = store
        .write_object(&StoredObject::new(
            Kind::ProviderProfile,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&profile).unwrap()),
            vec![],
        ))
        .unwrap();

    let system = SystemDefinition {
        system_id: "s".to_string(),
        namespace: "n".to_string(),
        title: "t".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: Some(profile_ref.clone()),
        standing_dimensions: vec![],
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "t".to_string(),
            machine_output_default: "t".to_string(),
            work_surface_mode: "t".to_string(),
        },
        activated_at: None,
    };

    let instr_text = r#"---
name: i
version: 1
purpose: p
input_classes: []
output_classes: []
execution_policy: default
trace_policy: default
register: ""
---
Short instruction"#;

    let instr_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr_text),
            vec![],
        ))
        .unwrap();

    let transition = ExecutionTransition {
        id: earmark_core::TransitionId::parse("t1").unwrap(),
        operation: WorkflowOperationKind::Transform,
        input_contracts: vec![],
        output_contracts: vec!["out".to_string()],
        instruction: Some(instr_ref),
        compiled_context: None,
        policy: None,
        provider_profile: Some(profile_ref),
    };

    let mut record = RunRecord {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        status: earmark_core::RunStatus::Running,
        started_at: Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        assignments: vec![],
        change_sets: vec![],
        work_packets: vec![],
        governance_events: vec![],
        manifests: vec![],
    };

    let mut active_objects = vec![];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "test".to_string(),
        compiled_context: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        work_packet: None,
        generated_at: Utc::now(),
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

    let request = WorkflowRunRequest {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let result = engine.execute_transition(
        &request,
        &system,
        &ExecutionIr {
            transitions: vec![transition.clone()],
            guards: vec![],
            edges: vec![],
        },
        &transition,
        &mut state,
        &mut record,
        &DEFAULT_COMPILED_CONTEXT_COMPILER,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        earmark_exec::error::ExecError::Provider(f) => {
            assert_eq!(f.kind, ProviderFailureKind::BudgetExceeded);
            assert!(f.message.contains("estimated output tokens"));
        }
        _ => panic!("Expected Provider error, got {:?}", err),
    }
}

#[test]
fn test_cost_budget_enforcement() {
    let (store, mut index) = setup_env();
    let provider = BudgetTestProvider {
        output_text: "ok".to_string(),
        cost_usd: Some(1.0f32),
    };
    let mut engine = ExecutionEngine::new(&store, &mut index, &provider);

    let budget = ProviderBudget {
        max_input_tokens: None,
        max_output_tokens: None,
        max_cost_usd: Some(0.5f32),
        max_latency_ms: None,
    };
    let profile = mock_profile(budget);
    let profile_ref = store
        .write_object(&StoredObject::new(
            Kind::ProviderProfile,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&profile).unwrap()),
            vec![],
        ))
        .unwrap();

    let system = SystemDefinition {
        system_id: "s".to_string(),
        namespace: "n".to_string(),
        title: "t".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: Some(profile_ref.clone()),
        standing_dimensions: vec![],
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "t".to_string(),
            machine_output_default: "t".to_string(),
            work_surface_mode: "t".to_string(),
        },
        activated_at: None,
    };

    let instr_text = r#"---
name: i
version: 1
purpose: p
input_classes: []
output_classes: []
execution_policy: default
trace_policy: default
register: ""
---
Short instruction"#;

    let instr_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr_text),
            vec![],
        ))
        .unwrap();

    let transition = ExecutionTransition {
        id: earmark_core::TransitionId::parse("t1").unwrap(),
        operation: WorkflowOperationKind::Transform,
        input_contracts: vec![],
        output_contracts: vec!["out".to_string()],
        instruction: Some(instr_ref),
        compiled_context: None,
        policy: None,
        provider_profile: Some(profile_ref),
    };

    let mut record = RunRecord {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        status: earmark_core::RunStatus::Running,
        started_at: Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        assignments: vec![],
        change_sets: vec![],
        work_packets: vec![],
        governance_events: vec![],
        manifests: vec![],
    };

    let mut active_objects = vec![];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "test".to_string(),
        compiled_context: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        work_packet: None,
        generated_at: Utc::now(),
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

    let request = WorkflowRunRequest {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let result = engine.execute_transition(
        &request,
        &system,
        &ExecutionIr {
            transitions: vec![transition.clone()],
            guards: vec![],
            edges: vec![],
        },
        &transition,
        &mut state,
        &mut record,
        &DEFAULT_COMPILED_CONTEXT_COMPILER,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        earmark_exec::error::ExecError::Provider(f) => {
            assert_eq!(f.kind, ProviderFailureKind::BudgetExceeded);
            assert!(f.message.contains("estimated cost"));
        }
        _ => panic!("Expected Provider error, got {:?}", err),
    }
}

#[test]
fn test_budget_within_limits() {
    let (store, mut index) = setup_env();
    let provider = BudgetTestProvider {
        output_text: "ok".to_string(),
        cost_usd: Some(0.1f32),
    };
    let mut engine = ExecutionEngine::new(&store, &mut index, &provider);

    let budget = ProviderBudget {
        max_input_tokens: Some(100),
        max_output_tokens: Some(100),
        max_cost_usd: Some(1.0f32),
        max_latency_ms: None,
    };
    let profile = mock_profile(budget);
    let profile_ref = store
        .write_object(&StoredObject::new(
            Kind::ProviderProfile,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&profile).unwrap()),
            vec![],
        ))
        .unwrap();

    let system = SystemDefinition {
        system_id: "s".to_string(),
        namespace: "n".to_string(),
        title: "t".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: Some(profile_ref.clone()),
        standing_dimensions: vec![],
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "t".to_string(),
            machine_output_default: "t".to_string(),
            work_surface_mode: "t".to_string(),
        },
        activated_at: None,
    };

    let instr_text = r#"---
name: i
version: 1
purpose: p
input_classes: []
output_classes: []
execution_policy: default
trace_policy: default
register: ""
---
Short instruction"#;

    let instr_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr_text),
            vec![],
        ))
        .unwrap();

    let transition = ExecutionTransition {
        id: earmark_core::TransitionId::parse("t1").unwrap(),
        operation: WorkflowOperationKind::Transform,
        input_contracts: vec![],
        output_contracts: vec!["out".to_string()],
        instruction: Some(instr_ref),
        compiled_context: None,
        policy: None,
        provider_profile: Some(profile_ref),
    };

    let mut record = RunRecord {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        status: earmark_core::RunStatus::Running,
        started_at: Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        assignments: vec![],
        change_sets: vec![],
        work_packets: vec![],
        governance_events: vec![],
        manifests: vec![],
    };

    let mut active_objects = vec![];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "test".to_string(),
        compiled_context: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        work_packet: None,
        generated_at: Utc::now(),
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

    let request = WorkflowRunRequest {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let result = engine.execute_transition(
        &request,
        &system,
        &ExecutionIr {
            transitions: vec![transition.clone()],
            guards: vec![],
            edges: vec![],
        },
        &transition,
        &mut state,
        &mut record,
        &DEFAULT_COMPILED_CONTEXT_COMPILER,
    );

    assert!(result.is_ok());
}

#[test]
fn test_missing_cost_metadata_warning() {
    let (store, mut index) = setup_env();
    let provider = BudgetTestProvider {
        output_text: "ok".to_string(),
        cost_usd: None,
    };
    let mut engine = ExecutionEngine::new(&store, &mut index, &provider);

    let budget = ProviderBudget {
        max_input_tokens: None,
        max_output_tokens: None,
        max_cost_usd: Some(1.0f32),
        max_latency_ms: None,
    };
    let profile = mock_profile(budget);
    let profile_ref = store
        .write_object(&StoredObject::new(
            Kind::ProviderProfile,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&profile).unwrap()),
            vec![],
        ))
        .unwrap();

    let system = SystemDefinition {
        system_id: "s".to_string(),
        namespace: "n".to_string(),
        title: "t".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: Some(profile_ref.clone()),
        standing_dimensions: vec![],
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "t".to_string(),
            machine_output_default: "t".to_string(),
            work_surface_mode: "t".to_string(),
        },
        activated_at: None,
    };

    let instr_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            earmark_core::Standing::default(),
            earmark_core::Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_markdown("---\nname: i\nversion: 1\npurpose: p\ninput_classes: []\noutput_classes: []\nexecution_policy: default\ntrace_policy: default\nregister: \"\"\n---\ni"),
            vec![],
        ))
        .unwrap();

    let transition = ExecutionTransition {
        id: earmark_core::TransitionId::parse("t1").unwrap(),
        operation: WorkflowOperationKind::Transform,
        input_contracts: vec![],
        output_contracts: vec!["out".to_string()],
        instruction: Some(instr_ref),
        compiled_context: None,
        policy: None,
        provider_profile: Some(profile_ref),
    };

    let mut record = RunRecord {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        status: earmark_core::RunStatus::Running,
        started_at: Utc::now(),
        ended_at: None,
        initial_marking: vec![],
        final_marking: vec![],
        events: vec![],
        assignments: vec![],
        change_sets: vec![],
        work_packets: vec![],
        governance_events: vec![],
        manifests: vec![],
    };

    let mut active_objects = vec![];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "test".to_string(),
        compiled_context: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        work_packet: None,
        generated_at: Utc::now(),
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

    let request = WorkflowRunRequest {
        run_id: earmark_core::RunId::parse("run_1").unwrap(),
        system_definition: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        workflow: VersionRef::new(ObjectId::generate(), VersionId::generate()),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let result = engine.execute_transition(
        &request,
        &system,
        &ExecutionIr {
            transitions: vec![transition.clone()],
            guards: vec![],
            edges: vec![],
        },
        &transition,
        &mut state,
        &mut record,
        &DEFAULT_COMPILED_CONTEXT_COMPILER,
    );

    assert!(result.is_ok());

    // Check for warning in the record
    let _provider_event = record
        .governance_events
        .iter()
        .find(|e| e.class == Some("provider_record".to_string()))
        .unwrap();
    // We need to check the actual ProviderRecord object
    // In our test setup, emitted_objects contains the ProviderRecord (indirectly)
    // Actually, record.events has metadata? No.
    // I'll check emitted_objects for Kind::ProviderRecord if it exists.
    // Wait, ProviderRecord is NOT an object in the store usually?
    // Yes, record_provider_event writes it.

    let objects = store.scan_objects().unwrap().scanned_objects;
    let provider_rec_obj = objects
        .iter()
        .find(|obj| obj.envelope.class == Some("provider_record".to_string()))
        .unwrap();
    let provider_rec: earmark_core::ProviderRecord =
        serde_json::from_slice(&provider_rec_obj.payload.bytes).unwrap();

    assert!(provider_rec
        .advisory_warnings
        .iter()
        .any(|w| w.contains("Budget not enforceable")));
}
