use earmark_core::{
    to_yaml, ClassDefinition, ClassStandingRules, CompiledContextExpansion, CompiledContextRender,
    CompiledContextSelect, CompiledContextTemplate, CompiledContextVisibility, JsonSchemaRef, Kind,
    Provenance, RuntimeProfile, Standing, SystemDefinition, VersionRef, WorkflowOperationKind,
};
use earmark_exec::{ExecutionEngine, ProviderRegistry, WorkflowRunRequest};
use earmark_index::DerivedIndex;
use earmark_store::{
    GitCanonicalStore, ObjectStore, StoreScanner, StoredObject, StoredPayload, WorkspaceLayout,
};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn guarded_edge_blocking() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let registry = ProviderRegistry::default();

    let system = SystemDefinition {
        system_id: "test-system".to_string(),
        namespace: "test".to_string(),
        title: "Test System".to_string(),
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
            execution_surface: "runtime_over_folder".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };

    let workflow = earmark_core::WorkflowDefinition {
        name: "guarded-branch".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        operations: vec![
            earmark_core::WorkflowOperation {
                id: "start_op".to_string(),
                kind: WorkflowOperationKind::Review,
                input_contracts: vec!["start".to_string()],
                output_contracts: vec!["middle".to_string()],
                instruction: None,
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
            earmark_core::WorkflowOperation {
                id: "guarded_op".to_string(),
                kind: WorkflowOperationKind::Review,
                input_contracts: vec!["middle".to_string()],
                output_contracts: vec!["end".to_string()],
                instruction: None,
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
        ],
        edges: vec![earmark_core::WorkflowEdge {
            from: "start_op".to_string(),
            to: "guarded_op".to_string(),
            condition: Some("operator_approved".to_string()),
        }],
        guards: vec![],
        output_contracts: vec![],
    };

    let system_ref = store
        .write_object(&StoredObject::new(
            Kind::SystemDefinition,
            None,
            Standing::default(),
            Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&system).unwrap()),
            vec![],
        ))
        .unwrap();

    let workflow_ref = store
        .write_object(&StoredObject::new(
            Kind::Workflow,
            None,
            Standing::default(),
            Provenance::direct_input("test"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&workflow).unwrap()),
            vec![],
        ))
        .unwrap();

    let start_obj = StoredObject::new(
        Kind::Object,
        Some("start".to_string()),
        Standing::default(),
        Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown("start"),
        vec![],
    );
    store.write_object(&start_obj).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };
    let request = WorkflowRunRequest {
        run_id: "test-run".to_string(),
        system_definition: system_ref,
        workflow: workflow_ref,
        inputs: vec![start_obj.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: false,
    };

    let outcome = engine.run_workflow(request).unwrap();

    // Verify that guarded_op was NOT executed
    let transition_ids: Vec<_> = outcome
        .record
        .events
        .iter()
        .map(|e| e.transition.as_str())
        .collect();
    assert!(transition_ids.contains(&"start_op"));
    assert!(!transition_ids.contains(&"guarded_op"));

    let objects = store.scan_objects().unwrap();
    let ledger_obj = objects
        .iter()
        .find(|obj| obj.envelope.kind == Kind::RunRecord)
        .unwrap();
    let ledger: earmark_core::RunRecord =
        serde_json::from_slice(&ledger_obj.payload.bytes).unwrap();
    let blocked_events: Vec<_> = ledger
        .events
        .iter()
        .filter(|e| e.event_type == "edge_blocked")
        .collect();
    assert!(!blocked_events.is_empty());
}

