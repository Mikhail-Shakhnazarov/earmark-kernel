use std::collections::BTreeMap;

use earmark_core::{
    ClassDefinition, ClassStandingRules, CompiledContextExpansion, CompiledContextRender,
    CompiledContextSelect, CompiledContextTemplate, CompiledContextVisibility, JsonSchemaRef,
    OperationRequirement, StandingPolicy, StandingTransitionRule, WorkflowDefinition,
    WorkflowEdge, WorkflowGuard, WorkflowOperation,
};
use earmark_declarations::{
    validate_class_definition, validate_compiled_context_template, validate_system_definition,
    validate_workflow_definition,
};
use earmark_store::{CanonicalStore, GitCanonicalStore, StoredObject, StoredPayload};
use tempfile::tempdir;

fn base_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        name: "wf_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        operations: vec![WorkflowOperation {
            id: "op_a".to_string(),
            kind: "transform".to_string(),
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: Some(earmark_core::VersionRef::new(
                earmark_core::ObjectId::new(),
                earmark_core::VersionId::new(),
            )),
            compiled_context: None,
            policy: None,
            provider_profile: None,
        }],
        edges: vec![],
        guards: vec![],
    }
}

#[test]
fn workflow_rejects_duplicate_operation_ids() {
    let mut wf = base_workflow();
    wf.operations.push(wf.operations[0].clone());
    assert!(validate_workflow_definition(&wf).is_err());
}

#[test]
fn workflow_rejects_unknown_guard_reference() {
    let mut wf = base_workflow();
    wf.edges.push(WorkflowEdge {
        from: "op_a".to_string(),
        to: "op_a".to_string(),
        condition: Some("guard_missing".to_string()),
    });
    wf.guards.push(WorkflowGuard {
        id: "guard_ok".to_string(),
        expression: "true".to_string(),
        message: None,
    });
    assert!(validate_workflow_definition(&wf).is_err());
}

#[test]
fn workflow_transform_requires_instruction() {
    let mut wf = base_workflow();
    wf.operations[0].instruction = None;
    assert!(validate_workflow_definition(&wf).is_err());
}

#[test]
fn class_rejects_invalid_relation_type() {
    let class = ClassDefinition {
        name: "finding".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "bad-type".to_string(),
            target_classes: vec!["source_note".to_string()],
            direction: None,
        }],
        validators: vec![],
    };
    assert!(validate_class_definition(&class).is_err());
}

#[test]
fn compiled_context_rejects_unknown_standing_dimension() {
    let template = CompiledContextTemplate {
        name: "ctx_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["finding".to_string()],
            standing: BTreeMap::from([("quality".to_string(), vec!["accepted".to_string()])]),
            relations: vec!["derived_from".to_string()],
            time_range: None,
        expansion: CompiledContextExpansion::default(),
        },
        group_by: vec![],
        render: CompiledContextRender {
            mode: "manifest".to_string(),
            manifest_format: None,
            prose_template: None,
        },
        visibility: CompiledContextVisibility {
            include_lineage: true,
            include_constraints: true,
            include_provenance: true,
        },
    };
    assert!(validate_compiled_context_template(&template).is_err());
}

#[test]
fn system_validation_rejects_wrong_kind_reference() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();

    let wrong_payload = StoredPayload::from_yaml(
        earmark_core::to_yaml(&earmark_core::ProviderProfile {
            name: "provider_a".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            provider: "x".to_string(),
            model: "y".to_string(),
            endpoint_env: None,
            auth_env: None,
            budget: earmark_core::ProviderBudget {
                max_input_tokens: None,
                max_output_tokens: None,
                max_cost_usd: None,
                max_latency_ms: None,
            },
            allowed_operations: vec![],
            exposure: earmark_core::ProviderExposure {
                allow_prose_objects: true,
                allow_structured_declarations: true,
                allow_work_surface_only: false,
                allow_export_requests: false,
            },
            response_contract: earmark_core::ProviderResponseContract {
                format: "json".to_string(),
                must_return_candidate_only: false,
                must_include_lineage: false,
            },
        })
        .unwrap(),
    );
    let stored = StoredObject::new(
        earmark_core::Kind::ProviderProfile,
        Some("provider_profile".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        wrong_payload,
        vec![],
    );
    let wrong_ref = store.write_object(&stored).unwrap();

    let system = earmark_core::SystemDefinition {
        system_id: "sys_test".to_string(),
        namespace: "systems/test".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![wrong_ref],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "runtime_over_folder".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };

    let err = validate_system_definition(&store, &system).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("wrong envelope kind"));
}

#[test]
fn standing_policy_rejects_invalid_transition_dimension_and_tokens() {
    let policy = StandingPolicy {
        name: "policy_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "quality".to_string(),
            from: vec!["accepted".to_string()],
            to: vec!["rejected".to_string()],
            requires_review: false,
        }],
        operation_requirements: vec![],
        escalations: vec![],
        rationale: None,
    };
    assert!(earmark_declarations::validate_standing_policy(&policy).is_err());
}

#[test]
fn standing_policy_rejects_invalid_minimums_and_forbidden_tokens() {
    let policy = StandingPolicy {
        name: "policy_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "review".to_string(),
            from: vec!["bad_token".to_string()],
            to: vec!["accepted".to_string()],
            requires_review: false,
        }],
        operation_requirements: vec![OperationRequirement {
            operation: "export".to_string(),
            minimums: BTreeMap::from([("review".to_string(), "invalid".to_string())]),
            forbidden: BTreeMap::from([("process".to_string(), vec!["bad".to_string()])]),
        }],
        escalations: vec![],
        rationale: None,
    };
    assert!(earmark_declarations::validate_standing_policy(&policy).is_err());
}

#[test]
fn system_validation_rejects_wrong_class_marker_for_class_reference() {
    let dir = tempdir().unwrap();
    let store = GitCanonicalStore::new(dir.path());
    store.init_layout().unwrap();

    let class_payload = StoredPayload::from_yaml(
        earmark_core::to_yaml(&ClassDefinition {
            name: "finding".to_string(),
            version: "1.0.0".to_string(),
            kind: "object".to_string(),
            required_headers: vec![],
            payload_schema: JsonSchemaRef("inline:any".to_string()),
            standing_rules: ClassStandingRules::default(),
            relation_rules: vec![],
            validators: vec![],
        })
        .unwrap(),
    );
    let wrong_marker = StoredObject::new(
        earmark_core::Kind::Object,
        Some("not_class_definition".to_string()),
        earmark_core::Standing::default(),
        earmark_core::Provenance::direct_input("test"),
        BTreeMap::new(),
        class_payload,
        vec![],
    );
    let wrong_ref = store.write_object(&wrong_marker).unwrap();

    let system = earmark_core::SystemDefinition {
        system_id: "sys_test".to_string(),
        namespace: "systems/test".to_string(),
        title: "Test".to_string(),
        description: None,
        classes: vec![wrong_ref],
        instructions: vec![],
        policies: vec![],
        workflows: vec![],
        compiled_contexts: vec![],
        provider_profiles: vec![],
        default_compiled_context: None,
        default_provider_profile: None,
        runtime_profile: earmark_core::RuntimeProfile {
            execution_surface: "runtime_over_folder".to_string(),
            machine_output_default: "json".to_string(),
            work_surface_mode: "materialized_manifest".to_string(),
        },
        activated_at: None,
    };

    let err = validate_system_definition(&store, &system).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("wrong class marker"));
}
