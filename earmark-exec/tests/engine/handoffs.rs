use super::*;

#[test]
fn successor_run_can_reconstruct_inputs_from_handoff_manifest() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(dir.path()).unwrap();

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

    let workflow_a_yaml = r#"name: status_flow
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
    let workflow_a_obj = StoredObject::new(
        Kind::Workflow,
        Some("composition_workflow".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_a_yaml),
        vec![],
    );
    let workflow_a_ref = store.write_object(&workflow_a_obj).unwrap();

    let workflow_b_yaml = r#"name: review_flow
version: "1"
description: consume bounded handoff and review it
operations:
  - id: op_review
    kind: review
    input_contracts: [status_summary]
    output_contracts: []
    instruction: null
    compiled_context: null
    policy: null
    provider_profile: null
edges: []
guards: []
"#;
    let workflow_b_obj = StoredObject::new(
        Kind::Workflow,
        Some("composition_workflow".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_b_yaml),
        vec![],
    );
    let workflow_b_ref = store.write_object(&workflow_b_obj).unwrap();

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
        workflows: vec![
            VersionRef::new(workflow_a_ref.id.clone(), workflow_a_ref.version_id.clone()),
            VersionRef::new(workflow_b_ref.id.clone(), workflow_b_ref.version_id.clone()),
        ],
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
    let mut engine = ExecutionEngine {
        store: &store,
        index: &mut index,
        provider_service: &registry,
    };

    let first_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: earmark_core::RunId::parse("run_stage_a").unwrap(),
            system_definition: VersionRef::new(
                system_ref.id.clone(),
                system_ref.version_id.clone(),
            ),
            workflow: VersionRef::new(workflow_a_ref.id.clone(), workflow_a_ref.version_id.clone()),
            inputs: vec![note.object_ref()],
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap();
    assert!(first_outcome
        .record
        .manifests
        .iter()
        .any(|manifest_id| !manifest_id.as_str().is_empty()));

    let handoff_id = store
        .scan_objects()
        .unwrap()
        .scanned_objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::HandoffManifest)
        .map(|obj| serde_json::from_slice::<HandoffManifest>(&obj.payload.bytes).unwrap())
        .find(|manifest| {
            manifest.run_id.as_str() == "run_stage_a"
                && manifest.from_transition_id.as_str() == "tr_op_transform"
        })
        .unwrap()
        .id;

    let second_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: earmark_core::RunId::parse("run_stage_b").unwrap(),
            system_definition: VersionRef::new(
                system_ref.id.clone(),
                system_ref.version_id.clone(),
            ),
            workflow: VersionRef::new(workflow_b_ref.id.clone(), workflow_b_ref.version_id.clone()),
            inputs: vec![],
            handoff_manifest: Some(handoff_id),
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap();

    assert!(second_outcome
        .record
        .events
        .iter()
        .any(|event| event.transition.as_str() == "tr_op_review"));
}

#[test]
fn claim_reference_continuation_uses_bounded_inputs() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(dir.path()).unwrap();

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

    let assignment = earmark_core::TransitionAssignment {
        id: earmark_core::TransitionAssignmentId::generate(),
        run_id: earmark_core::RunId::parse("claim_run").unwrap(),
        transition_id: earmark_core::TransitionId::parse("op_review").unwrap(),
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
    let mut engine = ExecutionEngine {
        store: &store,
        index: &mut index,
        provider_service: &registry,
    };

    let outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: earmark_core::RunId::parse("claim_resume_run").unwrap(),
            system_definition: system_ref,
            workflow: workflow_ref,
            inputs: vec![],
            handoff_manifest: None,
            transition_assignment: Some(assignment.id.clone()),
            operator_approved: true,
        })
        .unwrap();

    assert!(outcome
        .record
        .events
        .iter()
        .any(|event| event.transition.as_str() == "tr_op_review"));
}

