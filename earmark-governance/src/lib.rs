use std::collections::BTreeMap;

use chrono::Utc;
use earmark_core::{
    HeaderValue, Kind, ObjectRef, ProcessStanding, Provenance, ReviewStanding, Standing,
    StandingDimension, StandingPolicy, Timestamp,
};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
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
    pub fn create_review_object<S: CanonicalStore>(
        store: &S,
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
        store.write_object(&stored)?;
        Ok(stored)
    }

    pub fn create_governance_event_object<S: CanonicalStore>(
        store: &S,
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
        store.write_object(&stored)?;
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

pub fn validate_standing_transition(
    policy: &StandingPolicy,
    current: &Standing,
    requested: &Standing,
) -> Result<(), GovernanceError> {
    for rule in &policy.transition_rules {
        match StandingDimension::parse(&rule.dimension) {
            Ok(StandingDimension::Review) => {
                let from = current.review.as_str();
                let to = requested.review.as_str();
                if rule.from.iter().any(|v| v == from) && rule.to.iter().any(|v| v == to) {
                    return Ok(());
                }
            }
            Ok(StandingDimension::Process) => {
                let from = current.process.as_str();
                let to = requested.process.as_str();
                if rule.from.iter().any(|v| v == from) && rule.to.iter().any(|v| v == to) {
                    return Ok(());
                }
            }
            Ok(StandingDimension::Epistemic) => {
                let from = current.epistemic.as_str();
                let to = requested.epistemic.as_str();
                if rule.from.iter().any(|v| v == from) && rule.to.iter().any(|v| v == to) {
                    return Ok(());
                }
            }
            Err(_) => {
                return Err(GovernanceError::IllegalTransition(format!(
                    "invalid standing dimension '{}'",
                    rule.dimension
                )))
            }
        }
    }

    Err(GovernanceError::IllegalTransition(
        "requested standing does not match any transition rule".to_string(),
    ))
}

pub fn export_allowed(policy: &StandingPolicy, standing: &Standing) -> Result<(), GovernanceError> {
    for requirement in &policy.operation_requirements {
        if requirement.operation == "export" {
            if let Some(review_required) = requirement.minimums.get("review") {
                let current = standing.review.as_str();
                if current != review_required {
                    return Err(GovernanceError::ExportBlocked(
                        "review standing below export minimum".to_string(),
                    ));
                }
            }
            if requirement
                .forbidden
                .get("process")
                .map(|forbidden| {
                    let current = standing.process.as_str();
                    forbidden.iter().any(|v| v == current)
                })
                .unwrap_or(false)
            {
                return Err(GovernanceError::ExportBlocked(
                    "process standing forbids export".to_string(),
                ));
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

        assert!(validate_standing_transition(&policy, &current, &requested).is_ok());

        let requested_rejected = Standing {
            review: ReviewStanding::Rejected,
            ..Standing::default()
        };
        assert!(validate_standing_transition(&policy, &current, &requested_rejected).is_err());
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
                minimums: BTreeMap::from([("review".to_string(), "accepted".to_string())]),
                forbidden: BTreeMap::new(),
            }],
            escalations: vec![],
            rationale: None,
        };

        let mut standing = Standing::default();
        assert!(export_allowed(&policy, &standing).is_err());

        standing.review = ReviewStanding::Accepted;
        assert!(export_allowed(&policy, &standing).is_ok());
    }
}
