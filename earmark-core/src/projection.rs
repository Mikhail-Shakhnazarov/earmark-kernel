use std::collections::BTreeMap;

use crate::{
    CoreError, DimensionId, KernelProtocolId, ScalarValue, Standing, StandingRegistry, TokenId,
};

/// The result of projecting a `Standing` through a `StandingRegistry`.
///
/// Each kernel protocol field is resolved from the protocol bindings declared
/// on the tokens present in the standing map. Conflicts are collected in the
/// `conflicts` field rather than collapsing silently.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolProjection {
    pub review: Option<ReviewProjection>,
    pub process: Option<ProcessProjection>,
    pub visibility: VisibilityProjection,
    pub immutability: ImmutabilityProjection,
    pub conflicts: Vec<ProtocolConflict>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewProjection {
    Unreviewed,
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessProjection {
    Active,
    Blocked,
    Completed,
    Archived,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibilityProjection {
    pub include_in_standard_context: bool,
    pub expose_to_provider: bool,
}

impl VisibilityProjection {
    pub const fn defaults() -> Self {
        Self {
            include_in_standard_context: true,
            expose_to_provider: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImmutabilityProjection {
    Mutable,
    Sealed,
}

/// A structured conflict between two or more protocol projections originating
/// from different standing tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolConflict {
    pub protocol: KernelProtocolId,
    pub message: String,
    pub sources: Vec<ProtocolConflictSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolConflictSource {
    pub dimension: DimensionId,
    pub token: TokenId,
    pub projected_value: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProjectionError {
    #[error("unknown dimension '{0}' in standing at projection time")]
    UnknownDimension(String),
    #[error("unknown token '{token}' for dimension '{dimension}' at projection time")]
    UnknownToken { dimension: String, token: String },
    #[error("unknown protocol '{0}' in binding at projection time")]
    UnknownProtocol(String),
}

impl From<ProjectionError> for CoreError {
    fn from(e: ProjectionError) -> Self {
        CoreError::SchemaViolation(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Main projection entry point
// ---------------------------------------------------------------------------

/// Project a `Standing` through a `StandingRegistry` to produce a
/// `ProtocolProjection`.
///
/// Walk every (dimension, token) pair in the standing map, look up the token's
/// protocol bindings in the registry, collect per-protocol values, apply
/// conflict-resolution rules, and return the final projection along with any
/// structured conflicts.
pub fn project(
    standing: &Standing,
    registry: &StandingRegistry,
) -> Result<ProtocolProjection, ProjectionError> {
    let mut review_states: Vec<(String, DimensionId, TokenId)> = Vec::new();
    let mut process_states: Vec<(String, DimensionId, TokenId)> = Vec::new();
    let mut visibility_bindings: Vec<(BTreeMap<String, ScalarValue>, DimensionId, TokenId)> =
        Vec::new();
    let mut immutability_states: Vec<(String, DimensionId, TokenId)> = Vec::new();
    let mut conflicts: Vec<ProtocolConflict> = Vec::new();

    for (dim_id, token_id) in &standing.values {
        let dim_def = registry
            .dimensions
            .get(dim_id)
            .ok_or_else(|| ProjectionError::UnknownDimension(dim_id.as_str().to_string()))?;

        let token_def = dim_def
            .tokens
            .iter()
            .find(|t| t.id == *token_id)
            .ok_or_else(|| ProjectionError::UnknownToken {
                dimension: dim_id.as_str().to_string(),
                token: token_id.as_str().to_string(),
            })?;

        for binding in &token_def.implements {
            match binding.protocol.as_str() {
                "kernel:review" => {
                    if let Some(state) = &binding.state {
                        review_states.push((state.clone(), dim_id.clone(), token_id.clone()));
                    }
                }
                "kernel:process" => {
                    if let Some(state) = &binding.state {
                        process_states.push((state.clone(), dim_id.clone(), token_id.clone()));
                    }
                }
                "kernel:visibility" => {
                    visibility_bindings.push((
                        binding.properties.clone(),
                        dim_id.clone(),
                        token_id.clone(),
                    ));
                }
                "kernel:immutability" => {
                    if let Some(state) = &binding.state {
                        immutability_states.push((state.clone(), dim_id.clone(), token_id.clone()));
                    }
                }
                other => {
                    return Err(ProjectionError::UnknownProtocol(other.to_string()));
                }
            }
        }
    }

    // --- Resolve review ---
    let review = resolve_review(&review_states, &mut conflicts);

    // --- Resolve process ---
    let process = resolve_process(&process_states, &mut conflicts);

    // --- Resolve visibility (false wins) ---
    let visibility = resolve_visibility(&visibility_bindings);

    // --- Resolve immutability (sealed wins) ---
    let immutability = resolve_immutability(&immutability_states);

    Ok(ProtocolProjection {
        review,
        process,
        visibility,
        immutability,
        conflicts,
    })
}

/// Project visibility only (lightweight helper).
///
/// Unlike the full `project()`, this returns only the `VisibilityProjection`
/// and does not report conflicts. Useful for callers that only need the
/// visibility gate and want to avoid handling the full projection result.
pub fn project_visibility(
    standing: &Standing,
    registry: &StandingRegistry,
) -> VisibilityProjection {
    let mut visibility_bindings: Vec<(BTreeMap<String, ScalarValue>, DimensionId, TokenId)> =
        Vec::new();

    for (dim_id, token_id) in &standing.values {
        let Some(dim_def) = registry.dimensions.get(dim_id) else {
            continue;
        };
        let Some(token_def) = dim_def.tokens.iter().find(|t| t.id == *token_id) else {
            continue;
        };
        for binding in &token_def.implements {
            if binding.protocol.as_str() == "kernel:visibility" {
                visibility_bindings.push((
                    binding.properties.clone(),
                    dim_id.clone(),
                    token_id.clone(),
                ));
            }
        }
    }

    resolve_visibility(&visibility_bindings)
}

/// Project review only (lightweight helper).
///
/// Unlike the full `project()`, this returns only the review projection
/// and silently skips unknown dimension/token/protocol entries. Useful
/// for callers that only need the review gate.
pub fn project_review(
    standing: &Standing,
    registry: &StandingRegistry,
) -> Option<ReviewProjection> {
    let mut review_states: Vec<(String, DimensionId, TokenId)> = Vec::new();
    for (dim_id, token_id) in &standing.values {
        let Some(dim_def) = registry.dimensions.get(dim_id) else { continue; };
        let Some(token_def) = dim_def.tokens.iter().find(|t| t.id == *token_id) else { continue; };
        for binding in &token_def.implements {
            if binding.protocol.as_str() == "kernel:review" {
                if let Some(state) = &binding.state {
                    review_states.push((state.clone(), dim_id.clone(), token_id.clone()));
                }
            }
        }
    }
    let mut conflicts = Vec::new();
    resolve_review(&review_states, &mut conflicts)
}

/// Project process only (lightweight helper).
///
/// Unlike the full `project()`, this returns only the process projection
/// and silently skips unknown dimension/token/protocol entries.
pub fn project_process(
    standing: &Standing,
    registry: &StandingRegistry,
) -> Option<ProcessProjection> {
    let mut process_states: Vec<(String, DimensionId, TokenId)> = Vec::new();
    for (dim_id, token_id) in &standing.values {
        let Some(dim_def) = registry.dimensions.get(dim_id) else { continue; };
        let Some(token_def) = dim_def.tokens.iter().find(|t| t.id == *token_id) else { continue; };
        for binding in &token_def.implements {
            if binding.protocol.as_str() == "kernel:process" {
                if let Some(state) = &binding.state {
                    process_states.push((state.clone(), dim_id.clone(), token_id.clone()));
                }
            }
        }
    }
    let mut conflicts = Vec::new();
    resolve_process(&process_states, &mut conflicts)
}

/// Project immutability only (lightweight helper).
///
/// Unlike the full `project()`, this returns only the immutability projection
/// and silently skips unknown dimension/token/protocol entries.
pub fn project_immutability(
    standing: &Standing,
    registry: &StandingRegistry,
) -> ImmutabilityProjection {
    let mut immutability_states: Vec<(String, DimensionId, TokenId)> = Vec::new();
    for (dim_id, token_id) in &standing.values {
        let Some(dim_def) = registry.dimensions.get(dim_id) else { continue; };
        let Some(token_def) = dim_def.tokens.iter().find(|t| t.id == *token_id) else { continue; };
        for binding in &token_def.implements {
            if binding.protocol.as_str() == "kernel:immutability" {
                if let Some(state) = &binding.state {
                    immutability_states.push((state.clone(), dim_id.clone(), token_id.clone()));
                }
            }
        }
    }
    resolve_immutability(&immutability_states)
}

// ---------------------------------------------------------------------------
// Per-protocol resolution helpers
// ---------------------------------------------------------------------------

fn resolve_review(
    states: &[(String, DimensionId, TokenId)],
    conflicts: &mut Vec<ProtocolConflict>,
) -> Option<ReviewProjection> {
    if states.is_empty() {
        return None;
    }

    let distinct: std::collections::BTreeSet<&str> =
        states.iter().map(|(s, _, _)| s.as_str()).collect();

    if distinct.len() > 1 {
        let protocol = KernelProtocolId::new("kernel:review");
        let message = format!(
            "conflicting review projections: {}",
            distinct
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let sources = states
            .iter()
            .map(|(s, dim, tok)| ProtocolConflictSource {
                dimension: dim.clone(),
                token: tok.clone(),
                projected_value: s.clone(),
            })
            .collect();
        conflicts.push(ProtocolConflict {
            protocol,
            message,
            sources,
        });
        return None;
    }

    let state = states[0].0.as_str();
    match state {
        "unreviewed" => Some(ReviewProjection::Unreviewed),
        "pending" => Some(ReviewProjection::Pending),
        "accepted" => Some(ReviewProjection::Accepted),
        "rejected" => Some(ReviewProjection::Rejected),
        _ => None,
    }
}

fn resolve_process(
    states: &[(String, DimensionId, TokenId)],
    conflicts: &mut Vec<ProtocolConflict>,
) -> Option<ProcessProjection> {
    if states.is_empty() {
        return None;
    }

    let distinct: std::collections::BTreeSet<&str> =
        states.iter().map(|(s, _, _)| s.as_str()).collect();

    if distinct.len() > 1 {
        let protocol = KernelProtocolId::new("kernel:process");
        let message = format!(
            "conflicting process projections: {}",
            distinct
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let sources = states
            .iter()
            .map(|(s, dim, tok)| ProtocolConflictSource {
                dimension: dim.clone(),
                token: tok.clone(),
                projected_value: s.clone(),
            })
            .collect();
        conflicts.push(ProtocolConflict {
            protocol,
            message,
            sources,
        });
        return None;
    }

    let state = states[0].0.as_str();
    match state {
        "active" => Some(ProcessProjection::Active),
        "blocked" => Some(ProcessProjection::Blocked),
        "completed" => Some(ProcessProjection::Completed),
        "archived" => Some(ProcessProjection::Archived),
        _ => None,
    }
}

fn resolve_visibility(
    bindings: &[(BTreeMap<String, ScalarValue>, DimensionId, TokenId)],
) -> VisibilityProjection {
    if bindings.is_empty() {
        return VisibilityProjection::defaults();
    }

    // "false wins" for both boolean properties
    let include = bindings.iter().any(|(props, _, _)| {
        props
            .get("include_in_standard_context")
            .is_some_and(|v| matches!(v, ScalarValue::Bool(false)))
    });
    let expose = bindings.iter().any(|(props, _, _)| {
        props
            .get("expose_to_provider")
            .is_some_and(|v| matches!(v, ScalarValue::Bool(false)))
    });

    // If none said false, check if any said true
    let include_in_standard_context = if include {
        false
    } else {
        bindings.iter().any(|(props, _, _)| {
            props
                .get("include_in_standard_context")
                .is_some_and(|v| matches!(v, ScalarValue::Bool(true)))
        }) || VisibilityProjection::defaults().include_in_standard_context
    };

    let expose_to_provider = if expose {
        false
    } else {
        bindings.iter().any(|(props, _, _)| {
            props
                .get("expose_to_provider")
                .is_some_and(|v| matches!(v, ScalarValue::Bool(true)))
        }) || VisibilityProjection::defaults().expose_to_provider
    };

    VisibilityProjection {
        include_in_standard_context,
        expose_to_provider,
    }
}

fn resolve_immutability(states: &[(String, DimensionId, TokenId)]) -> ImmutabilityProjection {
    // "sealed wins" — if any binding says sealed, it's sealed
    for (state, _, _) in states {
        if state == "sealed" {
            return ImmutabilityProjection::Sealed;
        }
    }
    // If any binding says mutable (and none says sealed), it's mutable.
    // But if no bindings at all, default to mutable.
    ImmutabilityProjection::Mutable
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemDefinition;
    use std::collections::BTreeMap;

    fn kernel_registry() -> StandingRegistry {
        StandingRegistry::kernel_defaults()
    }

    fn custom_registry() -> StandingRegistry {
        use crate::{ProtocolBinding, StandingDimensionDefinition, StandingTokenDefinition};
        let sys = SystemDefinition {
            system_id: "sys_test".to_string(),
            namespace: "systems/test".to_string(),
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
                        implements: vec![
                            ProtocolBinding {
                                protocol: KernelProtocolId::from_static("kernel:review"),
                                state: Some("accepted".to_string()),
                                properties: BTreeMap::new(),
                            },
                            ProtocolBinding {
                                protocol: KernelProtocolId::from_static("kernel:visibility"),
                                state: None,
                                properties: BTreeMap::from([
                                    (
                                        "include_in_standard_context".to_string(),
                                        ScalarValue::Bool(true),
                                    ),
                                    ("expose_to_provider".to_string(), ScalarValue::Bool(true)),
                                ]),
                            },
                        ],
                    },
                ],
            }],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        StandingRegistry::from_system_definition(&sys).expect("custom registry")
    }

    // --- Test 1: Built-in kernel:review = accepted projects to review accepted ---

    #[test]
    fn test_builtin_review_accepted_projects_review_accepted() {
        let registry = kernel_registry();
        let standing = {
            let mut values = BTreeMap::new();
            values.insert(
                DimensionId::from_static("kernel:review"),
                TokenId::from_static("accepted"),
            );
            values.insert(
                DimensionId::from_static("kernel:epistemic"),
                TokenId::from_static("working"),
            );
            values.insert(
                DimensionId::from_static("kernel:process"),
                TokenId::from_static("active"),
            );
            Standing { values }
        };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.review, Some(ReviewProjection::Accepted));
        assert!(projection.conflicts.is_empty());
    }

    // --- Test 2: Custom research:status = verified projects to review accepted ---

    #[test]
    fn test_custom_dimension_projects_review_accepted() {
        let registry = custom_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("research:status"),
            TokenId::from_static("verified"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.review, Some(ReviewProjection::Accepted));
        assert_eq!(
            projection.visibility,
            VisibilityProjection {
                include_in_standard_context: true,
                expose_to_provider: true,
            }
        );
        assert_eq!(projection.immutability, ImmutabilityProjection::Mutable);
        assert!(projection.conflicts.is_empty());
    }

    // --- Test 3: Visibility defaults ---

    #[test]
    fn test_visibility_defaults_when_no_binding() {
        let registry = kernel_registry();
        // kernel:epistemic tokens have no visibility bindings
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        values.insert(
            DimensionId::from_static("kernel:review"),
            TokenId::from_static("unreviewed"),
        );
        values.insert(
            DimensionId::from_static("kernel:process"),
            TokenId::from_static("active"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(
            projection.visibility,
            VisibilityProjection {
                include_in_standard_context: true,
                expose_to_provider: false,
            }
        );
        assert!(projection.conflicts.is_empty());
    }

    // --- Test 4: Visibility false beats true ---

    #[test]
    fn test_visibility_false_beats_true() {
        let sys = SystemDefinition {
            system_id: "sys_conflict2".to_string(),
            namespace: "systems/conflict2".to_string(),
            title: "Conflict2".to_string(),
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
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:a"),
                    default: TokenId::from_static("visible"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("visible"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:visibility"),
                            state: None,
                            properties: BTreeMap::from([
                                (
                                    "include_in_standard_context".to_string(),
                                    ScalarValue::Bool(true),
                                ),
                                ("expose_to_provider".to_string(), ScalarValue::Bool(true)),
                            ]),
                        }],
                    }],
                },
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:b"),
                    default: TokenId::from_static("hidden"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("hidden"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:visibility"),
                            state: None,
                            properties: BTreeMap::from([(
                                "expose_to_provider".to_string(),
                                ScalarValue::Bool(false),
                            )]),
                        }],
                    }],
                },
            ],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("conflict registry");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:a"),
            TokenId::from_static("visible"),
        );
        values.insert(
            DimensionId::from_static("dim:b"),
            TokenId::from_static("hidden"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        // false beats true for expose_to_provider
        assert!(!projection.visibility.expose_to_provider);
        // no false for include_in_standard_context, so true from dim:a wins
        assert!(projection.visibility.include_in_standard_context);
        assert!(projection.conflicts.is_empty());
    }

    // --- Test 5: Immutability sealed beats mutable ---

    #[test]
    fn test_immutability_sealed_beats_mutable() {
        let sys = SystemDefinition {
            system_id: "sys_immut".to_string(),
            namespace: "systems/immut".to_string(),
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
            standing_dimensions: vec![
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:x"),
                    default: TokenId::from_static("mutable_token"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("mutable_token"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:immutability"),
                            state: Some("mutable".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:y"),
                    default: TokenId::from_static("sealed_token"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("sealed_token"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:immutability"),
                            state: Some("sealed".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
            ],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("immut registry");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:x"),
            TokenId::from_static("mutable_token"),
        );
        values.insert(
            DimensionId::from_static("dim:y"),
            TokenId::from_static("sealed_token"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.immutability, ImmutabilityProjection::Sealed);
        assert!(projection.conflicts.is_empty());
    }

    // --- Test 6: Conflicting review projections fail with structured conflict ---

    #[test]
    fn test_conflicting_review_projections_produce_conflict() {
        let sys = SystemDefinition {
            system_id: "sys_rev_conflict".to_string(),
            namespace: "systems/rev_conflict".to_string(),
            title: "RevConflict".to_string(),
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
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:review_one"),
                    default: TokenId::from_static("accepted_token"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("accepted_token"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:review_two"),
                    default: TokenId::from_static("rejected_token"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("rejected_token"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("rejected".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
            ],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("review conflict reg");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:review_one"),
            TokenId::from_static("accepted_token"),
        );
        values.insert(
            DimensionId::from_static("dim:review_two"),
            TokenId::from_static("rejected_token"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(
            projection.review, None,
            "review should be None due to conflict"
        );
        assert_eq!(projection.conflicts.len(), 1);
        let conflict = &projection.conflicts[0];
        assert_eq!(conflict.protocol.as_str(), "kernel:review");
        assert!(conflict.message.contains("accepted"));
        assert!(conflict.message.contains("rejected"));
        assert_eq!(conflict.sources.len(), 2);
        assert!(conflict
            .sources
            .iter()
            .any(|s| s.projected_value == "accepted"));
        assert!(conflict
            .sources
            .iter()
            .any(|s| s.projected_value == "rejected"));
    }

    // --- Test 7: Conflicting process projections fail with structured conflict ---

    #[test]
    fn test_conflicting_process_projections_produce_conflict() {
        let sys = SystemDefinition {
            system_id: "sys_proc_conflict".to_string(),
            namespace: "systems/proc_conflict".to_string(),
            title: "ProcConflict".to_string(),
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
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:proc_one"),
                    default: TokenId::from_static("active_token"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("active_token"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:process"),
                            state: Some("active".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:proc_two"),
                    default: TokenId::from_static("blocked_token"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("blocked_token"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:process"),
                            state: Some("blocked".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
            ],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("proc conflict reg");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:proc_one"),
            TokenId::from_static("active_token"),
        );
        values.insert(
            DimensionId::from_static("dim:proc_two"),
            TokenId::from_static("blocked_token"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(
            projection.process, None,
            "process should be None due to conflict"
        );
        assert_eq!(projection.conflicts.len(), 1);
        let conflict = &projection.conflicts[0];
        assert_eq!(conflict.protocol.as_str(), "kernel:process");
        assert!(conflict.message.contains("active"));
        assert!(conflict.message.contains("blocked"));
        assert_eq!(conflict.sources.len(), 2);
    }

    // --- Test 8: Identical repeated projections do not conflict ---

    #[test]
    fn test_identical_repeated_projections_no_conflict() {
        let sys = SystemDefinition {
            system_id: "sys_identical".to_string(),
            namespace: "systems/identical".to_string(),
            title: "Identical".to_string(),
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
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:alpha"),
                    default: TokenId::from_static("acc_a"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("acc_a"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:beta"),
                    default: TokenId::from_static("acc_b"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("acc_b"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
            ],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("identical reg");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:alpha"),
            TokenId::from_static("acc_a"),
        );
        values.insert(
            DimensionId::from_static("dim:beta"),
            TokenId::from_static("acc_b"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.review, Some(ReviewProjection::Accepted));
        assert!(projection.conflicts.is_empty());
    }

    // --- Test 9: Unknown dimension/token at projection time returns error ---

    #[test]
    fn test_unknown_dimension_returns_error() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("nonexistent:dim"),
            TokenId::from_static("whatever"),
        );
        let standing = Standing { values };

        let result = project(&standing, &registry);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProjectionError::UnknownDimension(_)));
        assert!(err.to_string().contains("nonexistent:dim"));
    }

    #[test]
    fn test_unknown_token_returns_error() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:review"),
            TokenId::from_static("nonexistent_token"),
        );
        let standing = Standing { values };

        let result = project(&standing, &registry);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProjectionError::UnknownToken { .. }));
    }

    // --- Additional: kernel:review = rejected projects review rejected ---

    #[test]
    fn test_builtin_review_rejected_projects_review_rejected() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:review"),
            TokenId::from_static("rejected"),
        );
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        values.insert(
            DimensionId::from_static("kernel:process"),
            TokenId::from_static("active"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.review, Some(ReviewProjection::Rejected));
        assert!(projection.conflicts.is_empty());
    }

    // --- Additional: kernel:process = completed projects process completed ---

    #[test]
    fn test_builtin_process_completed_projects_process_completed() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:process"),
            TokenId::from_static("completed"),
        );
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        values.insert(
            DimensionId::from_static("kernel:review"),
            TokenId::from_static("accepted"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.process, Some(ProcessProjection::Completed));
        assert!(projection.conflicts.is_empty());
    }

    // --- Additional: No review/process bindings yields None ---

    #[test]
    fn test_no_review_binding_yields_none() {
        let registry = kernel_registry();
        // kernel:epistemic tokens have no review binding
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        let standing = Standing { values };

        let projection = project(&standing, &registry).expect("projection should succeed");
        assert_eq!(projection.review, None);
        assert!(projection.conflicts.is_empty());
    }

    // --- Additional: Unknown protocol in binding returns error ---

    #[test]
    fn test_unknown_protocol_in_binding_returns_error() {
        let sys = SystemDefinition {
            system_id: "sys_bad_proto".to_string(),
            namespace: "systems/bad_proto".to_string(),
            title: "BadProto".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![crate::StandingDimensionDefinition {
                id: DimensionId::from_static("dim:bad"),
                default: TokenId::from_static("bad_token"),
                tokens: vec![crate::StandingTokenDefinition {
                    id: TokenId::from_static("bad_token"),
                    implements: vec![crate::ProtocolBinding {
                        protocol: KernelProtocolId::from_static("unknown:protocol"),
                        state: Some("thing".to_string()),
                        properties: BTreeMap::new(),
                    }],
                }],
            }],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("bad proto reg");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:bad"),
            TokenId::from_static("bad_token"),
        );
        let standing = Standing { values };

        let result = project(&standing, &registry);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProjectionError::UnknownProtocol(_)));
        assert!(err.to_string().contains("unknown:protocol"));
    }

    // --- Test project_visibility helper ---

    #[test]
    fn test_project_visibility_helper() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        let standing = Standing { values };

        let vis = project_visibility(&standing, &registry);
        assert!(vis.include_in_standard_context);
        assert!(!vis.expose_to_provider);
    }

    #[test]
    fn test_project_visibility_helper_custom() {
        let registry = custom_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("research:status"),
            TokenId::from_static("verified"),
        );
        let standing = Standing { values };

        let vis = project_visibility(&standing, &registry);
        assert!(vis.include_in_standard_context);
        assert!(vis.expose_to_provider);
    }

    #[test]
    fn test_project_visibility_helper_false_beats_true() {
        let sys = SystemDefinition {
            system_id: "sys_vis_help".to_string(),
            namespace: "systems/vis_help".to_string(),
            title: "VisHelp".to_string(),
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
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:a"),
                    default: TokenId::from_static("visible"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("visible"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:visibility"),
                            state: None,
                            properties: BTreeMap::from([
                                (
                                    "include_in_standard_context".to_string(),
                                    ScalarValue::Bool(true),
                                ),
                                ("expose_to_provider".to_string(), ScalarValue::Bool(true)),
                            ]),
                        }],
                    }],
                },
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:b"),
                    default: TokenId::from_static("hidden"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("hidden"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:visibility"),
                            state: None,
                            properties: BTreeMap::from([(
                                "include_in_standard_context".to_string(),
                                ScalarValue::Bool(false),
                            )]),
                        }],
                    }],
                },
            ],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("vis help registry");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:a"),
            TokenId::from_static("visible"),
        );
        values.insert(
            DimensionId::from_static("dim:b"),
            TokenId::from_static("hidden"),
        );
        let standing = Standing { values };

        // Use full project to verify
        let projection = project(&standing, &registry).expect("projection should succeed");
        assert!(!projection.visibility.include_in_standard_context);
        assert!(projection.visibility.expose_to_provider);

        let vis = project_visibility(&standing, &registry);
        assert!(!vis.include_in_standard_context);
        assert!(vis.expose_to_provider);
    }

    // --- Tests for lightweight project_review helper ---

    #[test]
    fn test_project_review_returns_accepted() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:review"),
            TokenId::from_static("accepted"),
        );
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        values.insert(
            DimensionId::from_static("kernel:process"),
            TokenId::from_static("active"),
        );
        let standing = Standing { values };

        let proj = project_review(&standing, &registry);
        assert_eq!(proj, Some(ReviewProjection::Accepted));
    }

    #[test]
    fn test_project_review_returns_none_for_no_bindings() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        let standing = Standing { values };

        let proj = project_review(&standing, &registry);
        assert_eq!(proj, None);
    }

    #[test]
    fn test_project_review_conflict_returns_none() {
        let sys = SystemDefinition {
            system_id: "sys_rev_conflict_lite".to_string(),
            namespace: "systems/rev_conflict_lite".to_string(),
            title: "RevConflictLite".to_string(),
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
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:rcl_a"),
                    default: TokenId::from_static("accepted_tok"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("accepted_tok"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
                crate::StandingDimensionDefinition {
                    id: DimensionId::from_static("dim:rcl_b"),
                    default: TokenId::from_static("rejected_tok"),
                    tokens: vec![crate::StandingTokenDefinition {
                        id: TokenId::from_static("rejected_tok"),
                        implements: vec![crate::ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("rejected".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    }],
                },
            ],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("conflict reg");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:rcl_a"),
            TokenId::from_static("accepted_tok"),
        );
        values.insert(
            DimensionId::from_static("dim:rcl_b"),
            TokenId::from_static("rejected_tok"),
        );
        let standing = Standing { values };

        let proj = project_review(&standing, &registry);
        assert_eq!(proj, None, "conflicting review should return None");
    }

    // --- Tests for lightweight project_process helper ---

    #[test]
    fn test_project_process_returns_active() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:process"),
            TokenId::from_static("active"),
        );
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        values.insert(
            DimensionId::from_static("kernel:review"),
            TokenId::from_static("unreviewed"),
        );
        let standing = Standing { values };

        let proj = project_process(&standing, &registry);
        assert_eq!(proj, Some(ProcessProjection::Active));
    }

    #[test]
    fn test_project_process_returns_none_for_no_bindings() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        let standing = Standing { values };

        let proj = project_process(&standing, &registry);
        assert_eq!(proj, None);
    }

    // --- Tests for lightweight project_immutability helper ---

    #[test]
    fn test_project_immutability_returns_mutable_by_default() {
        let registry = kernel_registry();
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("kernel:epistemic"),
            TokenId::from_static("working"),
        );
        let standing = Standing { values };

        let imm = project_immutability(&standing, &registry);
        assert_eq!(imm, ImmutabilityProjection::Mutable);
    }

    #[test]
    fn test_project_immutability_returns_sealed() {
        let sys = SystemDefinition {
            system_id: "sys_immut_lite".to_string(),
            namespace: "systems/immut_lite".to_string(),
            title: "ImmutLite".to_string(),
            description: None,
            classes: vec![],
            instructions: vec![],
            policies: vec![],
            workflows: vec![],
            compiled_contexts: vec![],
            provider_profiles: vec![],
            default_compiled_context: None,
            default_provider_profile: None,
            standing_dimensions: vec![crate::StandingDimensionDefinition {
                id: DimensionId::from_static("dim:seal"),
                default: TokenId::from_static("sealed_token"),
                tokens: vec![crate::StandingTokenDefinition {
                    id: TokenId::from_static("sealed_token"),
                    implements: vec![crate::ProtocolBinding {
                        protocol: KernelProtocolId::from_static("kernel:immutability"),
                        state: Some("sealed".to_string()),
                        properties: BTreeMap::new(),
                    }],
                }],
            }],
            runtime_profile: crate::RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys).expect("immut reg");
        let mut values = BTreeMap::new();
        values.insert(
            DimensionId::from_static("dim:seal"),
            TokenId::from_static("sealed_token"),
        );
        let standing = Standing { values };

        let imm = project_immutability(&standing, &registry);
        assert_eq!(imm, ImmutabilityProjection::Sealed);
    }
}