#[test]
fn claim_reference_continuation_uses_handoff_manifest() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let summary = StoredObject::new(
        Kind::Object,
        Some("status_summary".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_markdown("summary body"),
        vec![],
    );
    store.write_object(&summary).unwrap();
    let (system_ref, workflow_ref) = review_only_fixture(&store, "status_summary");

    let handoff = HandoffManifest {
        id: earmark_core::HandoffManifestId::generate(),
        run_id: earmark_core::RunId::parse("stage_a").unwrap(),
        from_transition_id: earmark_core::TransitionId::parse("op_transform").unwrap(),
        to_transition_id: Some(earmark_core::TransitionId::parse("op_review").unwrap()),
        source_change_set_id: earmark_core::ChangeSetId::generate(),
        source_assignment_id: None,
        root_object_ids: vec![summary.envelope.id.clone()],
        inherited_input_object_ids: vec![],
        newly_created_object_ids: vec![summary.envelope.id.clone()],
        newly_created_relation_ids: vec![],
        allowed_input_classes: vec!["status_summary".to_string()],
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
        id: earmark_core::TransitionAssignmentId::generate(),
        run_id: earmark_core::RunId::parse("claim_run").unwrap(),
        transition_id: earmark_core::TransitionId::parse("op_review").unwrap(),
        assigned_to: "operator".to_string(),
        status: earmark_core::AssignmentStatus::Assigned,
        input_object_ids: vec![],
        handoff_manifest_id: Some(handoff.id.clone()),
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
    let mut engine = ExecutionEngine {
        store: &store,
        index: &mut index,
        provider_service: &registry,
    };

    let outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: earmark_core::RunId::parse("claim_resume_handoff_run").unwrap(),
            system_definition: system_ref,
            workflow: workflow_ref,
            inputs: vec![],
            handoff_manifest: None,
            transition_assignment: Some(assignment.id.clone()),
            operator_approved: true,
        })
        .unwrap();

    assert!(outcome
        .record
        .events
        .iter()
        .any(|event| event.transition.as_str() == "tr_op_review"));
}

