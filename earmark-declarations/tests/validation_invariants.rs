use earmark_declarations::*;
use earmark_core::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_validate_standing_tokens(
        dim in prop_oneof![
            Just(StandingDimension::Epistemic),
            Just(StandingDimension::Review),
            Just(StandingDimension::Process)
        ],
        token in "[a-z]{1,20}"
    ) {
        // We know which tokens are valid from the source code
        let is_valid = match dim {
            StandingDimension::Epistemic => {
                matches!(token.as_str(), "unresolved" | "working" | "supported" | "contested" | "superseded")
            }
            StandingDimension::Review => {
                matches!(token.as_str(), "unreviewed" | "pending" | "accepted" | "rejected")
            }
            StandingDimension::Process => {
                matches!(token.as_str(), "active" | "blocked" | "completed" | "archived")
            }
        };

        // There is no public way to call validate_standing_token_for_dimension directly, 
        // but we can test it via validate_standing_policy if we construct one.
        
        let policy = StandingPolicy {
            name: "test_policy".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            transition_rules: vec![StandingTransitionRule {
                dimension: dim.as_str().to_string(),
                from: vec![token.clone()],
                to: vec![token.clone()],
                requires_review: false,
            }],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };

        let result = validate_standing_policy(&policy);
        if is_valid {
            assert!(result.is_ok(), "Expected token '{}' to be valid for dimension '{}'", token, dim.as_str());
        } else {
            assert!(result.is_err(), "Expected token '{}' to be invalid for dimension '{}'", token, dim.as_str());
        }
    }

    #[test]
    fn test_validate_workflow_operation_ids(id in "[a-z][a-z0-9_]{0,63}") {
        let op = WorkflowOperation {
            id: id.clone(),
            kind: "nop".to_string(),
            input_contracts: vec![],
            output_contracts: vec![],
            instruction: None,
            compiled_context: None,
            policy: None,
            provider_profile: None,
        };
        let workflow = WorkflowDefinition {
            name: "test_workflow".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            operations: vec![op],
            edges: vec![],
            guards: vec![],
        };
        assert!(validate_workflow_definition(&workflow).is_ok());
    }
}
