use earmark_core::{ClassDefinition, Kind, ObjectId, RelationRule, VersionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationEndpointFacts {
    pub id: ObjectId,
    pub version_id: VersionId,
    pub kind: Kind,
    pub class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationAuthorizationDecision {
    Allowed(RelationAuthorizationReason),
    Blocked(RelationAuthorizationFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationAuthorizationReason {
    SourceOutgoingRule {
        class: String,
        relation_type: String,
    },
    TargetIncomingRule {
        class: String,
        relation_type: String,
    },
    SourceBidirectionalRule {
        class: String,
        relation_type: String,
    },
    TargetBidirectionalRule {
        class: String,
        relation_type: String,
    },
    EitherEndpointSourceRule {
        class: String,
        relation_type: String,
    },
    EitherEndpointTargetRule {
        class: String,
        relation_type: String,
    },
    PrivilegedSystemRelation {
        relation_type: String,
    },
}

impl std::fmt::Display for RelationAuthorizationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceOutgoingRule {
                class,
                relation_type,
            } => write!(
                f,
                "authorized by source class '{}' outgoing rule for '{}'",
                class, relation_type
            ),
            Self::TargetIncomingRule {
                class,
                relation_type,
            } => write!(
                f,
                "authorized by target class '{}' incoming rule for '{}'",
                class, relation_type
            ),
            Self::SourceBidirectionalRule {
                class,
                relation_type,
            } => write!(
                f,
                "authorized by source class '{}' bidirectional rule for '{}'",
                class, relation_type
            ),
            Self::TargetBidirectionalRule {
                class,
                relation_type,
            } => write!(
                f,
                "authorized by target class '{}' bidirectional rule for '{}'",
                class, relation_type
            ),
            Self::EitherEndpointSourceRule {
                class,
                relation_type,
            } => write!(
                f,
                "authorized by source class '{}' either_endpoint rule for '{}'",
                class, relation_type
            ),
            Self::EitherEndpointTargetRule {
                class,
                relation_type,
            } => write!(
                f,
                "authorized by target class '{}' either_endpoint rule for '{}'",
                class, relation_type
            ),
            Self::PrivilegedSystemRelation { relation_type } => write!(
                f,
                "authorized as privileged system relation '{}'",
                relation_type
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationAuthorizationFailure {
    NoMatchingRule {
        relation_type: String,
        source_class: Option<String>,
        target_class: Option<String>,
    },
    MalformedRule {
        class: String,
        relation_type: String,
        error: String,
    },
    PrivilegeMismatch {
        relation_type: String,
        mode: String,
    },
    UntrustedPrivilegedProvenance {
        relation_type: String,
    },
    CounterpartyMismatch {
        rule_class: String,
        expected_counterparty_classes: Vec<String>,
        actual_counterparty_class: Option<String>,
    },
}

impl std::fmt::Display for RelationAuthorizationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMatchingRule {
                relation_type,
                source_class,
                target_class,
            } => write!(
                f,
                "no matching rule found for relation '{}' between source '{:?}' and target '{:?}'",
                relation_type, source_class, target_class
            ),
            Self::MalformedRule {
                class,
                relation_type,
                error,
            } => write!(
                f,
                "malformed matching rule in class '{}' for relation '{}': {}",
                class, relation_type, error
            ),
            Self::PrivilegeMismatch {
                relation_type,
                mode,
            } => write!(
                f,
                "privileged relation '{}' cannot be created with mode '{}'",
                relation_type, mode
            ),
            Self::UntrustedPrivilegedProvenance { relation_type } => write!(
                f,
                "privileged relation '{}' has untrusted provenance",
                relation_type
            ),
            Self::CounterpartyMismatch {
                rule_class,
                expected_counterparty_classes,
                actual_counterparty_class,
            } => {
                write!(
                    f,
                    "counterparty mismatch in class '{}' rule: expected one of {:?}, got {:?}",
                    rule_class, expected_counterparty_classes, actual_counterparty_class
                )
            }
        }
    }
}

pub struct RelationAuthorizationResolver<'a> {
    pub relation_type: &'a str,
    pub source: &'a RelationEndpointFacts,
    pub target: &'a RelationEndpointFacts,
    pub source_definition: Option<&'a ClassDefinition>,
    pub target_definition: Option<&'a ClassDefinition>,
    pub creation_mode: Option<&'a str>,
    pub is_trusted_provenance: bool,
}

impl<'a> RelationAuthorizationResolver<'a> {
    pub fn resolve(&self) -> RelationAuthorizationDecision {
        // 1. Privileged relation check
        let is_privileged_type = earmark_core::is_privileged_relation(self.relation_type);
        match (
            is_privileged_type,
            self.creation_mode,
            self.is_trusted_provenance,
        ) {
            (true, Some("privileged_system"), true) => {
                return RelationAuthorizationDecision::Allowed(
                    RelationAuthorizationReason::PrivilegedSystemRelation {
                        relation_type: self.relation_type.to_string(),
                    },
                );
            }
            (true, Some("privileged_system"), false) => {
                return RelationAuthorizationDecision::Blocked(
                    RelationAuthorizationFailure::UntrustedPrivilegedProvenance {
                        relation_type: self.relation_type.to_string(),
                    },
                );
            }
            (true, _, _) => {
                return RelationAuthorizationDecision::Blocked(
                    RelationAuthorizationFailure::PrivilegeMismatch {
                        relation_type: self.relation_type.to_string(),
                        mode: self.creation_mode.unwrap_or("declared").to_string(),
                    },
                );
            }
            (false, Some("privileged_system"), _) => {
                return RelationAuthorizationDecision::Blocked(
                    RelationAuthorizationFailure::PrivilegeMismatch {
                        relation_type: self.relation_type.to_string(),
                        mode: "privileged_system".to_string(),
                    },
                );
            }
            _ => {} // Continue to normal rules
        }

        // 2. Source-side rules
        if let Some(def) = self.source_definition {
            for rule in &def.relation_rules {
                if rule.relation_type == self.relation_type {
                    match self.evaluate_rule(rule, true) {
                        RelationAuthorizationDecision::Allowed(reason) => {
                            return RelationAuthorizationDecision::Allowed(reason)
                        }
                        RelationAuthorizationDecision::Blocked(
                            RelationAuthorizationFailure::NoMatchingRule { .. },
                        )
                        | RelationAuthorizationDecision::Blocked(
                            RelationAuthorizationFailure::CounterpartyMismatch { .. },
                        ) => continue,
                        other_blocked => return other_blocked, // Fail fast for malformed rules
                    }
                }
            }
        }

        // 3. Target-side rules
        if let Some(def) = self.target_definition {
            for rule in &def.relation_rules {
                if rule.relation_type == self.relation_type {
                    match self.evaluate_rule(rule, false) {
                        RelationAuthorizationDecision::Allowed(reason) => {
                            return RelationAuthorizationDecision::Allowed(reason)
                        }
                        RelationAuthorizationDecision::Blocked(
                            RelationAuthorizationFailure::NoMatchingRule { .. },
                        )
                        | RelationAuthorizationDecision::Blocked(
                            RelationAuthorizationFailure::CounterpartyMismatch { .. },
                        ) => continue,
                        other_blocked => return other_blocked, // Fail fast for malformed rules
                    }
                }
            }
        }

        RelationAuthorizationDecision::Blocked(RelationAuthorizationFailure::NoMatchingRule {
            relation_type: self.relation_type.to_string(),
            source_class: self.source.class.clone(),
            target_class: self.target.class.clone(),
        })
    }

    fn evaluate_rule(
        &self,
        rule: &RelationRule,
        is_source_rule: bool,
    ) -> RelationAuthorizationDecision {
        let direction = rule.direction.as_deref().unwrap_or("outgoing");
        let authorizing_endpoint = rule.authorizing_endpoint.as_deref().unwrap_or("source");

        // Validate rule fields first (Step 9)
        if !matches!(direction, "outgoing" | "incoming" | "bidirectional") {
            return RelationAuthorizationDecision::Blocked(
                RelationAuthorizationFailure::MalformedRule {
                    class: if is_source_rule {
                        self.source.class.clone().unwrap_or_default()
                    } else {
                        self.target.class.clone().unwrap_or_default()
                    },
                    relation_type: self.relation_type.to_string(),
                    error: format!("invalid direction: {}", direction),
                },
            );
        }
        if !matches!(
            authorizing_endpoint,
            "source" | "target" | "either_endpoint"
        ) {
            return RelationAuthorizationDecision::Blocked(
                RelationAuthorizationFailure::MalformedRule {
                    class: if is_source_rule {
                        self.source.class.clone().unwrap_or_default()
                    } else {
                        self.target.class.clone().unwrap_or_default()
                    },
                    relation_type: self.relation_type.to_string(),
                    error: format!("invalid authorizing_endpoint: {}", authorizing_endpoint),
                },
            );
        }

        if is_source_rule {
            // Source-side rule check
            if !matches!(direction, "outgoing" | "bidirectional") {
                return RelationAuthorizationDecision::Blocked(
                    RelationAuthorizationFailure::NoMatchingRule {
                        relation_type: self.relation_type.to_string(),
                        source_class: self.source.class.clone(),
                        target_class: self.target.class.clone(),
                    },
                );
            }

            if !matches!(authorizing_endpoint, "source" | "either_endpoint") {
                return RelationAuthorizationDecision::Blocked(
                    RelationAuthorizationFailure::NoMatchingRule {
                        relation_type: self.relation_type.to_string(),
                        source_class: self.source.class.clone(),
                        target_class: self.target.class.clone(),
                    },
                );
            }

            // Check counterparty
            if !rule.counterparty_classes.is_empty() {
                if let Some(target_class) = &self.target.class {
                    if !rule.counterparty_classes.contains(target_class) {
                        return RelationAuthorizationDecision::Blocked(
                            RelationAuthorizationFailure::CounterpartyMismatch {
                                rule_class: self.source.class.clone().unwrap_or_default(),
                                expected_counterparty_classes: rule.counterparty_classes.clone(),
                                actual_counterparty_class: Some(target_class.clone()),
                            },
                        );
                    }
                } else {
                    return RelationAuthorizationDecision::Blocked(
                        RelationAuthorizationFailure::CounterpartyMismatch {
                            rule_class: self.source.class.clone().unwrap_or_default(),
                            expected_counterparty_classes: rule.counterparty_classes.clone(),
                            actual_counterparty_class: None,
                        },
                    );
                }
            }

            let reason = if authorizing_endpoint == "either_endpoint" {
                RelationAuthorizationReason::EitherEndpointSourceRule {
                    class: self.source.class.clone().unwrap_or_default(),
                    relation_type: self.relation_type.to_string(),
                }
            } else if direction == "bidirectional" {
                RelationAuthorizationReason::SourceBidirectionalRule {
                    class: self.source.class.clone().unwrap_or_default(),
                    relation_type: self.relation_type.to_string(),
                }
            } else {
                RelationAuthorizationReason::SourceOutgoingRule {
                    class: self.source.class.clone().unwrap_or_default(),
                    relation_type: self.relation_type.to_string(),
                }
            };
            RelationAuthorizationDecision::Allowed(reason)
        } else {
            // Target-side rule check
            if !matches!(direction, "incoming" | "bidirectional") {
                return RelationAuthorizationDecision::Blocked(
                    RelationAuthorizationFailure::NoMatchingRule {
                        relation_type: self.relation_type.to_string(),
                        source_class: self.source.class.clone(),
                        target_class: self.target.class.clone(),
                    },
                );
            }

            if !matches!(authorizing_endpoint, "target" | "either_endpoint") {
                return RelationAuthorizationDecision::Blocked(
                    RelationAuthorizationFailure::NoMatchingRule {
                        relation_type: self.relation_type.to_string(),
                        source_class: self.source.class.clone(),
                        target_class: self.target.class.clone(),
                    },
                );
            }

            // Check counterparty
            if !rule.counterparty_classes.is_empty() {
                if let Some(source_class) = &self.source.class {
                    if !rule.counterparty_classes.contains(source_class) {
                        return RelationAuthorizationDecision::Blocked(
                            RelationAuthorizationFailure::CounterpartyMismatch {
                                rule_class: self.target.class.clone().unwrap_or_default(),
                                expected_counterparty_classes: rule.counterparty_classes.clone(),
                                actual_counterparty_class: Some(source_class.clone()),
                            },
                        );
                    }
                } else {
                    return RelationAuthorizationDecision::Blocked(
                        RelationAuthorizationFailure::CounterpartyMismatch {
                            rule_class: self.target.class.clone().unwrap_or_default(),
                            expected_counterparty_classes: rule.counterparty_classes.clone(),
                            actual_counterparty_class: None,
                        },
                    );
                }
            }

            let reason = if authorizing_endpoint == "either_endpoint" {
                RelationAuthorizationReason::EitherEndpointTargetRule {
                    class: self.target.class.clone().unwrap_or_default(),
                    relation_type: self.relation_type.to_string(),
                }
            } else if direction == "bidirectional" {
                RelationAuthorizationReason::TargetBidirectionalRule {
                    class: self.target.class.clone().unwrap_or_default(),
                    relation_type: self.relation_type.to_string(),
                }
            } else {
                RelationAuthorizationReason::TargetIncomingRule {
                    class: self.target.class.clone().unwrap_or_default(),
                    relation_type: self.relation_type.to_string(),
                }
            };
            RelationAuthorizationDecision::Allowed(reason)
        }
    }
}