#[test]
fn within_workflow_handoff_continuation_runs_successor_not_predecessor() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let mut index = DerivedIndex::open(dir.path()).unwrap();

    let source_note_class = ClassDefinition {
        name: "source_note".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec!["title".to_string()],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let sn_ref = store
        .write_object(&StoredObject::new(
            Kind::Object,
            Some("class_definition".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&source_note_class).unwrap()),
            vec![],
        ))
        .unwrap();

    let finding_class = ClassDefinition {
        name: "finding".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec!["title".to_string()],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let finding_ref = store
        .write_object(&StoredObject::new(
            Kind::Object,
            Some("class_definition".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&finding_class).unwrap()),
            vec![],
        ))
        .unwrap();

    let summary_class = ClassDefinition {
        name: "summary".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec!["title".to_string()],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let summary_ref = store
        .write_object(&StoredObject::new(
            Kind::Object,
            Some("class_definition".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&summary_class).unwrap()),
            vec![],
        ))
        .unwrap();

    let instr_extract = earmark_core::InstructionPayload {
        name: "extract_findings".to_string(),
        version: "1.0.0".to_string(),
        purpose: "Extract findings from source".to_string(),
        input_classes: vec!["source_note".to_string()],
        output_classes: vec!["finding".to_string()],
        execution_policy: "local".to_string(),
        provider_profile: None,
        trace_policy: "staged".to_string(),
        register: "finding".to_string(),
        body: earmark_core::MarkdownBody::new("extract".to_string()),
    };
    let instr_extract_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr_extract.to_markdown().unwrap()),
            vec![],
        ))
        .unwrap();

    let instr_summarize = earmark_core::InstructionPayload {
        name: "summarize_findings".to_string(),
        version: "1.0.0".to_string(),
        purpose: "Summarize findings".to_string(),
        input_classes: vec!["finding".to_string()],
        output_classes: vec!["summary".to_string()],
        execution_policy: "local".to_string(),
        provider_profile: None,
        trace_policy: "staged".to_string(),
        register: "summary".to_string(),
        body: earmark_core::MarkdownBody::new("summarize".to_string()),
    };
    let instr_summarize_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr_summarize.to_markdown().unwrap()),
            vec![],
        ))
        .unwrap();

    let compiled_context_template = CompiledContextTemplate {
        name: "test_surface".to_string(),
        version: "1".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["source_note".to_string(), "finding".to_string()],
            standing: BTreeMap::new(),
            relations: vec!["derived_from".to_string()],
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
    let template_ref = store
        .write_object(&StoredObject::new(
            Kind::CompiledContextTemplate,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_json_bytes(serde_json::to_vec(&compiled_context_template).unwrap()),
            vec![],
        ))
        .unwrap();

    let workflow_yaml = format!(
        r#"name: proj_extract_summarize
version: "1"
operations:
  - id: op_proj
    kind: compile_context
    input_contracts: []
    output_contracts: [ws]
    compiled_context:
      id: {}
      version_id: latest
  - id: op_extract
    kind: transform
    input_contracts: [source_note]
    output_contracts: [finding]
    instruction:
      id: {}
      version_id: latest
  - id: op_summarize
    kind: transform
    input_contracts: [finding]
    output_contracts: [summary]
    instruction:
      id: {}
      version_id: latest
    compiled_context:
      id: {}
      version_id: latest
edges:
  - from: op_proj
    to: op_extract
  - from: op_extract
    to: op_summarize
guards: []
"#,
        template_ref.id.as_str(),
        instr_extract_ref.id.as_str(),
        instr_summarize_ref.id.as_str(),
        template_ref.id.as_str(),
    );
    let workflow_ref = store
        .write_object(&StoredObject::new(
            Kind::Workflow,
            Some("composition_workflow".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(workflow_yaml),
            vec![],
        ))
        .unwrap();

    let system = SystemDefinition {
        system_id: "test-system".to_string(),
        namespace: "test".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![sn_ref, finding_ref, summary_ref],
        instructions: vec![instr_extract_ref, instr_summarize_ref],
        policies: vec![],
        workflows: vec![workflow_ref.clone()],
        compiled_contexts: vec![template_ref],
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
    let system_ref = store
        .write_object(&StoredObject::new(
            Kind::SystemDefinition,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(to_yaml(&system).unwrap()),
            vec![],
        ))
        .unwrap();

    let source_note = StoredObject::new(
        Kind::Object,
        Some("source_note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Source 1".to_string()),
        )]),
        StoredPayload::from_markdown("Source body"),
        vec![],
    );
    store.write_object(&source_note).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let registry = ProviderRegistry::default();
    let mut engine = ExecutionEngine {
        store: &store,
        index: &mut index,
        provider_service: &registry,
    };

    let first_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: earmark_core::RunId::parse("run_first").unwrap(),
            system_definition: system_ref.clone(),
            workflow: workflow_ref.clone(),
            inputs: vec![source_note.object_ref()],
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap();

    assert_eq!(
        first_outcome.record.status,
        earmark_core::RunStatus::Completed
    );

    let objects = store.scan_objects().unwrap().scanned_objects;
    let handoff_obj = objects
        .iter()
        .filter(|o| o.envelope.kind == Kind::HandoffManifest)
        .map(|o| serde_json::from_slice::<HandoffManifest>(&o.payload.bytes).unwrap())
        .find(|h| h.from_transition_id == "tr_op_extract")
        .expect("handoff from op_extract not found");

    assert_eq!(
        handoff_obj.to_transition_id.as_deref(),
        Some("tr_op_summarize"),
        "handoff should target op_summarize"
    );

    let first_finding_count = objects
        .iter()
        .filter(|o| o.envelope.class.as_deref() == Some("finding"))
        .count();
    assert!(
        first_finding_count > 0,
        "op_extract should have produced findings"
    );

    let second_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: earmark_core::RunId::parse("run_second").unwrap(),
            system_definition: system_ref,
            workflow: workflow_ref,
            inputs: vec![],
            handoff_manifest: Some(handoff_obj.id),
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap();

    assert_eq!(
        second_outcome.record.status,
        earmark_core::RunStatus::Completed
    );

    let continuation_events: Vec<_> = second_outcome
        .record
        .events
        .iter()
        .filter(|e| e.event_type == "continuation")
        .collect();
    assert_eq!(
        continuation_events.len(),
        1,
        "should have exactly one continuation event"
    );
    assert_eq!(continuation_events[0].transition, "tr_op_summarize");
    assert!(continuation_events[0]
        .message
        .as_deref()
        .unwrap_or("")
        .contains("continued from handoff"));

    let summarize_events: Vec<_> = second_outcome
        .record
        .events
        .iter()
        .filter(|e| e.transition == "tr_op_summarize")
        .collect();
    assert!(
        !summarize_events.is_empty(),
        "op_summarize should have execution events"
    );

    let proj_events: Vec<_> = second_outcome
        .record
        .events
        .iter()
        .filter(|e| e.transition == "tr_op_proj")
        .collect();
    assert!(
        proj_events.is_empty(),
        "op_proj should have no events in handoff run (got {})",
        proj_events.len()
    );
    let extract_events: Vec<_> = second_outcome
        .record
        .events
        .iter()
        .filter(|e| e.transition == "tr_op_extract")
        .collect();
    assert!(
        extract_events.is_empty(),
        "op_extract should have no events in handoff run (got {})",
        extract_events.len()
    );

    let partial_events: Vec<_> = second_outcome
        .record
        .events
        .iter()
        .filter(|e| e.event_type == "partial_execution")
        .collect();
    assert!(
        partial_events.is_empty(),
        "should not report partial_execution for intentionally skipped upstream ancestors"
    );

    let final_objects = store.scan_objects().unwrap().scanned_objects;
    let final_finding_count = final_objects
        .iter()
        .filter(|o| o.envelope.class.as_deref() == Some("finding"))
        .count();
    assert_eq!(
        final_finding_count, first_finding_count,
        "handoff continuation should not create new findings (predecessor skipped)"
    );
}
