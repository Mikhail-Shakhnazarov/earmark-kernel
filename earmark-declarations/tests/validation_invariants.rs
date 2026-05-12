use earmark_core::*;
use earmark_declarations::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_validate_standing_tokens_against_registry(
        token in "[a-z]{1,20}"
    ) {
        let registry = StandingRegistry::kernel_defaults();
        let valid_tokens = [
            "unresolved", "working", "supported", "contested", "superseded",
            "unreviewed", "pending", "accepted", "rejected",
            "active", "blocked", "completed", "archived",
        ];
        let is_valid = valid_tokens.contains(&token.as_str());

        // Test against kernel:epistemic
        let epi_policy = StandingPolicy {
            name: "test_policy".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            transition_rules: vec![StandingTransitionRule {
                dimension: "kernel:epistemic".to_string(),
                from: vec![token.clone()],
                to: vec![token.clone()],
                requires_review: false,
            }],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };
        let result = validate_standing_policy_against_registry(&epi_policy, &registry);
        if is_valid && valid_tokens[..5].contains(&token.as_str()) {
            assert!(result.is_ok(), "Expected token '{}' to be valid for kernel:epistemic", token);
        } else if !is_valid || !valid_tokens[..5].contains(&token.as_str()) {
            assert!(result.is_err(), "Expected token '{}' to be invalid for kernel:epistemic", token);
        }

        // Test against kernel:review
        let rev_policy = StandingPolicy {
            name: "test_policy".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            transition_rules: vec![StandingTransitionRule {
                dimension: "kernel:review".to_string(),
                from: vec![token.clone()],
                to: vec![token.clone()],
                requires_review: false,
            }],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };
        let result = validate_standing_policy_against_registry(&rev_policy, &registry);
        if is_valid && valid_tokens[5..9].contains(&token.as_str()) {
            assert!(result.is_ok(), "Expected token '{}' to be valid for kernel:review", token);
        } else if !is_valid || !valid_tokens[5..9].contains(&token.as_str()) {
            assert!(result.is_err(), "Expected token '{}' to be invalid for kernel:review", token);
        }
    }

    #[test]
    fn test_validate_workflow_operation_ids(id in "[a-z][a-z0-9_]{0,63}") {
        let op = WorkflowDeclarationOperation {
            id: id.clone(),
            kind: "nop".to_string(),
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: None,
            compiled_context: None,
            policy: None,
            provider_profile: None,
        };
        let workflow = WorkflowDeclaration {
            name: "test_workflow".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            operations: vec![op],
            edges: vec![],
            guards: vec![],
            output_contracts: vec![],
        };
        assert!(validate_workflow_definition(&workflow).is_ok());
    }
}
