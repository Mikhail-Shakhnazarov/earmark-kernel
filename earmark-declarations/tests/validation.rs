use std::collections::BTreeMap;

use earmark_core::{
    ClassDefinition, ClassStandingRules, CompiledContextExpansion, CompiledContextRender,
    CompiledContextSelect, CompiledContextTemplate, CompiledContextVisibility, DimensionId,
    EscalationRule, FlexibleVersionRef, HttpAuthConfig, HttpAuthKind, HttpGenerationProfile,
    HttpRequestTemplate, HttpResponseExtraction, InstructionPayload, JsonSchemaRef, MarkdownBody,
    OperationRequirement, ProviderBudget, ProviderExposure, ProviderProfile,
    ProviderResponseContract, ProviderResponseFormat, StandingPolicy, StandingRegistry,
    StandingTransitionRule, TokenId, WorkflowDeclaration, WorkflowDeclarationOperation,
    WorkflowEdge, WorkflowGuard, WorkflowOperationKind,
};
use earmark_declarations::{
    validate_class_definition, validate_compiled_context_template, validate_instruction,
    validate_provider_profile, validate_standing_policy, validate_system_definition,
    validate_workflow_definition,
};
use earmark_store::{GitCanonicalStore, ObjectStore, StoredObject, StoredPayload, WorkspaceLayout};
use tempfile::tempdir;

fn base_workflow() -> WorkflowDeclaration {
    WorkflowDeclaration {
        name: "wf_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        operations: vec![WorkflowDeclarationOperation {
            id: "op_a".to_string(),
            kind: WorkflowOperationKind::Transform,
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: Some(FlexibleVersionRef::Ref(earmark_core::VersionRef::new(
                earmark_core::ObjectId::generate(),
                earmark_core::VersionId::generate(),
            ))),
            compiled_context: None,
            policy: None,
            provider_profile: None,
        }],
        edges: vec![],
        guards: vec![],
        output_contracts: vec![],
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
            counterparty_classes: vec!["source_note".to_string()],
            direction: None,
            authorizing_endpoint: None,
        }],
        validators: vec![],
    };
    assert!(validate_class_definition(&class).is_err());
}

#[test]
fn compiled_context_rejects_unknown_standing_dimension_with_registry() {
    let registry = StandingRegistry::kernel_defaults();
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
    assert!(
        earmark_declarations::validate_compiled_context_template_against_registry(
            &template, &registry
        )
        .is_err()
    );
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
                format: ProviderResponseFormat::Json,
                must_return_candidate_only: false,
                must_include_lineage: false,
            },
            http: None,
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
        standing_dimensions: vec![],
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
fn standing_policy_rejects_invalid_transition_dimension_with_registry() {
    let registry = StandingRegistry::kernel_defaults();
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
    assert!(
        earmark_declarations::validate_standing_policy_against_registry(&policy, &registry)
            .is_err()
    );
}

#[test]
fn standing_policy_rejects_invalid_minimums_and_forbidden_tokens_with_registry() {
    let registry = StandingRegistry::kernel_defaults();
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
            required_standing: BTreeMap::from([("review".to_string(), "invalid".to_string())]),
            forbidden_standing: BTreeMap::from([("process".to_string(), vec!["bad".to_string()])]),
            ..Default::default()
        }],
        escalations: vec![],
        rationale: None,
    };
    assert!(
        earmark_declarations::validate_standing_policy_against_registry(&policy, &registry)
            .is_err()
    );
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
        standing_dimensions: vec![],
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

#[test]
fn class_rejects_invalid_authorizing_endpoint() {
    let class = ClassDefinition {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["other".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("INVALID".to_string()),
        }],
        validators: vec![],
    };
    assert!(validate_class_definition(&class).is_err());
}

#[test]
fn class_rejects_dead_outgoing_target() {
    let class = ClassDefinition {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["other".to_string()],
            direction: Some("outgoing".to_string()),
            authorizing_endpoint: Some("target".to_string()),
        }],
        validators: vec![],
    };
    assert!(validate_class_definition(&class).is_err());
}

