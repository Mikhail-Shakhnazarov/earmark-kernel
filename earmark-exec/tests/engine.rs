use std::collections::BTreeMap;

use chrono::Utc;
use earmark_core::{
    to_yaml, ChangeSet, ClassDefinition, ClassStandingRules, CompiledContextExpansion,
    CompiledContextRender, CompiledContextSelect, CompiledContextTemplate,
    CompiledContextVisibility, DimensionId, HandoffManifest, HeaderValue, InstructionPayload,
    JsonSchemaRef, Kind, MarkdownBody, Provenance, RuntimeProfile, Standing, SystemDefinition,
    TokenId, TransformationFailure, TransitionAssignment, VersionRef,
};
use earmark_exec::{ExecError, ExecutionEngine, ProviderRegistry, WorkflowRunRequest};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use tempfile::tempdir;

fn persist_transition_assignment(store: &GitCanonicalStore, assignment: &TransitionAssignment) {
    let stored = StoredObject::new(
        Kind::TransitionAssignment,
        Some("transition_assignment".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(assignment).unwrap()),
        vec![],
    );
    store.write_object(&stored).unwrap();
}

fn persist_handoff_manifest(store: &GitCanonicalStore, handoff: &HandoffManifest) {
    let stored = StoredObject::new(
        Kind::HandoffManifest,
        Some("handoff_manifest".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::new(),
        StoredPayload::from_json_bytes(serde_json::to_vec_pretty(handoff).unwrap()),
        vec![],
    );
    store.write_object(&stored).unwrap();
}

fn review_only_fixture(store: &GitCanonicalStore, class: &str) -> (VersionRef, VersionRef) {
    let workflow_yaml = format!(
        r#"name: review_flow
version: "1"
description: review bounded input
operations:
  - id: op_review
    kind: review
    input_contracts: [{class}]
    output_contracts: []
    instruction: null
    compiled_context: null
    policy: null
    provider_profile: null
edges: []
guards: []
"#
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
        system_id: "test-system".to_string(),
        namespace: "systems/test".to_string(),
        title: "Test System".to_string(),
        description: None,
        classes: vec![],
        instructions: vec![],
        policies: vec![],
        workflows: vec![VersionRef::new(
            workflow_ref.id.clone(),
            workflow_ref.version_id.clone(),
        )],
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
    (
        VersionRef::new(system_ref.id, system_ref.version_id),
        VersionRef::new(workflow_ref.id, workflow_ref.version_id),
    )
}

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
    output_contracts: [work_surface]
    instruction: null
    compiled_context:
      id: PLACEHOLDER_PROJ_ID
      version_id: PLACEHOLDER_PROJ_VERSION
    policy: null
    provider_profile: null
  - id: op_transform
    kind: transform
    input_contracts: [work_surface]
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

    let objects = store.scan_objects().unwrap();
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
                .any(|contract| contract == "work_surface")
            && manifest
                .allowed_output_classes
                .iter()
                .any(|contract| contract == "status_summary")
    }));
}

#[test]
fn successor_run_can_reconstruct_inputs_from_handoff_manifest() {
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
    output_contracts: [work_surface]
    instruction: null
    compiled_context:
      id: PLACEHOLDER_PROJ_ID
      version_id: PLACEHOLDER_PROJ_VERSION
    policy: null
    provider_profile: null
  - id: op_transform
    kind: transform
    input_contracts: [work_surface]
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
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let first_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run_stage_a".to_string(),
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
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::HandoffManifest)
        .map(|obj| serde_json::from_slice::<HandoffManifest>(&obj.payload.bytes).unwrap())
        .find(|manifest| {
            manifest.run_id == "run_stage_a" && manifest.from_transition_id == "op_transform"
        })
        .unwrap()
        .id;

    let second_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run_stage_b".to_string(),
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
        .any(|event| event.transition == "op_review"));
}

#[test]
fn claim_reference_continuation_uses_bounded_inputs() {
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

    let assignment = TransitionAssignment {
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

    let outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "claim_resume_run".to_string(),
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
        .any(|event| event.transition == "op_review"));
}

