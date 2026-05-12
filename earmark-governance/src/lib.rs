use std::collections::BTreeMap;

use chrono::Utc;
use earmark_core::projection::{
    project, ImmutabilityProjection, ProjectionError, ReviewProjection,
};
use earmark_core::{
    DimensionId, HeaderValue, Kind, ObjectRef, Provenance, ScalarValue, Standing, StandingPolicy,
    StandingRegistry, Timestamp, TokenId,
};
use earmark_store::{StoredObject, StoredPayload};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceEvent {
    pub class: String,
    pub severity: String,
    pub message: String,
    pub object: Option<ObjectRef>,
    pub occurred_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewPayload {
    pub target: ObjectRef,
    pub status: String,
    pub rationale: Option<String>,
    pub reviewed_at: Timestamp,
}

pub struct GovernanceService;

impl GovernanceService {
    pub fn create_review_object(
        target: ObjectRef,
        accepted: bool,
        rationale: Option<String>,
    ) -> Result<StoredObject, GovernanceError> {
        let status = if accepted { "accepted" } else { "rejected" }.to_string();
        let payload = ReviewPayload {
            target: target.clone(),
            status: status.clone(),
            rationale,
            reviewed_at: Utc::now(),
        };
        let stored = StoredObject::new(
            Kind::Review,
            Some("review".to_string()),
            Standing::default(),
            Provenance::direct_input("operator"),
            BTreeMap::from([
                (
                    "title".to_string(),
                    HeaderValue::String(format!("Review for {}", target.id.as_str())),
                ),
                ("review_status".to_string(), HeaderValue::String(status)),
            ]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&payload)?),
            vec![],
        );
        Ok(stored)
    }

    pub fn create_governance_event_object(
        event: GovernanceEvent,
    ) -> Result<StoredObject, GovernanceError> {
        let stored = StoredObject::new(
            Kind::Event,
            Some("governance_event".to_string()),
            Standing::default(),
            Provenance::direct_input("governance"),
            BTreeMap::from([
                (
                    "title".to_string(),
                    HeaderValue::String(format!("Governance event: {}", event.class)),
                ),
                (
                    "severity".to_string(),
                    HeaderValue::String(event.severity.clone()),
                ),
            ]),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&event)?),
            vec![],
        );
        Ok(stored)
    }

    pub fn apply_review_outcome(current: &Standing, accepted: bool) -> Standing {
        let mut next = current.clone();
        next.values.insert(
            DimensionId::new("kernel:review"),
            if accepted {
                TokenId::new("accepted")
            } else {
                TokenId::new("rejected")
            },
        );
        next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingTransitionResult {
    pub requires_review: bool,
}

pub fn validate_standing_transition(
    policy: &StandingPolicy,
    registry: &StandingRegistry,
    current: &Standing,
    requested: &Standing,
) -> Result<StandingTransitionResult, GovernanceError> {
    let all_keys: std::collections::BTreeSet<&DimensionId> = current
        .values
        .keys()
        .chain(requested.values.keys())
        .collect();

    let mut changed_dim: Option<&DimensionId> = None;

    for &dim_id in &all_keys {
        let cur_val = current.get(dim_id);
        let req_val = requested.get(dim_id);
        if cur_val != req_val {
            if changed_dim.is_some() {
                return Err(GovernanceError::IllegalTransition(
                    "standing requests must change exactly one dimension at a time".to_string(),
                ));
            }

            if !registry.dimensions.contains_key(dim_id) {
                return Err(GovernanceError::IllegalTransition(format!(
                    "dimension '{}' not found in standing registry",
                    dim_id.as_str()
                )));
            }

            let dim_def = registry.dimensions.get(dim_id).ok_or_else(|| {
                GovernanceError::IllegalTransition(format!(
                    "dimension '{}' not found",
                    dim_id.as_str()
                ))
            })?;

            if let Some(from_tok) = cur_val {
                if !dim_def.tokens.iter().any(|t| t.id == *from_tok) {
                    return Err(GovernanceError::IllegalTransition(format!(
                        "current token '{}' is not valid for dimension '{}'",
                        from_tok.as_str(),
                        dim_id.as_str()
                    )));
                }
            }
            if let Some(to_tok) = req_val {
                if !dim_def.tokens.iter().any(|t| t.id == *to_tok) {
                    return Err(GovernanceError::IllegalTransition(format!(
                        "requested token '{}' is not valid for dimension '{}'",
                        to_tok.as_str(),
                        dim_id.as_str()
                    )));
                }
            }

            changed_dim = Some(dim_id);
        }
    }

    let changed_dim = match changed_dim {
        Some(d) => d,
        None => {
            return Ok(StandingTransitionResult {
                requires_review: false,
            })
        }
    };

    let from = current
        .get(changed_dim)
        .map(TokenId::as_str)
        .unwrap_or("unknown");
    let to = requested
        .get(changed_dim)
        .map(TokenId::as_str)
        .unwrap_or("unknown");

    let projection = project(requested, registry)?;
    if !projection.conflicts.is_empty() {
        return Err(GovernanceError::IllegalTransition(format!(
            "transition would produce protocol conflicts: {}",
            projection
                .conflicts
                .iter()
                .map(|c| c.message.clone())
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }

    for rule in &policy.transition_rules {
        if rule.dimension == changed_dim.as_str()
            && rule.from.iter().any(|v| v == from)
            && rule.to.iter().any(|v| v == to)
        {
            return Ok(StandingTransitionResult {
                requires_review: rule.requires_review,
            });
        }
    }

    Err(GovernanceError::IllegalTransition(format!(
        "no transition rule allows changing {} from '{}' to '{}'",
        changed_dim.as_str(),
        from,
        to
    )))
}

pub fn check_immutability(
    registry: &StandingRegistry,
    standing: &Standing,
) -> Result<(), GovernanceError> {
    let projection = project(standing, registry)?;
    if projection.immutability == ImmutabilityProjection::Sealed {
        return Err(GovernanceError::ImmutabilityViolation(
            "object is sealed and cannot be mutated".to_string(),
        ));
    }
    Ok(())
}

pub fn is_trusted_actor(actor: &str) -> bool {
    matches!(actor, "runtime" | "execution_engine" | "system")
}

pub fn export_allowed(
    policy: &StandingPolicy,
    registry: &StandingRegistry,
    standing: &Standing,
) -> Result<(), GovernanceError> {
    let projection = project(standing, registry)?;

    for requirement in &policy.operation_requirements {
        if requirement.operation == "export" {
            for (dim_str, required_value) in &requirement.required_standing {
                let dim_id = DimensionId::parse(dim_str).map_err(|e| {
                    GovernanceError::ExportBlocked(format!(
                        "invalid dimension '{}': {}",
                        dim_str, e
                    ))
                })?;
                let actual = standing
                    .get(&dim_id)
                    .map(TokenId::as_str)
                    .unwrap_or("unknown");
                if actual != required_value {
                    return Err(GovernanceError::ExportBlocked(format!(
                        "export blocked: {} dimension '{}' does not match required value '{}'",
                        dim_str, actual, required_value
                    )));
                }
            }

            for (dim_str, forbidden_values) in &requirement.forbidden_standing {
                let dim_id = DimensionId::parse(dim_str).map_err(|e| {
                    GovernanceError::ExportBlocked(format!(
                        "invalid dimension '{}': {}",
                        dim_str, e
                    ))
                })?;
                let actual = standing
                    .get(&dim_id)
                    .map(TokenId::as_str)
                    .unwrap_or("unknown");
                if forbidden_values.iter().any(|v| v == actual) {
                    return Err(GovernanceError::ExportBlocked(format!(
                        "export blocked: {} dimension '{}' is forbidden",
                        dim_str, actual
                    )));
                }
            }

            for (protocol_id, props) in &requirement.required_protocols {
                match protocol_id.as_str() {
                    "kernel:review" => {
                        let state = props.get("state").and_then(|v| match v {
                            ScalarValue::String(s) => Some(s.as_str()),
                            _ => None,
                        });
                        let projected = projection.review.as_ref().map(|r| match r {
                            ReviewProjection::Unreviewed => "unreviewed",
                            ReviewProjection::Pending => "pending",
                            ReviewProjection::Accepted => "accepted",
                            ReviewProjection::Rejected => "rejected",
                        });
                        if let Some(expected) = state {
                            if projected != Some(expected) {
                                return Err(GovernanceError::ExportBlocked(format!(
                                    "export blocked: required protocol {} state '{}', projected '{:?}'",
                                    protocol_id, expected, projected
                                )));
                            }
                        }
                    }
                    "kernel:visibility" => {
                        if let Some(ScalarValue::Bool(true)) = props.get("expose_to_provider") {
                            if !projection.visibility.expose_to_provider {
                                return Err(GovernanceError::ExportBlocked(format!(
                                    "export blocked: protocol {} requires expose_to_provider=true",
                                    protocol_id
                                )));
                            }
                        }
                    }
                    _ => {}
                }
            }

            for (protocol_id, props) in &requirement.forbidden_protocols {
                if protocol_id.as_str() == "kernel:immutability" {
                    let state = props.get("state").and_then(|v| match v {
                        ScalarValue::String(s) => Some(s.as_str()),
                        _ => None,
                    });
                    if state == Some("sealed")
                        && projection.immutability == ImmutabilityProjection::Sealed
                    {
                        return Err(GovernanceError::ExportBlocked(format!(
                            "export blocked: protocol {} is sealed and export is forbidden",
                            protocol_id
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn escalation_for_trigger(
    policy: &StandingPolicy,
    trigger: &str,
    object: Option<ObjectRef>,
) -> Option<GovernanceEvent> {
    policy
        .escalations
        .iter()
        .find(|rule| rule.trigger == trigger)
        .map(|rule| GovernanceEvent {
            class: trigger.to_string(),
            severity: rule.severity.clone(),
            message: rule.message.clone(),
            object,
            occurred_at: Utc::now(),
        })
}

pub fn status_class_for_standing(
    standing: &Standing,
    registry: &StandingRegistry,
) -> Result<&'static str, GovernanceError> {
    let projection = project(standing, registry)?;

    if !projection.conflicts.is_empty() {
        return Err(GovernanceError::AmbiguousStatus(
            "ambiguous status: protocol projection produced conflicts".to_string(),
        ));
    }

    match (projection.review, projection.process) {
        (Some(ReviewProjection::Rejected), _) => Ok("attention_required"),
        (_, Some(earmark_core::projection::ProcessProjection::Blocked)) => Ok("blocked"),
        (
            Some(ReviewProjection::Accepted),
            Some(earmark_core::projection::ProcessProjection::Completed),
        ) => Ok("complete"),
        _ => Ok("active"),
    }
}

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("illegal standing transition: {0}")]
    IllegalTransition(String),
    #[error("export blocked: {0}")]
    ExportBlocked(String),
    #[error("immutability violation: {0}")]
    ImmutabilityViolation(String),
    #[error("review required: {0}")]
    ReviewRequired(String),
    #[error("ambiguous status: {0}")]
    AmbiguousStatus(String),
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("serde json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<ProjectionError> for GovernanceError {
    fn from(e: ProjectionError) -> Self {
        GovernanceError::IllegalTransition(format!("projection error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use earmark_core::{
        DimensionId, ObjectId, OperationRequirement, Standing, StandingPolicy,
        StandingTransitionRule, TokenId,
    };

    fn kernel_registry() -> StandingRegistry {
        StandingRegistry::kernel_defaults()
    }

    fn make_standing(review: &str, process: &str, epistemic: &str) -> Standing {
        let mut s = Standing::default();
        s.values
            .insert(DimensionId::new("kernel:review"), TokenId::new(review));
        s.values
            .insert(DimensionId::new("kernel:process"), TokenId::new(process));
        s.values.insert(
            DimensionId::new("kernel:epistemic"),
            TokenId::new(epistemic),
        );
        s
    }

    #[test]
    fn test_apply_review_outcome() {
        let standing = Standing::default();
        let next = GovernanceService::apply_review_outcome(&standing, true);
        assert_eq!(
            next.get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("accepted")
        );
        let next = GovernanceService::apply_review_outcome(&standing, false);
        assert_eq!(
            next.get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("rejected")
        );
    }

    #[test]
    fn test_status_class_for_standing() {
        let registry = kernel_registry();
        let mut standing = Standing::default();
        assert_eq!(
            status_class_for_standing(&standing, &registry).unwrap(),
            "active"
        );

        standing
            .values
            .insert(DimensionId::new("kernel:review"), TokenId::new("rejected"));
        assert_eq!(
            status_class_for_standing(&standing, &registry).unwrap(),
            "attention_required"
        );

        standing
            .values
            .insert(DimensionId::new("kernel:review"), TokenId::new("accepted"));
        standing.values.insert(
            DimensionId::new("kernel:process"),
            TokenId::new("completed"),
        );
        assert_eq!(
            status_class_for_standing(&standing, &registry).unwrap(),
            "complete"
        );

        standing
            .values
            .insert(DimensionId::new("kernel:process"), TokenId::new("blocked"));
        assert_eq!(
            status_class_for_standing(&standing, &registry).unwrap(),
            "blocked"
        );
    }

    #[test]
    fn test_validate_standing_transition() {
        let registry = kernel_registry();
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![StandingTransitionRule {
                dimension: "kernel:review".to_string(),
                from: vec!["unreviewed".to_string()],
                to: vec!["accepted".to_string()],
                requires_review: false,
            }],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };

        let current = Standing::default();
        let requested = make_standing("accepted", "active", "working");

        let res = validate_standing_transition(&policy, &registry, &current, &requested).unwrap();
        assert!(!res.requires_review);

        let requested_rejected = make_standing("rejected", "active", "working");
        assert!(
            validate_standing_transition(&policy, &registry, &current, &requested_rejected)
                .is_err()
        );
    }

    #[test]
    fn test_validate_standing_transition_multi_dim() {
        let registry = kernel_registry();
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![
                StandingTransitionRule {
                    dimension: "kernel:review".to_string(),
                    from: vec!["unreviewed".to_string()],
                    to: vec!["accepted".to_string()],
                    requires_review: false,
                },
                StandingTransitionRule {
                    dimension: "kernel:process".to_string(),
                    from: vec!["active".to_string()],
                    to: vec!["completed".to_string()],
                    requires_review: false,
                },
            ],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };

        let current = Standing::default();
        let requested = make_standing("accepted", "completed", "working");

        assert!(validate_standing_transition(&policy, &registry, &current, &requested).is_err());
    }

    #[test]
    fn test_export_allowed() {
        let registry = kernel_registry();
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![],
            operation_requirements: vec![OperationRequirement {
                operation: "export".to_string(),
                required_standing: BTreeMap::from([
                    ("kernel:review".to_string(), "accepted".to_string()),
                    ("kernel:epistemic".to_string(), "supported".to_string()),
                ]),
                forbidden_standing: BTreeMap::from([(
                    "kernel:process".to_string(),
                    vec!["blocked".to_string()],
                )]),
                ..Default::default()
            }],
            escalations: vec![],
            rationale: None,
        };

        let standing = make_standing("unreviewed", "active", "working");
        assert!(export_allowed(&policy, &registry, &standing).is_err());

        let standing = make_standing("accepted", "active", "working");
        assert!(export_allowed(&policy, &registry, &standing).is_err());

        let standing = make_standing("accepted", "active", "supported");
        assert!(export_allowed(&policy, &registry, &standing).is_ok());

        let standing = make_standing("accepted", "blocked", "supported");
        assert!(export_allowed(&policy, &registry, &standing).is_err());
    }

    #[test]
    fn test_generic_standing_transition() {
        use earmark_core::{
            KernelProtocolId, ProtocolBinding, StandingDimensionDefinition, StandingRegistry,
            StandingTokenDefinition, SystemDefinition,
        };
        let sys = SystemDefinition {
            system_id: "test".to_string(),
            namespace: "test".to_string(),
            title: "Test".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("research:status"),
                default: TokenId::from_static("draft"),
                tokens: vec![
                    StandingTokenDefinition {
                        id: TokenId::from_static("draft"),
                        implements: vec![],
                    },
                    StandingTokenDefinition {
                        id: TokenId::from_static("verified"),
                        implements: vec![ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    },
                ],
            }],
            runtime_profile: earmark_core::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("registry");

        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![StandingTransitionRule {
                dimension: "research:status".to_string(),
                from: vec!["draft".to_string()],
                to: vec!["verified".to_string()],
                requires_review: false,
            }],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };

        let mut current = Standing {
            values: BTreeMap::new(),
        };
        current.values.insert(
            DimensionId::from_static("research:status"),
            TokenId::from_static("draft"),
        );

        let mut requested = Standing {
            values: BTreeMap::new(),
        };
        requested.values.insert(
            DimensionId::from_static("research:status"),
            TokenId::from_static("verified"),
        );

        let res = validate_standing_transition(&policy, &registry, &current, &requested).unwrap();
        assert!(!res.requires_review);
    }

    #[test]
    fn test_transition_into_accepted_fails_without_review() {
        let registry = kernel_registry();
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![StandingTransitionRule {
                dimension: "kernel:review".to_string(),
                from: vec!["unreviewed".to_string()],
                to: vec!["accepted".to_string()],
                requires_review: true,
            }],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };

        let current = Standing::default();
        let requested = make_standing("accepted", "active", "working");

        let res = validate_standing_transition(&policy, &registry, &current, &requested).unwrap();
        assert!(res.requires_review);
    }

    #[test]
    fn test_immutability_check() {
        use earmark_core::{
            KernelProtocolId, ProtocolBinding, StandingDimensionDefinition,
            StandingTokenDefinition, SystemDefinition,
        };
        let sys = SystemDefinition {
            system_id: "test_immut".to_string(),
            namespace: "test/immut".to_string(),
            title: "Immut".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("dim:immut"),
                default: TokenId::from_static("mutable_val"),
                tokens: vec![
                    StandingTokenDefinition {
                        id: TokenId::from_static("mutable_val"),
                        implements: vec![],
                    },
                    StandingTokenDefinition {
                        id: TokenId::from_static("sealed_val"),
                        implements: vec![ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:immutability"),
                            state: Some("sealed".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    },
                ],
            }],
            runtime_profile: earmark_core::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("immut registry");

        let mut mutable = Standing::default();
        mutable.values.insert(
            DimensionId::from_static("dim:immut"),
            TokenId::from_static("mutable_val"),
        );
        assert!(check_immutability(&registry, &mutable).is_ok());

        let mut sealed = Standing::default();
        sealed.values.insert(
            DimensionId::from_static("dim:immut"),
            TokenId::from_static("sealed_val"),
        );
        assert!(check_immutability(&registry, &sealed).is_err());
    }

    #[test]
    fn test_export_allowed_with_protocols() {
        let registry = kernel_registry();
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![],
            operation_requirements: vec![OperationRequirement {
                operation: "export".to_string(),
                required_standing: BTreeMap::new(),
                forbidden_standing: BTreeMap::new(),
                required_protocols: BTreeMap::from([(
                    "kernel:review".to_string(),
                    BTreeMap::from([(
                        "state".to_string(),
                        ScalarValue::String("accepted".to_string()),
                    )]),
                )]),
                forbidden_protocols: BTreeMap::from([(
                    "kernel:immutability".to_string(),
                    BTreeMap::from([(
                        "state".to_string(),
                        ScalarValue::String("sealed".to_string()),
                    )]),
                )]),
            }],
            escalations: vec![],
            rationale: None,
        };

        let accepted = make_standing("accepted", "active", "working");
        assert!(export_allowed(&policy, &registry, &accepted).is_ok());

        let unreviewed = make_standing("unreviewed", "active", "working");
        assert!(export_allowed(&policy, &registry, &unreviewed).is_err());
    }

    #[test]
    fn test_projection_ambiguity_causes_status_error() {
        use earmark_core::{
            KernelProtocolId, ProtocolBinding, StandingDimensionDefinition,
            StandingTokenDefinition, SystemDefinition,
        };
        let sys = SystemDefinition {
            system_id: "sys_ambig".to_string(),
            namespace: "systems/ambig".to_string(),
            title: "Ambig".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![
                StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:rev_a"),
                    default: TokenId::from_static("accepted_tok"),
                    tokens: vec![StandingTokenDefinition {
                        id: TokenId::from_static("accepted_tok"),
                        implements: vec![ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
                StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:rev_b"),
                    default: TokenId::from_static("rejected_tok"),
                    tokens: vec![StandingTokenDefinition {
                        id: TokenId::from_static("rejected_tok"),
                        implements: vec![ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("rejected".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
            ],
            runtime_profile: earmark_core::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("ambig registry");

        let mut standing = Standing::default();
        standing.values.insert(
            DimensionId::from_static("dim:rev_a"),
            TokenId::from_static("accepted_tok"),
        );
        standing.values.insert(
            DimensionId::from_static("dim:rev_b"),
            TokenId::from_static("rejected_tok"),
        );

        let result = status_class_for_standing(&standing, &registry);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GovernanceError::AmbiguousStatus(_)
        ));
    }

    #[test]
    fn test_review_artifact_does_not_mutate_standing() {
        let registry = kernel_registry();
        let standing = Standing::default();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("unreviewed")
        );

        let review = GovernanceService::create_review_object(
            earmark_core::ObjectRef {
                id: ObjectId::new(),
                version_id: earmark_core::VersionId::new(),
                kind: Kind::Object,
                class: Some("artifact".to_string()),
            },
            true,
            None,
        )
        .unwrap();
        assert_eq!(review.envelope.kind, Kind::Review);

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.review, Some(ReviewProjection::Unreviewed));
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("unreviewed")
        );
    }

    #[test]
    fn test_export_requirement_checks_raw_dimension() {
        use earmark_core::{
            StandingDimensionDefinition, StandingTokenDefinition, SystemDefinition,
        };
        let sys = SystemDefinition {
            system_id: "test_export_raw".to_string(),
            namespace: "test/export_raw".to_string(),
            title: "ExportRaw".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![StandingDimensionDefinition {
                id: DimensionId::from_static("research:status"),
                default: TokenId::from_static("draft"),
                tokens: vec![
                    StandingTokenDefinition {
                        id: TokenId::from_static("draft"),
                        implements: vec![],
                    },
                    StandingTokenDefinition {
                        id: TokenId::from_static("verified"),
                        implements: vec![],
                    },
                ],
            }],
            runtime_profile: earmark_core::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("export_raw registry");

        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![],
            operation_requirements: vec![OperationRequirement {
                operation: "export".to_string(),
                required_standing: BTreeMap::from([(
                    "research:status".to_string(),
                    "verified".to_string(),
                )]),
                forbidden_standing: BTreeMap::new(),
                ..Default::default()
            }],
            escalations: vec![],
            rationale: None,
        };

        let mut draft_standing = Standing {
            values: BTreeMap::new(),
        };
        draft_standing.values.insert(
            DimensionId::from_static("research:status"),
            TokenId::from_static("draft"),
        );
        assert!(export_allowed(&policy, &registry, &draft_standing).is_err());

        let mut verified_standing = Standing {
            values: BTreeMap::new(),
        };
        verified_standing.values.insert(
            DimensionId::from_static("research:status"),
            TokenId::from_static("verified"),
        );
        assert!(export_allowed(&policy, &registry, &verified_standing).is_ok());
    }
}
