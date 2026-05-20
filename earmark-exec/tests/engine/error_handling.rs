use super::*;

#[test]
fn request_with_inputs_and_handoff_manifest_fails() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let note = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("seed body"),
        vec![],
    );
    store.write_object(&note).unwrap();
    let (system_ref, workflow_ref) = review_only_fixture(&store, "note");
    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "stage_a".to_string(),
        from_transition_id: "op_transform".to_string(),
        to_transition_id: Some("op_review".to_string()),
        source_change_set_id: earmark_core::ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![note.envelope.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec!["note".to_string()],
        allowed_output_classes: vec![],
        allowed_relation_types: vec![],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };
    persist_handoff_manifest(&store, &handoff);

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let error = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "conflict_run".to_string(),
            system_definition: system_ref,
            workflow: workflow_ref,
            inputs: vec![note.object_ref()],
            handoff_manifest: Some(handoff.id.clone()),
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        ExecError::ConflictingContinuationSources(_)
    ));
}

#[test]
fn request_with_handoff_manifest_and_transition_assignment_fails() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let note = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("seed body"),
        vec![],
    );
    store.write_object(&note).unwrap();
    let (system_ref, workflow_ref) = review_only_fixture(&store, "note");
    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "stage_a".to_string(),
        from_transition_id: "op_transform".to_string(),
        to_transition_id: Some("op_review".to_string()),
        source_change_set_id: earmark_core::ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![note.envelope.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec!["note".to_string()],
        allowed_output_classes: vec![],
        allowed_relation_types: vec![],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };
    persist_handoff_manifest(&store, &handoff);
    let assignment = earmark_core::TransitionAssignment {
        id: earmark_core::TransitionAssignmentId::new(),
        run_id: "claim_run".to_string(),
        transition_id: "op_review".to_string(),
        assigned_to: "operator".to_string(),
        status: earmark_core::AssignmentStatus::Assigned,
        input_object_ids: vec![note.envelope.id.clone()],
        handoff_manifest_id: None,
        event_ids: vec![],
        blocked_reason: None,
        completion_change_set_id: None,
        assigned_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: None,
        completed_at: None,
    };
    persist_transition_assignment(&store, &assignment);

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let error = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "conflict_claim_run".to_string(),
            system_definition: system_ref,
            workflow: workflow_ref,
            inputs: vec![],
            handoff_manifest: Some(handoff.id.clone()),
            transition_assignment: Some(assignment.id.clone()),
            operator_approved: true,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        ExecError::ConflictingContinuationSources(_)
    ));
}

#[test]
fn workflow_run_fails_when_transform_output_class_is_undeclared() {
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
        BTreeMap::new(),
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
        register: "machined".to_string(),
        body: MarkdownBody::new("Produce a bounded status summary.".to_string()),
    };
    let instruction_obj = StoredObject::new(
        Kind::Instruction,
        Some("instruction".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown(instruction.to_markdown().unwrap()),
        vec![],
    );
    let instruction_ref = store.write_object(&instruction_obj).unwrap();

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
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();

    let system = SystemDefinition {
        system_id: "pkm-core".to_string(),
        namespace: "systems/pkm-core".to_string(),
        title: "PKM Core".to_string(),
        description: Some("system".to_string()),
        classes: vec![],
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
        BTreeMap::new(),
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

    let error = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run_invalid_class".to_string(),
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
        .unwrap_err();

    assert!(matches!(error, ExecError::IncompleteExecution(_)));

    let objects = store.scan_objects().unwrap().scanned_objects;
    let failed_deltas = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::ChangeSet)
        .map(|obj| serde_json::from_slice::<ChangeSet>(&obj.payload.bytes).unwrap())
        .filter(|change_set| {
            change_set.run_id == "run_invalid_class" && change_set.transition_id == "op_transform"
        })
        .collect::<Vec<_>>();
    assert_eq!(failed_deltas.len(), 1);
    assert!(!failed_deltas[0].validation_results[0].is_valid);
    assert!(failed_deltas[0].handoff_manifest_id.is_none());
    assert!(!failed_deltas[0].created_relation_ids.is_empty());

    let blocked_claims = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::TransitionAssignment)
        .map(|obj| serde_json::from_slice::<TransitionAssignment>(&obj.payload.bytes).unwrap())
        .filter(|assignment| {
            assignment.run_id == "run_invalid_class" && assignment.transition_id == "op_transform"
        })
        .collect::<Vec<_>>();
    let blocked_claim = blocked_claims
        .iter()
        .find(|assignment| assignment.status == earmark_core::AssignmentStatus::Blocked)
        .unwrap();
    assert!(blocked_claim
        .blocked_reason
        .as_ref()
        .unwrap()
        .contains("validation failed"));

    let failure_store_entry = objects
        .iter()
        .find(|obj| obj.envelope.kind == Kind::TransformationFailure)
        .expect("TransformationFailure not found");
    let val_failure: TransformationFailure =
        serde_json::from_slice(&failure_store_entry.payload.bytes).unwrap();
    assert_eq!(val_failure.error_type, "validation_error");
    assert!(!val_failure.input_object_ids.is_empty());

    let run_record: earmark_core::RunRecord = objects
        .iter()
        .find(|obj| obj.envelope.kind == Kind::RunRecord)
        .map(|obj| serde_json::from_slice(&obj.payload.bytes).unwrap())
        .expect("run record not found");
    let failure_event = run_record
        .events
        .iter()
        .find(|ev| ev.event_type == "validation_error");
    assert!(
        failure_event.is_some(),
        "expected validation_error timeline event"
    );
    let event = failure_event.unwrap();
    assert!(
        event
            .outputs
            .iter()
            .any(|o| o.id == failure_store_entry.envelope.id),
        "validation_error event outputs should contain failure object id"
    );
}

