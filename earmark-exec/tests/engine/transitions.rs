use super::*;

#[test]
fn test_transform_emits_standing_request() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store");
    let index_path = dir.path().join("index");
    std::fs::create_dir_all(&store_path).unwrap();
    std::fs::create_dir_all(&index_path).unwrap();

    let store = GitCanonicalStore::new(&store_path);
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(&index_path).unwrap();

    let class_def = ClassDefinition {
        name: "finding".to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules {
            allowed_standing: BTreeMap::from([(
                DimensionId::new("kernel:epistemic"),
                vec![TokenId::new("supported")],
            )]),
            ..ClassStandingRules::default()
        },
        relation_rules: vec![],
        validators: vec![],
    };
    let stored_class = StoredObject::new(
        Kind::Object,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&class_def).unwrap()),
        vec![],
    );
    let class_ref = store.write_object(&stored_class).unwrap();

    let note_class = StoredObject::new(
        Kind::Object,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(
            serde_json::to_vec(&ClassDefinition {
                name: "note".to_string(),
                version: "1".to_string(),
                kind: "object".to_string(),
                required_headers: vec![],
                payload_schema: JsonSchemaRef("inline:any".to_string()),
                standing_rules: ClassStandingRules::default(),
                relation_rules: vec![],
                validators: vec![],
            })
            .unwrap(),
        ),
        vec![],
    );
    let note_class_ref = store.write_object(&note_class).unwrap();

    let instr_payload = InstructionPayload {
        name: "extract".to_string(),
        version: "1".to_string(),
        purpose: "test".to_string(),
        input_classes: vec!["note".to_string()],
        output_classes: vec!["finding".to_string()],
        execution_policy: "local".to_string(),
        provider_profile: None,
        trace_policy: "detailed".to_string(),
        register: "finding".to_string(),
        body: MarkdownBody::new("test".to_string()),
    };
    let stored_instr = StoredObject::new(
        Kind::Instruction,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown(instr_payload.to_markdown().unwrap()),
        vec![],
    );
    let instr_ref = store.write_object(&stored_instr).unwrap();

    let system = SystemDefinition {
        system_id: "sys".to_string(),
        namespace: "ns".to_string(),
        title: "title".to_string(),
        description: None,
        classes: vec![class_ref.clone(), note_class_ref.clone()],
        instructions: vec![instr_ref.clone()],
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
            work_surface_mode: "staged".to_string(),
        },
        activated_at: None,
    };
    let system_stored = StoredObject::new(
        Kind::SystemDefinition,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&system).unwrap()),
        vec![],
    );
    let system_ref = store.write_object(&system_stored).unwrap();

    let compiled_context_template = CompiledContextTemplate {
        name: "test".to_string(),
        version: "1".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["note".to_string()],
            standing: BTreeMap::new(),
            relations: vec![],
            time_range: None,
            expansion: CompiledContextExpansion::default(),
        },
        group_by: vec![],
        render: CompiledContextRender {
            mode: "staged".to_string(),
            manifest_format: None,
            prose_template: None,
        },
        visibility: CompiledContextVisibility {
            include_lineage: true,
            include_constraints: true,
            include_provenance: true,
        },
    };
    let pt_stored = StoredObject::new(
        Kind::CompiledContextTemplate,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec(&compiled_context_template).unwrap()),
        vec![],
    );
    let pt_ref = store.write_object(&pt_stored).unwrap();

    let workflow_yaml = format!(
        r#"name: test_standing
version: "1"
description: test
operations:
  - id: op0
    kind: compile_context
    input_contracts: []
    output_contracts: [ws]
    compiled_context:
      id: {}
      version_id: latest
    instruction: null
    policy: null
    provider_profile: null
  - id: op1
    kind: transform
    input_contracts: [note]
    output_contracts: [finding]
    instruction:
      id: {}
      version_id: latest
    compiled_context: null
    policy: null
    provider_profile: null
edges:
  - from: op0
    to: op1
    condition: null
guards: []
"#,
        pt_ref.id.as_str(),
        instr_ref.id.as_str()
    );
    let workflow_obj = StoredObject::new(
        Kind::Workflow,
        None,
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();

    let note = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("test"),
        vec![],
    );
    store.write_object(&note).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let mut engine = ExecutionEngine {
        store: &store,
        index: &mut index,
        provider_service: &registry,
    };

    let result = engine.run_workflow(WorkflowRunRequest {
        run_id: earmark_core::RunId::parse("run1").unwrap(),
        system_definition: system_ref.clone(),
        workflow: VersionRef::new(workflow_ref.id, workflow_ref.version_id),
        inputs: vec![note.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    });

    println!("Pattern Run Result: {:?}", result);
    assert!(result.is_err());

    let objects = store.scan_objects().unwrap().scanned_objects;

    let relations = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::Relation)
        .collect::<Vec<_>>();
    let standing_rel = relations
        .iter()
        .find(|obj| {
            let payload: earmark_core::RelationPayload =
                serde_json::from_slice(&obj.payload.bytes).unwrap();
            payload.relation_type == "requests_standing"
        })
        .expect("requests_standing relation not found");

    let rel_payload: earmark_core::RelationPayload =
        serde_json::from_slice(&standing_rel.payload.bytes).unwrap();
    let request_obj = objects
        .iter()
        .find(|obj| obj.envelope.id == rel_payload.target.id)
        .expect("Standing request object not found");
    let request: earmark_core::StandingTransitionRequest =
        serde_json::from_slice(&request_obj.payload.bytes).unwrap();

    assert_eq!(request.dimension, "kernel:epistemic");
    assert_eq!(request.to_value, "supported");
}
