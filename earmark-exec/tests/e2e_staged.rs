use earmark_core::{
    to_yaml, ClassDefinition, ClassStandingRules, CompiledContextExpansion, CompiledContextRender,
    CompiledContextSelect, CompiledContextTemplate, CompiledContextVisibility, DimensionId,
    JsonSchemaRef, Kind, Provenance, RuntimeProfile, Standing, SystemDefinition, TokenId,
};
use earmark_exec::{ExecutionEngine, ProviderRegistry, WorkflowRunRequest};
use earmark_index::DerivedIndex;
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn test_neutral_staged_fixture_source_note_to_summary() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();
    let index = DerivedIndex::open(dir.path()).unwrap();
    let registry = ProviderRegistry::default();

    // 1. Define Classes
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
        standing_rules: ClassStandingRules {
            allowed_standing: BTreeMap::from([(
                DimensionId::new("kernel:epistemic"),
                vec![TokenId::new("working")],
            )]),
            ..ClassStandingRules::default()
        },
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "derived_from".to_string(),
            counterparty_classes: vec!["source_note".to_string()],
            direction: None,
            authorizing_endpoint: None,
        }],
        validators: vec![],
    };
    let f_ref = store
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
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "derived_from".to_string(),
            counterparty_classes: vec!["finding".to_string()],
            direction: None,
            authorizing_endpoint: None,
        }],
        validators: vec![],
    };
    let s_ref = store
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

    // 2. Define Instructions
    let instr1 = earmark_core::InstructionPayload {
        name: "source_to_finding".to_string(),
        version: "1.0.0".to_string(),
        purpose: "Extract findings from source notes".to_string(),
        input_classes: vec!["source_note".to_string()],
        output_classes: vec!["finding".to_string()],
        execution_policy: "local".to_string(),
        provider_profile: None,
        trace_policy: "staged".to_string(),
        register: "findings".to_string(),
        body: earmark_core::MarkdownBody::new("extract finding".to_string()),
    };
    let instr1_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr1.to_markdown().unwrap()),
            vec![],
        ))
        .unwrap();

    let instr2 = earmark_core::InstructionPayload {
        name: "finding_to_summary".to_string(),
        version: "1.0.0".to_string(),
        purpose: "Summarize findings".to_string(),
        input_classes: vec!["finding".to_string()],
        output_classes: vec!["summary".to_string()],
        execution_policy: "local".to_string(),
        provider_profile: None,
        trace_policy: "staged".to_string(),
        register: "summaries".to_string(),
        body: earmark_core::MarkdownBody::new("summarize findings".to_string()),
    };
    let instr2_ref = store
        .write_object(&StoredObject::new(
            Kind::Instruction,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_markdown(instr2.to_markdown().unwrap()),
            vec![],
        ))
        .unwrap();

    // 3. Define Projection Template
    let compiled_context_template = CompiledContextTemplate {
        name: "test".to_string(),
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
    let pt_ref = store
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

    // 4. Define Patterns
    let workflow_a_yaml = format!(
        r#"name: workflow_a
version: "1"
description: extraction
operations:
  - id: op_proj
    kind: compile_context
    input_contracts: []
    output_contracts: [ws]
    compiled_context:
      id: {}
      version_id: latest
  - id: op_ext
    kind: transform
    input_contracts: [source_note]
    output_contracts: [finding]
    instruction:
      id: {}
      version_id: latest
edges:
  - from: op_proj
    to: op_ext
guards: []
"#,
        pt_ref.id.as_str(),
        instr1_ref.id.as_str()
    );

    let workflow_a_ref = store
        .write_object(&StoredObject::new(
            Kind::Workflow,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(workflow_a_yaml),
            vec![],
        ))
        .unwrap();

    let workflow_b_yaml = format!(
        r#"name: workflow_b
version: "1"
description: summarization
operations:
  - id: op_proj
    kind: compile_context
    input_contracts: []
    output_contracts: [ws]
    compiled_context:
      id: {}
      version_id: latest
  - id: op_sum
    kind: transform
    input_contracts: [finding]
    output_contracts: [summary]
    instruction:
      id: {}
      version_id: latest
edges:
  - from: op_proj
    to: op_sum
guards: []
"#,
        pt_ref.id.as_str(),
        instr2_ref.id.as_str()
    );

    let workflow_b_ref = store
        .write_object(&StoredObject::new(
            Kind::Workflow,
            None,
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::new(),
            StoredPayload::from_yaml(workflow_b_yaml),
            vec![],
        ))
        .unwrap();

    // 5. Define System
    let system = SystemDefinition {
        system_id: "neutral-system".to_string(),
        namespace: "neutral".to_string(),
        title: "Neutral Staged Fixture".to_string(),
        description: None,
        classes: vec![sn_ref, f_ref, s_ref],
        instructions: vec![instr1_ref, instr2_ref],
        policies: vec![],
        workflows: vec![workflow_a_ref.clone(), workflow_b_ref.clone()],
        compiled_contexts: vec![pt_ref],
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

    // 6. Deposit Source Note
    let source_note = StoredObject::new(
        Kind::Object,
        Some("source_note".to_string()),
        Standing::default(),
        Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            earmark_core::HeaderValue::String("Source 1".to_string()),
        )]),
        StoredPayload::from_markdown("This is a source note."),
        vec![],
    );
    store.write_object(&source_note).unwrap();

    index.rebuild_from_store(&store).unwrap();
    let engine = ExecutionEngine {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // 7. Run Pattern A (Stage 1)
    let outcome_a = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run1".to_string(),
            system_definition: system_ref.clone(),
            workflow: workflow_a_ref,
            inputs: vec![source_note.object_ref()],
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap();

    assert_eq!(outcome_a.record.status, earmark_core::RunStatus::Completed);

    let objects = store.scan_objects().unwrap();
    let handoffs = objects
        .iter()
        .filter(|o| o.envelope.kind == Kind::HandoffManifest)
        .collect::<Vec<_>>();

    let op_ext_handoff_obj = handoffs
        .iter()
        .find(|o| {
            let h: earmark_core::HandoffManifest =
                serde_json::from_slice(&o.payload.bytes).unwrap();
            h.from_transition_id == "op_ext"
        })
        .expect("Handoff from op_ext missing");

    let handoff_payload: earmark_core::HandoffManifest =
        serde_json::from_slice(&op_ext_handoff_obj.payload.bytes).unwrap();
    assert!(!handoff_payload.newly_created_object_ids.is_empty());

    // 8. Run Pattern B (Stage 2) using handoff from Stage 1
    let outcome_b = engine
        .run_workflow(WorkflowRunRequest {
            run_id: "run2".to_string(),
            system_definition: system_ref,
            workflow: workflow_b_ref,
            inputs: vec![], // No direct inputs, use handoff
            handoff_manifest: Some(handoff_payload.id),
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap();

    assert_eq!(outcome_b.record.status, earmark_core::RunStatus::Completed);

    // Verify summary was created
    let final_objects = store.scan_objects().unwrap();
    let summary = final_objects
        .iter()
        .find(|o| o.envelope.class.as_deref() == Some("summary"))
        .expect("Summary missing");
    let payload = summary.payload.as_utf8().unwrap();
    assert!(payload.contains("# Candidate Output"));
    assert!(payload.contains("Instruction: finding_to_summary"));

    // 9. Verify Standing Request Persistence
    let change_sets = final_objects
        .iter()
        .filter(|o| o.envelope.kind == Kind::ChangeSet)
        .collect::<Vec<_>>();
    assert!(change_sets.len() >= 2);

    // 9. Verify Final State
    println!("SUCCESS: E2E Staged Flow Verified");
}