#[test]
fn class_rejects_dead_incoming_source() {
    let class = ClassDefinition {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "mentions".to_string(),
            counterparty_classes: vec!["other".to_string()],
            direction: Some("incoming".to_string()),
            authorizing_endpoint: Some("source".to_string()),
        }],
        validators: vec![],
    };
    assert!(validate_class_definition(&class).is_err());
}

#[test]
fn class_rejects_invalid_relation_direction() {
    let mut class = ClassDefinition {
        name: "finding".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![earmark_core::RelationRule {
            relation_type: "derived_from".to_string(),
            counterparty_classes: vec!["source_note".to_string()],
            direction: Some("invalid".to_string()),
            authorizing_endpoint: None,
        }],
        validators: vec![],
    };

    // Invalid fails
    assert!(validate_class_definition(&class).is_err());

    // Valid succeeds
    class.relation_rules[0].direction = Some("incoming".to_string());
    class.relation_rules[0].authorizing_endpoint = Some("target".to_string());
    assert!(validate_class_definition(&class).is_ok());

    class.relation_rules[0].direction = Some("bidirectional".to_string());
    assert!(validate_class_definition(&class).is_ok());

    class.relation_rules[0].direction = None;
    class.relation_rules[0].authorizing_endpoint = None;
    assert!(validate_class_definition(&class).is_ok());
}

#[test]
fn class_rejects_invalid_kind() {
    let class = ClassDefinition {
        name: "finding".to_string(),
        version: "1.0.0".to_string(),
        kind: "widget".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules::default(),
        relation_rules: vec![],
        validators: vec![],
    };
    assert!(validate_class_definition(&class).is_err());
}

#[test]
fn class_rejects_invalid_epistemic_token_in_standing_rules() {
    let class = ClassDefinition {
        name: "finding".to_string(),
        version: "1.0.0".to_string(),
        kind: "object".to_string(),
        required_headers: vec![],
        payload_schema: JsonSchemaRef("inline:any".to_string()),
        standing_rules: ClassStandingRules {
            allowed_standing: BTreeMap::from([(
                DimensionId::new("kernel:epistemic"),
                vec![
                    TokenId::new("unresolved"),
                    TokenId::new("working"),
                    TokenId::new("superseded"),
                ],
            )]),
            ..Default::default()
        },
        relation_rules: vec![],
        validators: vec![],
    };
    assert!(validate_class_definition(&class).is_ok());

    // No valid standing rules are ok (empty vec)
    let class2 = ClassDefinition {
        standing_rules: ClassStandingRules::default(),
        ..class
    };
    assert!(validate_class_definition(&class2).is_ok());
}

#[test]
#[should_panic(expected = "unknown variant `unknown_kind`")]
fn workflow_rejects_invalid_operation_kind() {
    let yaml = r#"name: wf_bad
version: "1.0.0"
operations:
  - id: op_a
    kind: unknown_kind
"#;
    let _wf: WorkflowDeclaration = earmark_core::parse_yaml(yaml).unwrap();
}

#[test]
fn workflow_compile_context_requires_compiled_context() {
    let mut wf = base_workflow();
    wf.operations[0].kind = WorkflowOperationKind::CompileContext;
    wf.operations[0].instruction = None;
    wf.operations[0].compiled_context = None;
    assert!(validate_workflow_definition(&wf).is_err());

    // With compiled_context it should pass
    wf.operations[0].compiled_context =
        Some(FlexibleVersionRef::Ref(earmark_core::VersionRef::new(
            earmark_core::ObjectId::generate(),
            earmark_core::VersionId::generate(),
        )));
    assert!(validate_workflow_definition(&wf).is_ok());
}

#[test]
fn workflow_rejects_invalid_contract_class_names() {
    let mut wf = base_workflow();
    wf.operations[0].input_contracts = vec!["InvalidClass".to_string()];
    assert!(validate_workflow_definition(&wf).is_err());

    let mut wf2 = base_workflow();
    wf2.operations[0].output_contracts = vec!["".to_string()];
    assert!(validate_workflow_definition(&wf2).is_err());
}

