use super::*;

#[test]
fn workflow_run_materializes_packet_and_run_ledger() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

    let note = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String("Seed note".to_string()),
        )]),
        StoredPayload::from_markdown("seed body"),
        vec![],
    );
    store.write_object(&note).unwrap();

    let compiled_context = CompiledContextTemplate {
        name: "status_surface".to_string(),
        version: "1".to_string(),
        description: Some("Projection".to_string()),
        select: CompiledContextSelect {
            classes: vec!["note".to_string()],
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
    let compiled_context_obj = StoredObject::new(
        Kind::CompiledContextTemplate,
        Some("compiled_context_template".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String("Status Surface".to_string()),
        )]),
        StoredPayload::from_yaml(to_yaml(&compiled_context).unwrap()),
        vec![],
    );
    let compiled_context_ref = store.write_object(&compiled_context_obj).unwrap();

    let instruction = InstructionPayload {
        name: "compose_status".to_string(),
        version: "1".to_string(),
        purpose: "Compose status".to_string(),
        input_classes: vec!["note".to_string()],
        output_classes: vec!["status_summary".to_string()],
        execution_policy: "runtime_permitted".to_string(),
        provider_profile: None,
        trace_policy: "summary".to_string(),
        register: "status_summary".to_string(),
        body: MarkdownBody::new("Produce a bounded status summary.".to_string()),
    };
    let instruction_obj = StoredObject::new(
        Kind::Instruction,
        Some("instruction".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String("Compose status".to_string()),
        )]),
        StoredPayload::from_markdown(instruction.to_markdown().unwrap()),
        vec![],
    );
    let instruction_ref = store.write_object(&instruction_obj).unwrap();

    let output_class = ClassDefinition {
        name: "status_summary".to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let output_class_obj = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String("Status Summary Class".to_string()),
        )]),
        StoredPayload::from_yaml(to_yaml(&output_class).unwrap()),
        vec![],
    );
    let output_class_ref = store.write_object(&output_class_obj).unwrap();

    let workflow_yaml = r#"name: status_flow
version: "1"
description: run compiled_context then local transform
operations:
  - id: op_project
    kind: compile_context
    input_contracts: []
    output_contracts: [work_packet]
    instruction: null
    compiled_context:
      id: PLACEHOLDER_PROJ_ID
      version_id: PLACEHOLDER_PROJ_VERSION
    policy: null
    provider_profile: null
  - id: op_transform
    kind: transform
    input_contracts: [work_packet]
    output_contracts: [status_summary]
    instruction:
      id: PLACEHOLDER_INSTR_ID
      version_id: PLACEHOLDER_INSTR_VERSION
    compiled_context: null
    policy: null
    provider_profile: null
edges:
  - from: op_project
    to: op_transform
    condition: null
guards: []
"#
    .replace("PLACEHOLDER_PROJ_ID", compiled_context_ref.id.as_str())
    .replace(
        "PLACEHOLDER_PROJ_VERSION",
        compiled_context_ref.version_id.as_str(),
    )
    .replace("PLACEHOLDER_INSTR_ID", instruction_ref.id.as_str())
    .replace(
        "PLACEHOLDER_INSTR_VERSION",
        instruction_ref.version_id.as_str(),
    );
    let workflow_obj = StoredObject::new(
        Kind::Workflow,
        Some("composition_workflow".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String("Status Flow".to_string()),
        )]),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();

    let system = SystemDefinition {
        system_id: "pkm-core".to_string(),
        namespace: "systems/pkm-core".to_string(),
        title: "PKM Core".to_string(),
        description: Some("system".to_string()),
        classes: vec![VersionRef::new(
            output_class_ref.id.clone(),
            output_class_ref.version_id.clone(),
        )],
        instructions: vec![VersionRef::new(
            instruction_ref.id.clone(),
            instruction_ref.version_id.clone(),
        )],
        policies: vec![],
        workflows: vec![VersionRef::new(
            workflow_ref.id.clone(),
            workflow_ref.version_id.clone(),
        )],
        compiled_contexts: vec![VersionRef::new(
            compiled_context_ref.id.clone(),
            compiled_context_ref.version_id.clone(),
        )],
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
    let system_obj = StoredObject::new(
        Kind::SystemDefinition,
        Some("system_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String(system.title.clone()),
        )]),
        StoredPayload::from_yaml(to_yaml(&system).unwrap()),
        vec![],
    );
    let system_ref = store.write_object(&system_obj).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run_test_1".to_string(),
            system_definition: VersionRef::new(
                system_ref.id.clone(),
                system_ref.version_id.clone(),
            ),
            workflow: VersionRef::new(workflow_ref.id.clone(), workflow_ref.version_id.clone()),
            inputs: vec![note.object_ref()],
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: false,
        })
        .unwrap();

    assert_eq!(outcome.record.run_id, "run_test_1");
    assert!(!outcome.emitted_packets.is_empty());
    assert_eq!(outcome.emitted_objects.len(), 1);

    let objects = store.scan_objects().unwrap().scanned_objects;
    assert!(objects
        .iter()
        .any(|obj| obj.envelope.kind == Kind::RunRecord));
    let handoffs = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::HandoffManifest)
        .map(|obj| serde_json::from_slice::<HandoffManifest>(&obj.payload.bytes).unwrap())
        .collect::<Vec<_>>();
    assert!(handoffs.iter().any(|manifest| {
        manifest.from_transition_id == "op_project"
            && manifest.to_transition_id.as_deref() == Some("op_transform")
            && manifest
                .allowed_input_classes
                .iter()
                .any(|contract| contract == "work_packet")
            && manifest
                .allowed_output_classes
                .iter()
                .any(|contract| contract == "status_summary")
    }));
}
