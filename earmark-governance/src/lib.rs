use std::collections::BTreeMap;

use chrono::Utc;
use earmark_core::{
    HeaderValue, Kind, ObjectRef, ProcessStanding, Provenance, ReviewStanding, Standing,
    StandingDimension, StandingPolicy, Timestamp,
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
        next.review = if accepted {
            ReviewStanding::Accepted
        } else {
            ReviewStanding::Rejected
        };
        next
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingTransitionResult {
    pub requires_review: bool,
}

pub fn validate_standing_transition(
    policy: &StandingPolicy,
    current: &Standing,
    requested: &Standing,
) -> Result<StandingTransitionResult, GovernanceError> {
    let mut changed = Vec::new();
    if current.epistemic != requested.epistemic {
        changed.push(StandingDimension::Epistemic);
    }
    if current.review != requested.review {
        changed.push(StandingDimension::Review);
    }
    if current.process != requested.process {
        changed.push(StandingDimension::Process);
    }

    if changed.is_empty() {
        return Ok(StandingTransitionResult {
            requires_review: false,
        });
    }

    if changed.len() > 1 {
        return Err(GovernanceError::IllegalTransition(
            "standing requests must change exactly one dimension at a time".to_string(),
        ));
    }

    let dim = changed[0];
    let (from, to) = match dim {
        StandingDimension::Epistemic => (current.epistemic.as_str(), requested.epistemic.as_str()),
        StandingDimension::Review => (current.review.as_str(), requested.review.as_str()),
        StandingDimension::Process => (current.process.as_str(), requested.process.as_str()),
    };

    for rule in &policy.transition_rules {
        if let Ok(rule_dim) = StandingDimension::parse(&rule.dimension) {
            if rule_dim == dim {
                if rule.from.iter().any(|v| v == from) && rule.to.iter().any(|v| v == to) {
                    return Ok(StandingTransitionResult {
                        requires_review: rule.requires_review,
                    });
                }
            }
        }
    }

    Err(GovernanceError::IllegalTransition(format!(
        "no transition rule allows changing {} from '{}' to '{}'",
        dim.as_str(),
        from,
        to
    )))
}

pub fn export_allowed(policy: &StandingPolicy, standing: &Standing) -> Result<(), GovernanceError> {
    for requirement in &policy.operation_requirements {
        if requirement.operation == "export" {
            // Check minimums
            for (dim_str, required_value) in &requirement.minimums {
                let dim = StandingDimension::parse(dim_str)
                    .map_err(|e| GovernanceError::IllegalTransition(e.to_string()))?;
                let actual_value = match dim {
                    StandingDimension::Epistemic => standing.epistemic.as_str(),
                    StandingDimension::Review => standing.review.as_str(),
                    StandingDimension::Process => standing.process.as_str(),
                };
                if actual_value != required_value {
                    return Err(GovernanceError::ExportBlocked(format!(
                        "export blocked: {} dimension '{}' does not match required value '{}'",
                        dim_str, actual_value, required_value
                    )));
                }
            }

            // Check forbidden
            for (dim_str, forbidden_values) in &requirement.forbidden {
                let dim = StandingDimension::parse(dim_str)
                    .map_err(|e| GovernanceError::IllegalTransition(e.to_string()))?;
                let actual_value = match dim {
                    StandingDimension::Epistemic => standing.epistemic.as_str(),
                    StandingDimension::Review => standing.review.as_str(),
                    StandingDimension::Process => standing.process.as_str(),
                };
                if forbidden_values.iter().any(|v| v == actual_value) {
                    return Err(GovernanceError::ExportBlocked(format!(
                        "export blocked: {} dimension '{}' is forbidden",
                        dim_str, actual_value
                    )));
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

pub fn status_class_for_standing(standing: &Standing) -> &'static str {
    match (&standing.review, &standing.process) {
        (ReviewStanding::Rejected, _) => "attention_required",
        (_, ProcessStanding::Blocked) => "blocked",
        (ReviewStanding::Accepted, ProcessStanding::Completed) => "complete",
        _ => "active",
    }
}

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error("illegal standing transition: {0}")]
    IllegalTransition(String),
    #[error("export blocked: {0}")]
    ExportBlocked(String),
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("serde json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use earmark_core::{
        OperationRequirement, ProcessStanding, ReviewStanding, Standing, StandingPolicy,
        StandingTransitionRule,
    };

    #[test]
    fn test_apply_review_outcome() {
        let standing = Standing::default();
        let next = GovernanceService::apply_review_outcome(&standing, true);
        assert_eq!(next.review, ReviewStanding::Accepted);
        let next = GovernanceService::apply_review_outcome(&standing, false);
        assert_eq!(next.review, ReviewStanding::Rejected);
    }

    #[test]
    fn test_status_class_for_standing() {
        let mut standing = Standing::default();
        assert_eq!(status_class_for_standing(&standing), "active");

        standing.review = ReviewStanding::Rejected;
        assert_eq!(status_class_for_standing(&standing), "attention_required");

        standing.review = ReviewStanding::Accepted;
        standing.process = ProcessStanding::Completed;
        assert_eq!(status_class_for_standing(&standing), "complete");

        standing.process = ProcessStanding::Blocked;
        assert_eq!(status_class_for_standing(&standing), "blocked");
    }

    #[test]
    fn test_validate_standing_transition() {
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![StandingTransitionRule {
                dimension: "review".to_string(),
                from: vec!["unreviewed".to_string()],
                to: vec!["accepted".to_string()],
                requires_review: false,
            }],
            operation_requirements: vec![],
            escalations: vec![],
            rationale: None,
        };

        let current = Standing::default();
        let requested = Standing {
            review: ReviewStanding::Accepted,
            ..Standing::default()
        };

        let res = validate_standing_transition(&policy, &current, &requested).unwrap();
        assert!(!res.requires_review);

        let requested_rejected = Standing {
            review: ReviewStanding::Rejected,
            ..Standing::default()
        };
        assert!(validate_standing_transition(&policy, &current, &requested_rejected).is_err());
    }

    #[test]
    fn test_validate_standing_transition_multi_dim() {
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![
                StandingTransitionRule {
                    dimension: "review".to_string(),
                    from: vec!["unreviewed".to_string()],
                    to: vec!["accepted".to_string()],
                    requires_review: false,
                },
                StandingTransitionRule {
                    dimension: "process".to_string(),
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
        let requested = Standing {
            review: ReviewStanding::Accepted,
            process: ProcessStanding::Completed,
            ..Standing::default()
        };

        // Multi-dimension should fail
        assert!(validate_standing_transition(&policy, &current, &requested).is_err());
    }

    #[test]
    fn test_export_allowed() {
        let policy = StandingPolicy {
            name: "test".to_string(),
            version: "1".to_string(),
            description: None,
            transition_rules: vec![],
            operation_requirements: vec![OperationRequirement {
                operation: "export".to_string(),
                minimums: BTreeMap::from([
                    ("review".to_string(), "accepted".to_string()),
                    ("epistemic".to_string(), "supported".to_string()),
                ]),
                forbidden: BTreeMap::from([("process".to_string(), vec!["blocked".to_string()])]),
            }],
            escalations: vec![],
            rationale: None,
        };

        let mut standing = Standing::default();
        // Fails because review is unreviewed and epistemic is working
        assert!(export_allowed(&policy, &standing).is_err());

        standing.review = ReviewStanding::Accepted;
        // Still fails because epistemic is working
        assert!(export_allowed(&policy, &standing).is_err());

        standing.epistemic = earmark_core::EpistemicStanding::Supported;
        // Should pass now
        assert!(export_allowed(&policy, &standing).is_ok());

        standing.process = ProcessStanding::Blocked;
        // Fails because process is blocked (forbidden)
        assert!(export_allowed(&policy, &standing).is_err());
    }
}
