use chrono::Utc;
use earmark_core::{
    to_yaml, ClassDefinition, ClassStandingRules, CompiledContextExpansion, CompiledContextRender,
    CompiledContextSelect, CompiledContextTemplate, CompiledContextVisibility, InstructionPayload,
    JsonSchemaRef, Kind, MarkdownBody, ObjectRef, ProviderRequest, ProviderResponse, ProviderUsage,
    RuntimeProfile, SystemDefinition, VersionRef,
};
use earmark_exec::{ProviderAdapter, ProviderFailure, ProviderRegistry, WorkflowRunRequest};
use earmark_index::DerivedIndex;
use earmark_runtime_tools::RuntimeToolSurface;
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use std::collections::BTreeMap;
use std::sync::Arc;
use tempfile::tempdir;

struct MockAdapter;
impl ProviderAdapter for MockAdapter {
    fn provider_key(&self) -> &'static str {
        "mock"
    }

    fn provide(
        &self,
        request: ProviderRequest,
        _profile: &earmark_core::ProviderProfile,
        _transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        Ok(ProviderResponse {
            request_id: request.request_id,
            provider: "mock".to_string(),
            model: "echo".to_string(),
            status: earmark_core::ProviderResponseStatus::Completed,
            candidate_payload: "Mocked candidate response".to_string(),
            metadata: BTreeMap::new(),
            advisory_warnings: vec![],
            usage: Some(ProviderUsage {
                input_tokens: Some(100),
                output_tokens: Some(50),
                estimated_cost_usd: None,
                latency_ms: Some(10),
            }),
            received_at: Utc::now(),
        })
    }
}

#[test]
fn test_six_step_flow_via_runtime_tool_surface() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let store = GitCanonicalStore::new(root);
    store.init_layout().unwrap();
    let index = DerivedIndex::open(root).unwrap();
    let mut registry = ProviderRegistry::default();
    registry.register(Arc::new(MockAdapter));

    let surface = RuntimeToolSurface {
        store: &store,
        index: &index,
        provider_service: &registry,
    };

    // SETUP: Define System, Class, Instruction, Projection, Pattern

    let note = StoredObject::new(
        Kind::Object,
        Some("source_note".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown("seed body"),
        vec![],
    );
    let note_ref = store.write_object(&note).unwrap();

    let output_class = ClassDefinition {
        name: "finding".to_string(),
        version: "1".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    let class_obj = StoredObject::new(
        Kind::Object,
        Some("class_definition".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&output_class).unwrap()),
        vec![],
    );
    let class_ref = store.write_object(&class_obj).unwrap();

    let compiled_context = CompiledContextTemplate {
        name: "surface".to_string(),
        version: "1".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["source_note".to_string()],
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
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&compiled_context).unwrap()),
        vec![],
    );
    let proj_ref = store.write_object(&proj_obj).unwrap();

    let instruction = InstructionPayload {
        name: "extract".to_string(),
        version: "1".to_string(),
        purpose: "Extract findings".to_string(),
        input_classes: vec!["source_note".to_string()],
        output_classes: vec!["finding".to_string()],
        execution_policy: "runtime_permitted".to_string(),
        provider_profile: None,
        trace_policy: "summary".to_string(),
        register: "finding".to_string(),
        body: MarkdownBody::new("Extract findings.".to_string()),
    };
    let instr_obj = StoredObject::new(
        Kind::Instruction,
        Some("instruction".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_markdown(instruction.to_markdown().unwrap()),
        vec![],
    );
    let instr_ref = store.write_object(&instr_obj).unwrap();

    let workflow_yaml = format!(
        r#"name: flow
version: "1"
operations:
  - id: op_project
    kind: compile_context
    input_contracts: []
    output_contracts: [work_surface]
    instruction: null
    compiled_context:
      id: {}
      version_id: {}
    policy: null
    provider_profile: null
  - id: op_transform
    kind: transform
    input_contracts: [work_surface]
    output_contracts: [finding]
    instruction:
      id: {}
      version_id: {}
    compiled_context: null
    policy: null
    provider_profile: null
edges:
  - from: op_project
    to: op_transform
    condition: null
guards: []
"#,
        proj_ref.id.as_str(),
        proj_ref.version_id.as_str(),
        instr_ref.id.as_str(),
        instr_ref.version_id.as_str()
    );
    let workflow_obj = StoredObject::new(
        Kind::Workflow,
        Some("composition_workflow".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_yaml(workflow_yaml),
        vec![],
    );
    let workflow_ref = store.write_object(&workflow_obj).unwrap();

    let system = SystemDefinition {
        system_id: "sys".to_string(),
        namespace: "sys".to_string(),
        title: "Sys".to_string(),
        description: None,
        classes: vec![VersionRef::new(class_ref.id, class_ref.version_id)],
        instructions: vec![VersionRef::new(instr_ref.id, instr_ref.version_id)],
        policies: vec![],
        workflows: vec![VersionRef::new(
            workflow_ref.id.clone(),
            workflow_ref.version_id.clone(),
        )],
        compiled_contexts: vec![VersionRef::new(proj_ref.id, proj_ref.version_id)],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        standing_dimensions: vec![],
        runtime_profile: RuntimeProfile {
            execution_surface: "runtime".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };
    let system_obj = StoredObject::new(
        Kind::SystemDefinition,
        Some("system_definition".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        StoredPayload::from_yaml(to_yaml(&system).unwrap()),
        vec![],
    );
    let system_ref = store.write_object(&system_obj).unwrap();
    let system_vref = VersionRef::new(system_ref.id, system_ref.version_id);

    index.rebuild_from_store(&store).unwrap();

    // EXECUTION

    let outcome = surface
        .run_workflow(WorkflowRunRequest {
            run_id: "run_1".to_string(),
            system_definition: system_vref,
            workflow: VersionRef::new(workflow_ref.id, workflow_ref.version_id),
            inputs: vec![ObjectRef::new(
                note_ref.id,
                note_ref.version_id,
                Kind::Object,
                Some("source_note".to_string()),
            )],
            handoff_manifest: None,
            transition_assignment: None,
            operator_approved: true,
        })
        .unwrap();

    assert_eq!(outcome.record.run_id, "run_1");
    assert_eq!(outcome.emitted_objects.len(), 1);
    assert_eq!(
        outcome.emitted_objects[0].class,
        Some("finding".to_string())
    );
}