#[test]
fn workflow_run_fails_when_output_standing_violates_class_rules() {
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
        BTreeMap::new(),
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
        BTreeMap::new(),
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
    let output_class_obj = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
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
        BTreeMap::new(),
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
        BTreeMap::new(),
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

    let error = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run_invalid_standing".to_string(),
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
        .unwrap_err();

    match error {
        ExecError::IncompleteExecution(message) => {
            assert!(message.contains("disallowed kernel:epistemic standing"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn workflow_run_fails_when_declared_transition_is_unreachable() {
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
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&output_class).unwrap()),
        vec![],
    );
    let output_class_ref = store.write_object(&output_class_obj).unwrap();

    let workflow_yaml = r#"name: broken_flow
version: "1"
description: unreachable transition should fail execution
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
  - id: op_review
    kind: review
    input_contracts: [status_summary]
    output_contracts: [reviewed_summary]
    instruction: null
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
            HeaderValue::String("Broken Flow".to_string()),
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
            run_id: "run_test_incomplete".to_string(),
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

    let partial_events: Vec<_> = outcome
        .record
        .events
        .iter()
        .filter(|e| e.event_type == "partial_execution")
        .collect();
    assert!(!partial_events.is_empty());
    assert!(partial_events[0]
        .message
        .as_ref()
        .unwrap()
        .contains("1 transitions unreached"));

    let objects = store.scan_objects().unwrap().scanned_objects;
    let ledgers = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::RunRecord)
        .collect::<Vec<_>>();
    assert!(!ledgers.is_empty());
}

#[test]
fn test_mixed_source_rejection_no_side_effects() {
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
    let (system_ref, workflow_ref) = review_only_fixture(&store, "note");

    let assignment = earmark_core::TransitionAssignment {
        id: earmark_core::TransitionAssignmentId::new(),
        run_id: "mixed_run".to_string(),
        transition_id: "op_review".to_string(),
        assigned_to: "operator".to_string(),
        status: earmark_core::AssignmentStatus::Assigned,
        input_object_ids: vec![note.envelope.id.clone()],
        handoff_manifest_id: None,
        event_ids: vec![],
        blocked_reason: None,
        completion_change_set_id: None,
        assigned_at: Utc::now(),
        updated_at: Utc::now(),
        expires_at: None,
        completed_at: None,
    };
    persist_transition_assignment(&store, &assignment);

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let result = engine.run_workflow(WorkflowRunRequest {
        run_id: "mixed_request_run".to_string(),
        system_definition: system_ref,
        workflow: workflow_ref,
        inputs: vec![note.object_ref()],
        handoff_manifest: None,
        transition_assignment: Some(assignment.id.clone()),
        operator_approved: true,
    });

    assert!(result.is_err());

    let objects = store.scan_objects().unwrap().scanned_objects;
    let claim_count = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::TransitionAssignment)
        .count();
    let delta_count = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::ChangeSet)
        .count();

    assert_eq!(claim_count, 1);
    assert_eq!(delta_count, 0);
}

