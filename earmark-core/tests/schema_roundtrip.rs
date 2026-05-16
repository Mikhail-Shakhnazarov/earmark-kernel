use std::collections::BTreeMap;

use earmark_core::*;

#[test]
fn envelope_roundtrip() {
    let payload_ref = PayloadRef::from_bytes(br#"{"body":"hello"}"#);
    let mut headers = BTreeMap::new();
    headers.insert("title".to_string(), HeaderValue::String("Test".to_string()));

    let envelope = Envelope {
        id: ObjectId::new(),
        version_id: VersionId::new(),
        kind: Kind::Object,
        class: Some("note".to_string()),
        standing: Standing::default(),
        provenance: Provenance::direct_input("operator"),
        headers,
        payload_ref,
        parents: vec![],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&envelope).unwrap();
    let parsed: Envelope = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.kind, Kind::Object);
    assert_eq!(parsed.class.as_deref(), Some("note"));
}

#[test]
fn standing_and_provenance_roundtrip() {
    let standing = Standing::default();
    let json = serde_json::to_string(&standing).unwrap();
    let parsed: Standing = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed
            .get(&DimensionId::new("kernel:review"))
            .map(TokenId::as_str),
        Some("unreviewed")
    );

    let provenance = Provenance::direct_input("operator");
    let json = serde_json::to_string(&provenance).unwrap();
    let parsed: Provenance = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.actor, "operator");
}

#[test]
fn relation_payload_roundtrip() {
    let obj = ObjectRef::new(
        ObjectId::new(),
        VersionId::new(),
        Kind::Object,
        Some("note".to_string()),
    );

    let payload = RelationPayload {
        source: obj.clone(),
        target: obj,
        relation_type: "supports".to_string(),
        qualifiers: BTreeMap::new(),
        scope: Some("testing".to_string()),
    };

    let json = serde_json::to_string(&payload).unwrap();
    let parsed: RelationPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.relation_type, "supports");
    assert_eq!(parsed.scope.as_deref(), Some("testing"));
}

#[test]
fn instruction_markdown_parse_roundtrip() {
    let input = r#"---
name: compose_status_summary
version: "3"
purpose: Compose a bounded status summary from supplied material
input_classes:
  - note
  - project
output_classes:
  - status_summary
execution_policy: runtime_propose_operator_ratify
provider_profile: null
trace_policy: summary
register: machined
---

Compose a status summary from the supplied objects.
"#;

    let parsed = InstructionPayload::parse_markdown(input).unwrap();
    assert_eq!(parsed.name, "compose_status_summary");
    assert!(parsed.body.as_str().contains("Compose a status summary"));

    let rendered = parsed.to_markdown().unwrap();
    let reparsed = InstructionPayload::parse_markdown(&rendered).unwrap();
    assert_eq!(reparsed.name, "compose_status_summary");
}

#[test]
fn declaration_yaml_parse_roundtrip() {
    let workflow: WorkflowDefinition = parse_yaml(
        r#"name: status_summary_flow
version: "1"
description: test
operations:
  - id: op_project
    kind: compile_context
    input_contracts: []
    output_contracts: [work_surface]
    instruction: null
    compiled_context: null
    policy: null
    provider_profile: null
edges: []
guards: []
"#,
    )
    .unwrap();
    assert_eq!(workflow.operations.len(), 1);

    let policy: StandingPolicy = parse_yaml(
        r#"name: default_export_policy
version: "1"
description: Default policy
transition_rules: []
operation_requirements: []
escalations: []
rationale: ok
"#,
    )
    .unwrap();
    assert_eq!(policy.name, "default_export_policy");

    let compiled_context: CompiledContextTemplate = parse_yaml(
        r#"name: status_surface
version: "1"
description: CompiledContext
select:
  classes: [note]
  standing: {}
  relations: []
  time_range: null
group_by: []
render:
  mode: work_surface_materialization
  manifest_format: json
  prose_template: null
visibility:
  include_lineage: true
  include_constraints: false
  include_provenance: true
"#,
    )
    .unwrap();
    assert_eq!(compiled_context.render.mode, "work_surface_materialization");

    let provider: ProviderProfile = parse_yaml(
        r#"name: gemma_free_tier
version: "1"
description: test
provider: google
model: gemma
endpoint_env: GEMMA_API_ENDPOINT
auth_env: GEMMA_API_KEY
budget:
  max_input_tokens: 16000
  max_output_tokens: 4000
  max_cost_usd: 0.0
  max_latency_ms: 30000
allowed_operations: [transform]
exposure:
  allow_prose_objects: true
  allow_structured_declarations: true
  allow_work_surface_only: true
  allow_export_requests: false
response_contract:
  format: json
  must_return_candidate_only: true
  must_include_lineage: true
"#,
    )
    .unwrap();
    assert_eq!(provider.provider, "google");

    let system: SystemDefinition = parse_yaml(
        r#"system_id: pkm-core
namespace: systems/pkm-core
title: PKM Core
description: Example
classes: []
instructions: []
policies: []
workflows: []
compiled_contexts: []
provider_profiles: []
default_compiled_context: null
default_provider_profile: null
runtime_profile:
  execution_surface: runtime_over_folder
  machine_output_default: json
  work_surface_mode: materialized_manifest
activated_at: null
"#,
    )
    .unwrap();
    assert_eq!(system.system_id, "pkm-core");
}

#[test]
fn runtime_profile_rejects_unknown_fields() {
    let parsed = parse_yaml::<SystemDefinition>(
        r#"system_id: pkm-core
namespace: systems/pkm-core
title: PKM Core
description: Example
classes: []
instructions: []
policies: []
workflows: []
compiled_contexts: []
provider_profiles: []
default_compiled_context: null
default_provider_profile: null
runtime_profile:
  execution_surface: runtime_over_folder
  machine_output_default: json
  work_surface_mode: materialized_manifest
  unknown_field: true
activated_at: null
"#,
    );
    assert!(parsed.is_err());
}