#[test]
fn branching_execution() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let registry = ProviderRegistry::default();

    let system = SystemDefinition {
        system_id: "test-system".to_string(),
        namespace: "test".to_string(),
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
            execution_surface: "runtime_over_folder".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };

    let workflow = earmark_core::WorkflowDefinition {
        name: "branch-test".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        operations: vec![
            earmark_core::WorkflowOperation {
                id: "root".to_string(),
                kind: WorkflowOperationKind::Review,
                input_contracts: vec!["start".to_string()],
                output_contracts: vec!["fork".to_string()],
                instruction: None,
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
            earmark_core::WorkflowOperation {
                id: "branch1".to_string(),
                kind: WorkflowOperationKind::Review,
                input_contracts: vec!["fork".to_string()],
                output_contracts: vec!["end1".to_string()],
                instruction: None,
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
            earmark_core::WorkflowOperation {
                id: "branch2".to_string(),
                kind: WorkflowOperationKind::Review,
                input_contracts: vec!["fork".to_string()],
                output_contracts: vec!["end2".to_string()],
                instruction: None,
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
        ],
        edges: vec![
            earmark_core::WorkflowEdge {
                from: "root".to_string(),
                to: "branch1".to_string(),
                condition: None,
            },
            earmark_core::WorkflowEdge {
                from: "root".to_string(),
                to: "branch2".to_string(),
                condition: None,
            },
        ],
        guards: vec![],
        output_contracts: vec![],
    };

    let system_ref = store
        .write_object(&StoredObject::new(
            Kind::SystemDefinition,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&system).unwrap()),
            vec![],
        ))
        .unwrap();
    let workflow_ref = store
        .write_object(&StoredObject::new(
            Kind::Workflow,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&workflow).unwrap()),
            vec![],
        ))
        .unwrap();
    let start_obj = StoredObject::new(
        Kind::Object,
        Some("start".to_string()),
        Standing::default(),
        Provenance::direct_input("t"),
        BTreeMap::new(),
        StoredPayload::from_markdown("s"),
        vec![],
    );
    store.write_object(&start_obj).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };
    let request = WorkflowRunRequest {
        run_id: "branch-run".to_string(),
        system_definition: system_ref,
        workflow: workflow_ref,
        inputs: vec![start_obj.object_ref()],
        operator_approved: true,
        handoff_manifest: None,
        transition_assignment: None,
    };

    let outcome = engine.run_workflow(request).unwrap();
    let ids: Vec<_> = outcome
        .record
        .events
        .iter()
        .map(|e| e.transition.as_str())
        .collect();
    assert!(ids.contains(&"root"));
    assert!(ids.contains(&"branch1"));
    assert!(ids.contains(&"branch2"));
}