#[test]
fn handoff_continuation_with_invalid_target_fails() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

    let (system_ref, workflow_ref) = review_only_fixture(&store, "note");

    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::new(),
        run_id: "dummy_run".to_string(),
        from_transition_id: "op_prev".to_string(),
        to_transition_id: Some("op_nonexistent".to_string()),
        source_change_set_id: earmark_core::ChangeSetId::new(),
        source_assignment_id: None,
        root_object_ids: vec![],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec![],
        allowed_output_classes: vec![],
        allowed_relation_types: vec![],
        standing_constraints: vec![],
        unresolved_ambiguities: vec![],
        blocked_conditions: vec![],
        required_checks: vec![],
        compiled_context_template_id: None,
        created_at: Utc::now(),
    };
    persist_handoff_manifest(&store, &handoff);

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let result = engine.run_workflow(WorkflowRunRequest {
        run_id: "invalid_target_run".to_string(),
        system_definition: system_ref,
        workflow: workflow_ref,
        inputs: vec![],
        handoff_manifest: Some(handoff.id),
        transition_assignment: None,
        operator_approved: true,
    });

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("op_nonexistent") && err_msg.contains("not present"),
        "error should mention the missing transition, got: {}",
        err_msg
    );
}

#[test]
fn test_transformation_failure_recorded_on_error() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

    let note = StoredObject::new(
        Kind::Object,
        Some("note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("seed body"),
        vec![],
    );
    store.write_object(&note).unwrap();

    let (system_ref, _) = review_only_fixture(&store, "note");

    let workflow_yaml = r#"name: fail_flow
version: "1"
description: fail during transform
operations:
  - id: op_fail
    kind: transform
    input_contracts: [note]
    output_contracts: [note]
    instruction: null
    compiled_context: null
    policy: null
    provider_profile: null
edges: []
guards: []
"#;
    let workflow_obj = StoredObject::new(
        Kind::Workflow,
        Some("composition_workflow".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let result = engine.run_workflow(WorkflowRunRequest {
        run_id: "fail_run".to_string(),
        system_definition: system_ref,
        workflow: VersionRef::new(workflow_ref.id, workflow_ref.version_id),
        inputs: vec![note.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    });

    println!("Result: {:?}", result);
    assert!(result.is_err());

    let objects = store.scan_objects().unwrap().scanned_objects;
    let failure_obj = objects
        .iter()
        .find(|obj| obj.envelope.kind == Kind::TransformationFailure)
        .expect("TransformationFailure not found");
    let failure: TransformationFailure =
        serde_json::from_slice(&failure_obj.payload.bytes).unwrap();
    assert!(failure
        .message
        .contains("requires an instruction reference"));

    let delta_id = failure
        .failed_change_set_id
        .expect("Failed change_set ID missing from failure record");
    let delta_obj = objects
        .iter()
        .find(|obj| {
            if obj.envelope.kind != Kind::ChangeSet {
                return false;
            }
            let change_set: ChangeSet = serde_json::from_slice(&obj.payload.bytes).unwrap();
            change_set.id.as_str() == delta_id.as_str()
        })
        .expect("ChangeSet not found in store");
    let change_set: ChangeSet = serde_json::from_slice(&delta_obj.payload.bytes).unwrap();
    assert!(!change_set.validation_results[0].is_valid);
    assert!(
        change_set.validation_results[0].failures[0].contains("requires an instruction reference")
    );

    let claim_obj = objects
        .iter()
        .find(|obj| {
            if obj.envelope.kind != Kind::TransitionAssignment {
                return false;
            }
            let assignment: TransitionAssignment =
                serde_json::from_slice(&obj.payload.bytes).unwrap();
            assignment.id.as_str() == failure.assignment_id.as_str()
                && assignment.status == earmark_core::AssignmentStatus::Blocked
        })
        .expect("TransitionAssignment with Blocked status not found");
    let assignment: TransitionAssignment =
        serde_json::from_slice(&claim_obj.payload.bytes).unwrap();
    assert!(assignment
        .blocked_reason
        .as_ref()
        .unwrap()
        .contains(delta_id.as_str()));

    assert_eq!(failure.run_id, "fail_run");
    assert_eq!(failure.transition_id, "op_fail");
    assert_eq!(failure.error_type, "invalid_workflow");
    assert!(!failure.input_object_ids.is_empty());
}
