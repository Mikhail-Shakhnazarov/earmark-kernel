use earmark_core::{
    WorkflowDefinition, WorkflowOperation, VersionRef, ObjectId, VersionId,
};
use earmark_store::*;
use earmark_index::*;
use tempfile::tempdir;
use earmark_store::GitCanonicalStore;
use crate::error::ProviderFailure;
use crate::provider::{ProviderExecutionOutcome, ProviderService};


#[test]
fn test_execution_ir_compilation() {
    let workflow = WorkflowDefinition {
        name: "test_flow".to_string(),
        version: "1".to_string(),
        description: None,
        operations: vec![
            WorkflowOperation {
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
            },
        ],
        edges: vec![],
        guards: vec![],
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

    let _engine = crate::engine::ExecutionEngine::new(
        &store,
        &index,
        &registry,
    );
}

struct NoopProviderService;

impl ProviderService for NoopProviderService {
    fn provide(
        &self,
        _profile: &earmark_core::ProviderProfile,
        _request: earmark_core::ProviderRequest,
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
    assert!(boundaries.iter().any(|b| b.id == "provider_dispatch" && b.future_async_candidate));
    assert!(boundaries.iter().any(|b| b.id == "provider_http_client" && b.future_async_candidate));
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
    fn provide(&self, _profile: &earmark_core::ProviderProfile, _request: earmark_core::ProviderRequest) -> Result<ProviderExecutionOutcome, ProviderFailure> {
        Ok(ProviderExecutionOutcome {
            response: None,
            record: earmark_core::ProviderRecord {
                record_id: "prec_1".to_string(),
                request_id: "req_1".to_string(),
                run_id: "run_1".to_string(),
                work_packet: earmark_core::ObjectRef::new(ObjectId::new(), VersionId::new(), earmark_core::Kind::WorkPacket, None),
                provider_profile: VersionRef::new(ObjectId::new(), VersionId::new()),
                provider: "broken".to_string(),
                model: "broken".to_string(),
                status: "ok".to_string(),
                usage: None,
                message: None,
                recorded_at: chrono::Utc::now(),
            },
        })
    }
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

    let instr_text = format!("---\n{}---\ntest body", earmark_core::to_yaml(&instruction).unwrap());
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

    let mut active_objects = vec![earmark_core::ObjectRef::new(note_ref.id.clone(), note_ref.version_id.clone(), earmark_core::Kind::Object, Some("note".to_string()))];
    let mut emitted_packets = vec![];
    let mut emitted_objects = vec![];
    let mut governance_events = vec![];
    let mut compiled_context = Some(earmark_connected_context::WorkSurfaceManifest {
        surface_id: "surf_1".to_string(),
        compiled_context: earmark_core::VersionRef::new(ObjectId::new(), VersionId::new()),
        work_packet: Some(earmark_core::ObjectRef::new(ObjectId::new(), VersionId::new(), earmark_core::Kind::WorkPacket, None)),
        generated_at: chrono::Utc::now(),
        objects: vec![],
        constraints: std::collections::BTreeMap::new(),
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
    assert!(err_msg.contains("delegated outcome did not contain a response"), "Error message was: {}", err_msg);
}