#[test]
fn parallel_transform_leak_bug() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let registry = ProviderRegistry::default();

    // Setup compiled_context
    let compiled_context = CompiledContextTemplate {
        name: "p1".to_string(),
        version: "1".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["start_class".to_string()],
            standing: BTreeMap::new(),
            relations: vec![],
            time_range: None,
            expansion: CompiledContextExpansion::default(),
        },
        group_by: vec![],
        render: CompiledContextRender {
            mode: "work_surface_materialization".to_string(),
            manifest_format: Some("json".to_string()),
            prose_template: None,
        },
        visibility: CompiledContextVisibility {
            include_lineage: true,
            include_constraints: true,
            include_provenance: true,
        },
    };
    let proj_obj = StoredObject::new(
        Kind::CompiledContextTemplate,
        Some("compiled_context_template".to_string()),
        Standing::default(),
        Provenance::direct_input("t"),
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&compiled_context).unwrap()),
        vec![],
    );
    let proj_ref = store.write_object(&proj_obj).unwrap();

    // Setup instructions
    let instr1 = earmark_core::InstructionPayload {
        name: "i1".to_string(),
        version: "1".to_string(),
        purpose: "p1".to_string(),
        input_classes: vec!["start_class".to_string()],
        output_classes: vec!["out1".to_string()],
        execution_policy: "local".to_string(),
        provider_profile: None,
        trace_policy: "t".to_string(),
        register: "out1".to_string(),
        body: earmark_core::MarkdownBody::new("b1".to_string()),
    };
    let instr1_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr1.to_markdown().unwrap()),
            vec![],
        ))
        .unwrap();

    let instr2 = earmark_core::InstructionPayload {
        name: "i2".to_string(),
        version: "1".to_string(),
        purpose: "p2".to_string(),
        input_classes: vec!["start_class".to_string()],
        output_classes: vec!["out2".to_string()],
        execution_policy: "local".to_string(),
        provider_profile: None,
        trace_policy: "t".to_string(),
        register: "out2".to_string(),
        body: earmark_core::MarkdownBody::new("b2".to_string()),
    };
    let instr2_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr2.to_markdown().unwrap()),
            vec![],
        ))
        .unwrap();

    let workflow = earmark_core::WorkflowDefinition {
        name: "leak-test".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        operations: vec![
            earmark_core::WorkflowOperation {
                id: "project".to_string(),
                kind: WorkflowOperationKind::CompileContext,
                input_contracts: vec!["start_class".to_string()],
                output_contracts: vec!["surface".to_string()],
                instruction: None,
                compiled_context: Some(proj_ref),
                policy: None,
                provider_profile: None,
            },
            earmark_core::WorkflowOperation {
                id: "branch1".to_string(),
                kind: WorkflowOperationKind::Transform,
                input_contracts: vec!["surface".to_string()],
                output_contracts: vec!["out1".to_string()],
                instruction: Some(instr1_ref),
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
            earmark_core::WorkflowOperation {
                id: "branch2".to_string(),
                kind: WorkflowOperationKind::Transform,
                input_contracts: vec!["surface".to_string()],
                output_contracts: vec!["out2".to_string()],
                instruction: Some(instr2_ref),
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
        ],
        edges: vec![
            earmark_core::WorkflowEdge {
                from: "project".to_string(),
                to: "branch1".to_string(),
                condition: None,
            },
            earmark_core::WorkflowEdge {
                from: "project".to_string(),
                to: "branch2".to_string(),
                condition: None,
            },
        ],
        guards: vec![],
        output_contracts: vec![],
    };

    let out1_class = ClassDefinition {
        name: "out1".to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let out1_class_ref = store
        .write_object(&StoredObject::new(
            Kind::Object,
            Some("class_definition".to_string()),
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&out1_class).unwrap()),
            vec![],
        ))
        .unwrap();
    let out2_class = ClassDefinition {
        name: "out2".to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let out2_class_ref = store
        .write_object(&StoredObject::new(
            Kind::Object,
            Some("class_definition".to_string()),
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&out2_class).unwrap()),
            vec![],
        ))
        .unwrap();

    let system = SystemDefinition {
        system_id: "s".to_string(),
        namespace: "n".to_string(),
        title: "t".to_string(),
        description: None,
        classes: vec![
            VersionRef::new(out1_class_ref.id.clone(), out1_class_ref.version_id.clone()),
            VersionRef::new(out2_class_ref.id.clone(), out2_class_ref.version_id.clone()),
        ],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "r".to_string(),
            machine_output_default: "j".to_string(),
            work_surface_mode: "m".to_string(),
        },
        activated_at: None,
    };

    let system_ref = store
        .write_object(&StoredObject::new(
            Kind::SystemDefinition,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&system).unwrap()),
            vec![],
        ))
        .unwrap();
    let workflow_ref = store
        .write_object(&StoredObject::new(
            Kind::Workflow,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&workflow).unwrap()),
            vec![],
        ))
        .unwrap();
    let start_obj = StoredObject::new(
        Kind::Object,
        Some("start_class".to_string()),
        Standing::default(),
        Provenance::direct_input("t"),
        BTreeMap::new(),
        StoredPayload::from_markdown("s"),
        vec![],
    );
    store.write_object(&start_obj).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };
    let request = WorkflowRunRequest {
        run_id: "leak-run".to_string(),
        system_definition: system_ref,
        workflow: workflow_ref,
        inputs: vec![start_obj.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let outcome = engine.run_workflow(request).unwrap();
    let b1 = outcome
        .record
        .events
        .iter()
        .find(|e| e.transition == "branch1")
        .unwrap();
    let b2 = outcome
        .record
        .events
        .iter()
        .find(|e| e.transition == "branch2")
        .unwrap();
    let handoffs = store
        .scan_objects()
        .unwrap()
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::HandoffManifest)
        .map(|obj| {
            serde_json::from_slice::<earmark_core::HandoffManifest>(&obj.payload.bytes).unwrap()
        })
        .filter(|manifest| {
            manifest.run_id == "leak-run" && manifest.from_transition_id == "project"
        })
        .collect::<Vec<_>>();

    assert_eq!(b1.inputs[0], start_obj.object_ref());
    // This is expected to FAIL if the bug exists
    assert_eq!(
        b2.inputs[0],
        start_obj.object_ref(),
        "Branch 2 should take start_obj, NOT branch 1 output"
    );
    assert_eq!(handoffs.len(), 2);
    assert!(handoffs
        .iter()
        .any(|manifest| manifest.to_transition_id.as_deref() == Some("branch1")));
    assert!(handoffs
        .iter()
        .any(|manifest| manifest.to_transition_id.as_deref() == Some("branch2")));
}