#[test]
fn claim_reference_continuation_uses_handoff_manifest() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

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
        id: earmark_core::HandoffManifestId::new(),
        run_id: "stage_a".to_string(),
        from_transition_id: "op_transform".to_string(),
        to_transition_id: Some("op_review".to_string()),
        source_change_set_id: earmark_core::ChangeSetId::new(),
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

    let assignment = TransitionAssignment {
        id: earmark_core::TransitionAssignmentId::new(),
        run_id: "claim_run".to_string(),
        transition_id: "op_review".to_string(),
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
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    let outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "claim_resume_handoff_run".to_string(),
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
        .any(|event| event.transition == "op_review"));
}

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
    let assignment = TransitionAssignment {
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
        // Intentionally mismatched register/output contract to exercise validation failure.
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
    output_contracts: [work_surface]
    instruction: null
    compiled_context:
      id: PLACEHOLDER_PROJ_ID
      version_id: PLACEHOLDER_PROJ_VERSION
    policy: null
    provider_profile: null
  - id: op_transform
    kind: transform
    input_contracts: [work_surface]
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

    let objects = store.scan_objects().unwrap();
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

    // Verify failure object and timeline event for validation failure
    let failure_store_entry = objects
        .iter()
        .find(|obj| obj.envelope.kind == Kind::TransformationFailure)
        .expect("TransformationFailure not found");
    let val_failure: TransformationFailure =
        serde_json::from_slice(&failure_store_entry.payload.bytes).unwrap();
    assert_eq!(val_failure.error_type, "validation_error");
    assert!(!val_failure.input_object_ids.is_empty());

    // Verify run timeline includes a validation_error event with the failure ref
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
    output_contracts: [work_surface]
    instruction: null
    compiled_context:
      id: PLACEHOLDER_PROJ_ID
      version_id: PLACEHOLDER_PROJ_VERSION
    policy: null
    provider_profile: null
  - id: op_transform
    kind: transform
    input_contracts: [work_surface]
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
    output_contracts: [work_surface]
    instruction: null
    compiled_context:
      id: PLACEHOLDER_PROJ_ID
      version_id: PLACEHOLDER_PROJ_VERSION
    policy: null
    provider_profile: null
  - id: op_transform
    kind: transform
    input_contracts: [work_surface]
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

    let objects = store.scan_objects().unwrap();
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

    let assignment = TransitionAssignment {
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

    // Both inputs and transition_assignment set
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

    // Assert no NEW assignments or change_sets were written
    let objects = store.scan_objects().unwrap();
    let claim_count = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::TransitionAssignment)
        .count();
    let delta_count = objects
        .iter()
        .filter(|obj| obj.envelope.kind == Kind::ChangeSet)
        .count();

    // We already persisted ONE assignment manually
    assert_eq!(claim_count, 1);
    assert_eq!(delta_count, 0);
}

