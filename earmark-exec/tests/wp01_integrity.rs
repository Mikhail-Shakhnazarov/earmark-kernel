use earmark_connected_context::DEFAULT_COMPILED_CONTEXT_COMPILER;
use earmark_core::{Kind, ObjectId, RunRecord, WorkflowOperationKind};
use earmark_exec::engine::ExecutionEngine;
use earmark_exec::helpers::store_work_packet;
use earmark_exec::ir::{ExecutionIr, ExecutionTransition, WorkflowRunRequest};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_exec::provider::{ProviderExecutionOutcome, ProviderService};
use earmark_exec::state::ExecutionState;
use earmark_index::{DerivedIndex, QueryFilter};
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, WorkspaceLayout};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_immediate_index_visibility_in_transition() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();

    let index = DerivedIndex::open(root).unwrap();

    // 1. Verify TransitionAssignment indexing
    let assignment_id = earmark_core::TransitionAssignmentId::new();
    let assignment = earmark_core::TransitionAssignment {
        id: assignment_id.clone(),
        run_id: ObjectId::new().to_string(),
        transition_id: "t1".to_string(),
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
    };

    let stored_assignment = StoredObject::new(
        Kind::TransitionAssignment,
        Some("transition_assignment".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        earmark_store::StoredPayload::from_json_bytes(
            serde_json::to_vec_pretty(&assignment).unwrap(),
        ),
        vec![],
    );

    write_object_and_index(&store, &index, &stored_assignment).unwrap();

    let heads = index
        .query_objects(&QueryFilter {
            class: Some("transition_assignment".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(heads
        .iter()
        .any(|h| h.object_id == stored_assignment.envelope.id.as_str()));

    // 2. Verify Event indexing (ProviderRecord)
    let event = StoredObject::new(
        Kind::Event,
        Some("provider_record".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        earmark_store::StoredPayload::from_json_bytes(vec![1, 2, 3]),
        vec![],
    );

    write_object_and_index(&store, &index, &event).unwrap();
    let heads = index
        .query_objects(&QueryFilter {
            class: Some("provider_record".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(heads
        .iter()
        .any(|h| h.object_id == event.envelope.id.as_str()));

    // 3. Verify HandoffManifest indexing
    let handoff = StoredObject::new(
        Kind::HandoffManifest,
        Some("handoff_manifest".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        earmark_store::StoredPayload::from_json_bytes(vec![4, 5, 6]),
        vec![],
    );

    write_object_and_index(&store, &index, &handoff).unwrap();
    let heads = index
        .query_objects(&QueryFilter {
            class: Some("handoff_manifest".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(heads
        .iter()
        .any(|h| h.object_id == handoff.envelope.id.as_str()));

    // 4. Verify store_work_packet indexing
    let work_packet = earmark_core::WorkPacket {
        work_packet_id: "wp1".to_string(),
        run_id: ObjectId::new().to_string(),
        work_packet_type: "test".to_string(),
        purpose: "verification".to_string(),
        system_definition: earmark_core::VersionRef::new(
            ObjectId::new(),
            earmark_core::VersionId::new(),
        ),
        workflow: None,
        instruction: None,
        provider_profile: None,
        inputs: vec![],
        compiled_contexts: vec![],
        constraints: earmark_core::WorkPacketConstraints {
            standing_requirements: BTreeMap::new(),
            review_requirements: vec![],
            prohibited_operations: vec![],
            export_permitted: true,
        },
        expected_outputs: vec![],
        work_surface: None,
        advisory_warnings: vec![],
        created_at: chrono::Utc::now(),
    };

    let wp_stored = store_work_packet(&store, &index, &work_packet).unwrap();
    let heads = index
        .query_objects(&QueryFilter {
            class: Some("work_packet".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(heads
        .iter()
        .any(|h| h.object_id == wp_stored.envelope.id.as_str()));
}

#[test]
fn test_live_transition_indexing() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();

    struct MockProvider;
    impl ProviderService for MockProvider {
        fn provide(
            &self,
            _: &earmark_core::ProviderProfile,
            _: earmark_core::ProviderRequest,
            _: &str,
        ) -> Result<ProviderExecutionOutcome, earmark_exec::error::ProviderFailure> {
            panic!(
                "MockProvider::provide should never be called in this test; \
                 provide is only invoked for 'transform' transitions with a Delegated provider mode, \
                 but this test only exercises 'review' transitions"
            )
        }
    }
    let engine = ExecutionEngine::new(&store, &index, &MockProvider);

    let run_id = ObjectId::new();
    let mut record = RunRecord {
        run_id: run_id.to_string(),
        system_definition: earmark_core::VersionRef::new(
            ObjectId::new(),
            earmark_core::VersionId::new(),
        ),
        workflow: earmark_core::VersionRef::new(ObjectId::new(), earmark_core::VersionId::new()),
        status: earmark_core::RunStatus::Running,
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

    // Pre-create an input object so validation passes
    let input_obj = StoredObject::new(
        Kind::Object,
        Some("test".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        earmark_store::StoredPayload::from_json_bytes(vec![1, 2, 3]),
        vec![],
    );
    store.write_object(&input_obj).unwrap();

    let mut active_objects = vec![input_obj.object_ref()];
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

    let ir = ExecutionIr {
        transitions: vec![ExecutionTransition {
            id: "t1".to_string(),
            operation: WorkflowOperationKind::Review,
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: None,
            compiled_context: None,
            policy: None,
            provider_profile: None,
        }],
        guards: vec![],
        edges: vec![],
    };

    let request = WorkflowRunRequest {
        run_id: run_id.to_string(),
        system_definition: earmark_core::VersionRef::new(
            ObjectId::new(),
            earmark_core::VersionId::new(),
        ),
        workflow: earmark_core::VersionRef::new(ObjectId::new(), earmark_core::VersionId::new()),
        inputs: vec![],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let system = earmark_core::SystemDefinition {
        title: "test".to_string(),
        system_id: "s1".to_string(),
        namespace: "n1".to_string(),
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
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "test".to_string(),
            machine_output_default: "test".to_string(),
            work_surface_mode: "test".to_string(),
        },
        activated_at: None,
    };

    let transition = &ir.transitions[0];

    engine
        .execute_transition(
            &request,
            &system,
            &ir,
            transition,
            &mut state,
            &mut record,
            &DEFAULT_COMPILED_CONTEXT_COMPILER,
        )
        .unwrap();

    // 1. Verify TransitionAssignment created by execute_transition is indexed
    let assignments = index
        .query_objects(&QueryFilter {
            class: Some("transition_assignment".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(
        !assignments.is_empty(),
        "TransitionAssignment should be indexed"
    );

    // 2. Verify Review created by execute_transition is indexed
    let reviews = index
        .query_objects(&QueryFilter {
            class: Some("review".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(!reviews.is_empty(), "Review should be indexed");

    // 3. Verify HandoffManifest created by execute_transition is indexed
    let manifests = index
        .query_objects(&QueryFilter {
            class: Some("handoff_manifest".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(!manifests.is_empty(), "HandoffManifest should be indexed");
}