#[test]
fn execution_error_persists_failed_delta() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let registry = ProviderRegistry::default();

    let workflow = earmark_core::WorkflowDefinition {
        name: "error-op".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        operations: vec![earmark_core::WorkflowOperation {
            id: "fail".to_string(),
            kind: WorkflowOperationKind::Transform,
            input_contracts: vec!["s".to_string()],
            output_contracts: vec!["e1".to_string(), "e2".to_string()],
            instruction: None,
            compiled_context: None,
            policy: None,
            provider_profile: None,
        }],
        edges: vec![],
        guards: vec![],
        output_contracts: vec![],
    };

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
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "r".to_string(),
            machine_output_default: "j".to_string(),
            work_surface_mode: "m".to_string(),
        },
        activated_at: None,
    };

    let system_ref = store
        .write_object(&StoredObject::new(
            Kind::SystemDefinition,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&system).unwrap()),
            vec![],
        ))
        .unwrap();
    let workflow_ref = store
        .write_object(&StoredObject::new(
            Kind::Workflow,
            None,
            Standing::default(),
            Provenance::direct_input("t"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&workflow).unwrap()),
            vec![],
        ))
        .unwrap();
    let start_obj = StoredObject::new(
        Kind::Object,
        Some("s".to_string()),
        Standing::default(),
        Provenance::direct_input("t"),
        BTreeMap::new(),
        StoredPayload::from_markdown("s"),
        vec![],
    );
    store.write_object(&start_obj).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };
    let request = WorkflowRunRequest {
        run_id: "run-fail".to_string(),
        system_definition: system_ref,
        workflow: workflow_ref,
        inputs: vec![start_obj.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    };

    let result = engine.run_workflow(request);
    assert!(result.is_err());

    let objects = store.scan_objects().unwrap();
    let change_set = objects
        .iter()
        .find(|obj| obj.envelope.kind == Kind::ChangeSet)
        .expect("ChangeSet should be persisted on execution error");

    let payload: earmark_core::ChangeSet =
        serde_json::from_slice(&change_set.payload.bytes).unwrap();
    assert!(!payload.validation_results.is_empty());
    assert!(!payload.validation_results[0].is_valid);
    assert!(payload.validation_results[0]
        .failures
        .join(" ")
        .contains("multi-output transform operations are not yet implemented"));

    let claim_obj = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::TransitionAssignment)
        .find(|obj| {
            let c: earmark_core::TransitionAssignment =
                serde_json::from_slice(&obj.payload.bytes).unwrap();
            c.status == earmark_core::AssignmentStatus::Blocked
        })
        .expect("Blocked TransitionAssignment should be persisted");
    let assignment: earmark_core::TransitionAssignment =
        serde_json::from_slice(&claim_obj.payload.bytes).unwrap();
    assert!(assignment
        .blocked_reason
        .as_ref()
        .unwrap()
        .contains(change_set.envelope.id.as_str()));
}