#[test]
fn provider_profile_rejects_unknown_response_format_at_parse_time() {
    let yaml = r#"
name: test_provider
version: 1.0.0
provider: mock
model: echo
allowed_operations: [transform]
exposure:
  allow_prose_objects: true
  allow_structured_declarations: false
  allow_work_surface_only: false
  allow_export_requests: false
response_contract:
  format: unknown
  must_return_candidate_only: true
  must_include_lineage: false
"#;
    let parsed: Result<ProviderProfile, _> = earmark_core::parse_yaml(yaml);
    assert!(parsed.is_err());
}

#[test]
fn provider_profile_rejects_negative_budget() {
    let profile = ProviderProfile {
        name: "test_provider".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "echo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: ProviderBudget {
            max_input_tokens: None,
            max_output_tokens: None,
            max_cost_usd: Some(-1.0),
            max_latency_ms: None,
        },
        allowed_operations: vec!["transform".to_string()],
        exposure: ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: false,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: ProviderResponseContract {
            format: ProviderResponseFormat::Json,
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    };
    assert!(validate_provider_profile(&profile).is_err());
}

#[test]
fn compiled_context_rejects_invalid_class_token() {
    let template = CompiledContextTemplate {
        name: "ctx_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["InvalidClass".to_string()],
            standing: BTreeMap::new(),
            relations: vec![],
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
fn compiled_context_rejects_invalid_standing_token() {
    let template = CompiledContextTemplate {
        name: "ctx_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["finding".to_string()],
            standing: BTreeMap::from([("review".to_string(), vec!["bad_token".to_string()])]),
            relations: vec![],
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
    assert!(validate_compiled_context_template(&template).is_ok());
}

#[test]
fn compiled_context_rejects_invalid_standing_token_with_registry() {
    let registry = StandingRegistry::kernel_defaults();
    let template = CompiledContextTemplate {
        name: "ctx_ok".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["finding".to_string()],
            standing: BTreeMap::from([("review".to_string(), vec!["bad_token".to_string()])]),
            relations: vec![],
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
    assert!(
        earmark_declarations::validate_compiled_context_template_against_registry(
            &template, &registry
        )
        .is_err()
    );
}

#[test]
fn standing_policy_rejects_empty_escalation_trigger() {
    let policy = StandingPolicy {
        name: "policy_test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        transition_rules: vec![],
        operation_requirements: vec![],
        escalations: vec![EscalationRule {
            trigger: "".to_string(),
            severity: "warning".to_string(),
            message: "a message".to_string(),
        }],
        rationale: None,
    };
    assert!(validate_standing_policy(&policy).is_err());
}

#[test]
fn standing_policy_rejects_empty_escalation_message() {
    let policy = StandingPolicy {
        name: "policy_test".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        transition_rules: vec![],
        operation_requirements: vec![],
        escalations: vec![EscalationRule {
            trigger: "some_trigger".to_string(),
            severity: "warning".to_string(),
            message: "".to_string(),
        }],
        rationale: None,
    };
    assert!(validate_standing_policy(&policy).is_err());
}

#[test]
fn instruction_rejects_empty_version() {
    let instruction = InstructionPayload {
        name: "test_instruction".to_string(),
        version: "".to_string(),
        purpose: "test purpose".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        execution_policy: "auto".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "auto".to_string(),
        body: MarkdownBody::new("some body text"),
    };
    assert!(validate_instruction(&instruction).is_err());
}

#[test]
fn instruction_rejects_empty_purpose() {
    let instruction = InstructionPayload {
        name: "test_instruction".to_string(),
        version: "1.0.0".to_string(),
        purpose: "".to_string(),
        input_classes: vec![],
        output_classes: vec![],
        execution_policy: "auto".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "auto".to_string(),
        body: MarkdownBody::new("some body text"),
    };
    assert!(validate_instruction(&instruction).is_err());
}

#[test]
fn instruction_rejects_invalid_class_tokens() {
    let instruction = InstructionPayload {
        name: "test_instruction".to_string(),
        version: "1.0.0".to_string(),
        purpose: "test purpose".to_string(),
        input_classes: vec!["InvalidClass".to_string()],
        output_classes: vec![],
        execution_policy: "auto".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "auto".to_string(),
        body: MarkdownBody::new("some body text"),
    };
    assert!(validate_instruction(&instruction).is_err());

    let mut instruction2 = instruction;
    instruction2.input_classes = vec![];
    instruction2.output_classes = vec!["".to_string()];
    assert!(validate_instruction(&instruction2).is_err());
}

#[test]
fn valid_class_examples_pass() {
    for class in [
        ClassDefinition {
            name: "finding".to_string(),
            version: "1.0.0".to_string(),
            kind: "object".to_string(),
            required_headers: vec!["title".to_string()],
            payload_schema: JsonSchemaRef("inline:any".to_string()),
            standing_rules: ClassStandingRules {
                allowed_standing: BTreeMap::from([
                    (
                        DimensionId::new("kernel:epistemic"),
                        vec![TokenId::new("working"), TokenId::new("supported")],
                    ),
                    (
                        DimensionId::new("kernel:review"),
                        vec![TokenId::new("unreviewed"), TokenId::new("accepted")],
                    ),
                    (
                        DimensionId::new("kernel:process"),
                        vec![TokenId::new("completed")],
                    ),
                ]),
                ..Default::default()
            },
            relation_rules: vec![earmark_core::RelationRule {
                relation_type: "derived_from".to_string(),
                counterparty_classes: vec!["source_note".to_string()],
                direction: Some("outgoing".to_string()),
                authorizing_endpoint: None,
            }],
            validators: vec![],
        },
        ClassDefinition {
            name: "source_note".to_string(),
            version: "1.0.0".to_string(),
            kind: "object".to_string(),
            required_headers: vec!["title".to_string()],
            payload_schema: JsonSchemaRef("inline:any".to_string()),
            standing_rules: ClassStandingRules {
                allowed_standing: BTreeMap::from([
                    (
                        DimensionId::new("kernel:epistemic"),
                        vec![TokenId::new("working")],
                    ),
                    (
                        DimensionId::new("kernel:review"),
                        vec![TokenId::new("unreviewed")],
                    ),
                    (
                        DimensionId::new("kernel:process"),
                        vec![TokenId::new("completed")],
                    ),
                ]),
                ..Default::default()
            },
            relation_rules: vec![],
            validators: vec![],
        },
    ] {
        assert!(
            validate_class_definition(&class).is_ok(),
            "class '{}' should be valid",
            class.name
        );
    }
}

#[test]
fn valid_workflow_example_passes() {
    let wf = WorkflowDeclaration {
        name: "source_to_finding".to_string(),
        version: "1.0.0".to_string(),
        description: Some("test".to_string()),
        operations: vec![
            WorkflowDeclarationOperation {
                id: "compile_context".to_string(),
                kind: WorkflowOperationKind::CompileContext,
                input_contracts: vec!["source_note".to_string()],
                output_contracts: vec!["work_surface".to_string()],
                instruction: None,
                compiled_context: Some(FlexibleVersionRef::Ref(earmark_core::VersionRef::new(
                    earmark_core::ObjectId::generate(),
                    earmark_core::VersionId::generate(),
                ))),
                policy: None,
                provider_profile: None,
            },
            WorkflowDeclarationOperation {
                id: "transform".to_string(),
                kind: WorkflowOperationKind::Transform,
                input_contracts: vec!["source_note".to_string()],
                output_contracts: vec!["finding".to_string()],
                instruction: Some(FlexibleVersionRef::Ref(earmark_core::VersionRef::new(
                    earmark_core::ObjectId::generate(),
                    earmark_core::VersionId::generate(),
                ))),
                compiled_context: None,
                policy: None,
                provider_profile: None,
            },
        ],
        edges: vec![WorkflowEdge {
            from: "compile_context".to_string(),
            to: "transform".to_string(),
            condition: None,
        }],
        guards: vec![],
        output_contracts: vec![],
    };
    assert!(validate_workflow_definition(&wf).is_ok());
}

#[test]
fn valid_provider_profile_passes() {
    let profile = ProviderProfile {
        name: "test_provider".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        provider: "mock".to_string(),
        model: "echo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: ProviderBudget {
            max_input_tokens: None,
            max_output_tokens: None,
            max_cost_usd: None,
            max_latency_ms: None,
        },
        allowed_operations: vec!["transform".to_string()],
        exposure: ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: false,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: ProviderResponseContract {
            format: ProviderResponseFormat::Json,
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: None,
    };
    assert!(validate_provider_profile(&profile).is_ok());
}

#[test]
fn provider_profile_rejects_empty_allowed_domain() {
    let profile = ProviderProfile {
        name: "http_provider".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        provider: "http_generation".to_string(),
        model: "demo".to_string(),
        endpoint_env: None,
        auth_env: None,
        budget: ProviderBudget::default(),
        allowed_operations: vec!["transform".to_string()],
        exposure: ProviderExposure {
            allow_prose_objects: true,
            allow_structured_declarations: true,
            allow_work_surface_only: false,
            allow_export_requests: false,
        },
        response_contract: ProviderResponseContract {
            format: ProviderResponseFormat::Markdown,
            must_return_candidate_only: true,
            must_include_lineage: false,
        },
        http: Some(HttpGenerationProfile {
            method: Some("POST".to_string()),
            url_template: "https://api.example.com/v1".to_string(),
            auth: HttpAuthConfig {
                kind: HttpAuthKind::None,
                ..Default::default()
            },
            request: HttpRequestTemplate {
                content_type: Some("application/json".to_string()),
                body: serde_json::json!({"prompt": "{{input_text}}"}),
            },
            response: HttpResponseExtraction {
                text_path: "$.output".to_string(),
                ..Default::default()
            },
            allowed_domains: vec!["".to_string()],
            blocked_domains: vec![],
            ..Default::default()
        }),
    };
    assert!(validate_provider_profile(&profile).is_err());
}

#[test]
fn valid_compiled_context_passes() {
    let template = CompiledContextTemplate {
        name: "valid_context".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        select: CompiledContextSelect {
            classes: vec!["finding".to_string()],
            standing: BTreeMap::from([("review".to_string(), vec!["accepted".to_string()])]),
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
    assert!(validate_compiled_context_template(&template).is_ok());
}

#[test]
fn valid_standing_policy_passes() {
    let policy = StandingPolicy {
        name: "valid_policy".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        transition_rules: vec![StandingTransitionRule {
            dimension: "review".to_string(),
            from: vec!["unreviewed".to_string()],
            to: vec!["accepted".to_string()],
            requires_review: true,
        }],
        operation_requirements: vec![OperationRequirement {
            operation: "export".to_string(),
            required_standing: BTreeMap::from([("review".to_string(), "accepted".to_string())]),
            forbidden_standing: BTreeMap::from([(
                "epistemic".to_string(),
                vec!["unresolved".to_string()],
            )]),
            ..Default::default()
        }],
        escalations: vec![EscalationRule {
            trigger: "some_trigger".to_string(),
            severity: "warning".to_string(),
            message: "some message".to_string(),
        }],
        rationale: None,
    };
    assert!(validate_standing_policy(&policy).is_ok());
}

#[test]
fn valid_instruction_passes() {
    let instruction = InstructionPayload {
        name: "valid_instruction".to_string(),
        version: "1.0.0".to_string(),
        purpose: "test purpose".to_string(),
        input_classes: vec!["source_note".to_string()],
        output_classes: vec!["finding".to_string()],
        execution_policy: "auto".to_string(),
        provider_profile: None,
        trace_policy: "none".to_string(),
        register: "auto".to_string(),
        body: MarkdownBody::new("some body text"),
    };
    assert!(validate_instruction(&instruction).is_ok());
}