#[test]
fn within_workflow_handoff_continuation_runs_successor_not_predecessor() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

    // Define classes
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

    // Define instructions
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

    // Compiled context template (needed by transforms)
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

    // Workflow with 3 transitions:
    // op_proj (compile_context) -> op_extract (transform) -> op_summarize (transform)
    // op_summarize declares compiled_context explicitly so the handoff from
    // op_extract carries it, allowing continuation to rebuild the work surface.
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

    // System referencing all classes, instructions, and the template
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

    // Deposit source note
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
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // Run 1: from direct inputs
    let first_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run_first".to_string(),
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

    // Find handoff from op_extract (should have to_transition_id = Some("op_summarize"))
    let objects = store.scan_objects().unwrap();
    let handoff_obj = objects
        .iter()
        .filter(|o| o.envelope.kind == Kind::HandoffManifest)
        .map(|o| serde_json::from_slice::<HandoffManifest>(&o.payload.bytes).unwrap())
        .find(|h| h.from_transition_id == "op_extract")
        .expect("handoff from op_extract not found");

    assert_eq!(
        handoff_obj.to_transition_id.as_deref(),
        Some("op_summarize"),
        "handoff should target op_summarize"
    );

    // Record how many findings were created in Run 1
    let first_finding_count = objects
        .iter()
        .filter(|o| o.envelope.class.as_deref() == Some("finding"))
        .count();
    assert!(
        first_finding_count > 0,
        "op_extract should have produced findings"
    );

    // Run 2: from handoff — should only run op_summarize, not re-run op_proj or op_extract
    let second_outcome = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run_second".to_string(),
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

    // Verify continuation event was recorded
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
    assert_eq!(continuation_events[0].transition, "op_summarize");
    assert!(continuation_events[0]
        .message
        .as_deref()
        .unwrap_or("")
        .contains("continued from handoff"));

    // Verify op_summarize executed (should have events with that transition)
    let summarize_events: Vec<_> = second_outcome
        .record
        .events
        .iter()
        .filter(|e| e.transition == "op_summarize")
        .collect();
    assert!(
        !summarize_events.is_empty(),
        "op_summarize should have execution events"
    );

    // Verify op_proj and op_extract did NOT execute (no events with those transition ids)
    let proj_events: Vec<_> = second_outcome
        .record
        .events
        .iter()
        .filter(|e| e.transition == "op_proj")
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
        .filter(|e| e.transition == "op_extract")
        .collect();
    assert!(
        extract_events.is_empty(),
        "op_extract should have no events in handoff run (got {})",
        extract_events.len()
    );

    // Verify no misleading partial_execution event for intentionally skipped ancestors
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

    // Verify no new findings were created by the handoff continuation run
    let final_objects = store.scan_objects().unwrap();
    let final_finding_count = final_objects
        .iter()
        .filter(|o| o.envelope.class.as_deref() == Some("finding"))
        .count();
    assert_eq!(
        final_finding_count, first_finding_count,
        "handoff continuation should not create new findings (predecessor skipped)"
    );
}

#[test]
fn handoff_continuation_with_invalid_target_fails() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();

    let (system_ref, workflow_ref) = review_only_fixture(&store, "note");

    // Create a handoff with to_transition_id pointing to a non-existent transition
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

    // Pattern that triggers failure due to missing work surface (no project before transform)
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

    let objects = store.scan_objects().unwrap();
    let failure_obj = objects
        .iter()
        .find(|obj| obj.envelope.kind == Kind::TransformationFailure)
        .expect("TransformationFailure not found");
    let failure: TransformationFailure =
        serde_json::from_slice(&failure_obj.payload.bytes).unwrap();
    assert!(failure
        .message
        .contains("requires an instruction reference"));

    // Check for failed change_set
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

    // Check assignment status and linkage
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

    // Verify failure links to run_id and transition_id are populated
    assert_eq!(failure.run_id, "fail_run");
    assert_eq!(failure.transition_id, "op_fail");
    assert_eq!(failure.error_type, "invalid_workflow");
    assert!(!failure.input_object_ids.is_empty());
}

#[test]
fn test_transform_emits_standing_request() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store");
    let index_path = dir.path().join("index");
    std::fs::create_dir_all(&store_path).unwrap();
    std::fs::create_dir_all(&index_path).unwrap();

    let store = GitCanonicalStore::new(&store_path);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(&index_path).unwrap();

    // 1. Define a class that REQUIRES 'supported' epistemic standing
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

    // 2. Define an instruction and system
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

    // 3. Define compiled_context template
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

    // 4. Define workflow
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

    // 5. Note object
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
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // 6. Run workflow
    let result = engine.run_workflow(WorkflowRunRequest {
        run_id: "run1".to_string(),
        system_definition: system_ref.clone(),
        workflow: VersionRef::new(workflow_ref.id, workflow_ref.version_id),
        inputs: vec![note.object_ref()],
        handoff_manifest: None,
        transition_assignment: None,
        operator_approved: true,
    });

    println!("Pattern Run Result: {:?}", result);
    assert!(result.is_err()); // Should fail validation because standing is 'unresolved' but class requires 'supported'

    let objects = store.scan_objects().unwrap();

    // 6. Assert standing request recorded via relations
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
