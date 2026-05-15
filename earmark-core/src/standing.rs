//! Standing state and registry management.

use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::errors::CoreError;
use crate::ids::{DimensionId, KernelProtocolId, ObjectId, TokenId};
use crate::system::SystemDefinition;
use crate::values::ScalarValue;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::RuntimeProfile;

    #[test]
    fn test_standing_serializes_as_clean_map() {
        let standing = Standing::kernel_defaults();
        let json = serde_json::to_string(&standing).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let map = parsed.as_object().unwrap();
        assert!(map.contains_key("kernel:epistemic"));
        assert!(map.contains_key("kernel:review"));
        assert!(map.contains_key("kernel:process"));
        assert_eq!(map["kernel:epistemic"], "working");
        assert_eq!(map["kernel:review"], "unreviewed");
        assert_eq!(map["kernel:process"], "active");
        assert!(
            !json.contains("epistemic_standing"),
            "should not contain old type names"
        );
    }

    #[test]
    fn test_standing_old_format_no_longer_normalizes() {
        // Legacy keys now deserialize as bare dimension IDs (not kernel:*).
        let old_json = r#"{"epistemic": "working", "review": "unreviewed", "process": "active"}"#;
        let standing: Standing = serde_json::from_str(old_json).unwrap();
        assert_eq!(
            standing.get(&DimensionId::new("epistemic")),
            Some(&TokenId::new("working"))
        );
        assert_eq!(
            standing.get(&DimensionId::new("kernel:epistemic")),
            None,
            "bare 'epistemic' must not normalize to 'kernel:epistemic'"
        );
    }

    #[test]
    fn test_standing_new_format_deserializes() {
        let new_json = r#"{"kernel:epistemic": "supported", "kernel:review": "accepted", "kernel:process": "completed"}"#;
        let standing: Standing = serde_json::from_str(new_json).unwrap();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("supported")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("accepted")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:process"))
                .map(TokenId::as_str),
            Some("completed")
        );
    }

    #[test]
    fn test_kernel_registry_contains_builtin_dimensions() {
        let registry = StandingRegistry::kernel_defaults();
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:epistemic")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:review")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:process")));
    }

    #[test]
    fn test_materialize_defaults_fills_omitted_dimensions() {
        let registry = StandingRegistry::kernel_defaults();
        let supplied = BTreeMap::new();
        let standing = materialize_defaults(&registry, supplied).unwrap();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("working")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:review"))
                .map(TokenId::as_str),
            Some("unreviewed")
        );
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:process"))
                .map(TokenId::as_str),
            Some("active")
        );
    }

    #[test]
    fn test_new_writes_do_not_emit_old_shape() {
        let standing = Standing::kernel_defaults();
        let yaml = serde_yaml::to_string(&standing).unwrap();
        // New format uses kernel: prefixes
        assert!(yaml.contains("kernel:epistemic"));
        // Old format would have had "epistemic: working" as a flat field
        // The new format should NOT contain old-style single-word keys as top-level map entries
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let map = value.as_mapping().unwrap();
        for key in map.keys() {
            let ks = key.as_str().unwrap();
            assert!(
                ks.contains(':'),
                "new standing serialization should use namespaced keys, got: {}",
                ks
            );
        }
    }

    #[test]
    fn test_materialize_defaults_rejects_unknown_token() {
        let registry = StandingRegistry::kernel_defaults();
        let mut supplied = BTreeMap::new();
        supplied.insert(
            DimensionId::new("kernel:epistemic"),
            TokenId::new("nonexistent"),
        );
        assert!(materialize_defaults(&registry, supplied).is_err());
    }

    #[test]
    fn test_standing_is_empty_after_clear() {
        let mut standing = Standing::default();
        assert!(!standing.is_empty());
        standing.values.clear();
        assert!(standing.is_empty());
        assert_eq!(standing.len(), 0);
    }

    #[test]
    fn test_standing_deserialize_rejects_invalid_dimension() {
        let bad = r#"{"UPPERCASE_DIM": "value"}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
        let bad = r#"{"": "value"}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
    }

    #[test]
    fn test_standing_deserialize_rejects_invalid_token() {
        let bad = r#"{"kernel:epistemic": "UPPERCASE_TOKEN"}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
        let bad = r#"{"kernel:epistemic": ""}"#;
        assert!(serde_json::from_str::<Standing>(bad).is_err());
    }

    #[test]
    fn test_materialize_defaults_rejects_unknown_dimension() {
        let registry = StandingRegistry::kernel_defaults();
        let mut supplied = BTreeMap::new();
        supplied.insert(DimensionId::new("unknown:dimension"), TokenId::new("value"));
        let result = materialize_defaults(&registry, supplied);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unknown dimension"),
            "error should mention unknown dimension"
        );
    }

    #[test]
    fn test_materialize_defaults_rejects_unknown_dimension_token() {
        let registry = StandingRegistry::kernel_defaults();
        let mut supplied = BTreeMap::new();
        supplied.insert(
            DimensionId::new("kernel:epistemic"),
            TokenId::new("nonexistent"),
        );
        let result = materialize_defaults(&registry, supplied);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("unknown token"),
            "error should mention unknown token"
        );
    }

    #[test]
    fn test_builtin_registry_validates_successfully() {
        let registry = StandingRegistry::kernel_defaults();
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn test_kernel_review_tokens_all_bind_to_kernel_review() {
        let registry = StandingRegistry::kernel_defaults();
        let review = registry
            .dimensions
            .get(&DimensionId::new("kernel:review"))
            .expect("kernel:review dimension");
        for token in &review.tokens {
            let has_review_binding = token.implements.iter().any(|b| {
                b.protocol.as_str() == "kernel:review"
                    && b.state.as_deref() == Some(token.id.as_str())
            });
            assert!(
                has_review_binding,
                "token '{}' should bind to kernel:review state '{}'",
                token.id.as_str(),
                token.id.as_str()
            );
        }
    }

    #[test]
    fn test_kernel_process_tokens_all_bind_to_kernel_process() {
        let registry = StandingRegistry::kernel_defaults();
        let process = registry
            .dimensions
            .get(&DimensionId::new("kernel:process"))
            .expect("kernel:process dimension");
        for token in &process.tokens {
            let has_process_binding = token.implements.iter().any(|b| {
                b.protocol.as_str() == "kernel:process"
                    && b.state.as_deref() == Some(token.id.as_str())
            });
            assert!(
                has_process_binding,
                "token '{}' should bind to kernel:process state '{}'",
                token.id.as_str(),
                token.id.as_str()
            );
        }
    }

    #[test]
    fn test_kernel_epistemic_default_is_working_not_unresolved() {
        // Standing::kernel_defaults() uses working
        let standing = Standing::kernel_defaults();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("working")
        );
        // Standing::default() delegates to kernel_defaults()
        let standing = Standing::default();
        assert_eq!(
            standing
                .get(&DimensionId::new("kernel:epistemic"))
                .map(TokenId::as_str),
            Some("working")
        );
        // Registry default is also working
        let registry = StandingRegistry::kernel_defaults();
        let epi = registry
            .dimensions
            .get(&DimensionId::new("kernel:epistemic"))
            .expect("kernel:epistemic dimension");
        assert_eq!(epi.default.as_str(), "working");
    }

    #[test]
    fn test_system_definition_with_custom_standing_dimensions_parses() {
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
                        implements: vec![ProtocolBinding {
                            protocol: KernelProtocolId::from_static("kernel:review"),
                            state: Some("accepted".to_string()),
                            properties: BTreeMap::new(),
                        }],
                    },
                ],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry = StandingRegistry::from_system_definition(&sys)
            .expect("registry construction should succeed");
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:epistemic")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:review")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("kernel:process")));
        assert!(registry
            .dimensions
            .contains_key(&DimensionId::new("research:status")));
    }

    #[test]
    fn test_registry_construction_includes_builtin_and_custom() {
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
                id: DimensionId::from_static("security:clearance"),
                default: TokenId::from_static("public"),
                tokens: vec![
                    StandingTokenDefinition {
                        id: TokenId::from_static("public"),
                        implements: vec![],
                    },
                    StandingTokenDefinition {
                        id: TokenId::from_static("restricted"),
                        implements: vec![],
                    },
                ],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry =
            StandingRegistry::from_system_definition(&sys).expect("registry should succeed");
        assert_eq!(registry.dimensions.len(), 4);
        let epi_default = registry
            .dimensions
            .get(&DimensionId::new("kernel:epistemic"))
            .map(|d| d.default.as_str());
        assert_eq!(epi_default, Some("working"));
        let clearance = registry
            .dimensions
            .get(&DimensionId::new("security:clearance"))
            .expect("security:clearance dimension");
        assert_eq!(clearance.default.as_str(), "public");
    }

    #[test]
    fn test_duplicate_dimension_fails_validation() {
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
                id: DimensionId::from_static("kernel:epistemic"),
                default: TokenId::from_static("working"),
                tokens: vec![StandingTokenDefinition {
                    id: TokenId::from_static("working"),
                    implements: vec![],
                }],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        assert!(StandingRegistry::from_system_definition(&sys).is_err());
    }

    #[test]
    fn test_default_token_missing_from_token_list_fails() {
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
                default: TokenId::from_static("missing_token"),
                tokens: vec![StandingTokenDefinition {
                    id: TokenId::from_static("draft"),
                    implements: vec![],
                }],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        assert!(StandingRegistry::from_system_definition(&sys).is_err());
    }

    #[test]
    fn test_unknown_protocol_binding_fails() {
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
                tokens: vec![StandingTokenDefinition {
                    id: TokenId::from_static("verified"),
                    implements: vec![ProtocolBinding {
                        protocol: KernelProtocolId::from_static("nonexistent:protocol"),
                        state: Some("x".to_string()),
                        properties: BTreeMap::new(),
                    }],
                }],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        assert!(StandingRegistry::from_system_definition(&sys).is_err());
    }

    #[test]
    fn test_unknown_dimension_in_class_standing_rules_fails_registry_validation() {
        let registry = StandingRegistry::kernel_defaults();
        let _rules = ClassStandingRules {
            allowed_standing: BTreeMap::from([(
                DimensionId::from_static("unknown:dim"),
                vec![TokenId::from_static("unknown_token")],
            )]),
            ..Default::default()
        };
        assert!(!registry
            .dimensions
            .contains_key(&DimensionId::new("unknown:dim")));
    }

    #[test]
    fn test_class_standing_rules_against_registry_checks() {
        let registry = StandingRegistry::kernel_defaults();
        let dim_id = DimensionId::new("kernel:epistemic");
        let def = registry.dimensions.get(&dim_id).expect("kernel:epistemic");
        let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        assert!(valid.contains(&"working"));
        assert!(!valid.contains(&"bogus"));
        let unknown_id = DimensionId::new("research:status");
        assert!(!registry.dimensions.contains_key(&unknown_id));
    }

    #[test]
    fn test_standing_policy_unknown_dimension_fails_registry_validation() {
        let registry = StandingRegistry::kernel_defaults();
        let dim_id = DimensionId::parse("nonexistent").expect("valid dim id");
        assert!(!registry.dimensions.contains_key(&dim_id));
    }

    #[test]
    fn test_custom_dimension_compiled_context_filter_passes_registry_validation() {
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
                        implements: vec![],
                    },
                ],
            }],
            runtime_profile: RuntimeProfile {
                execution_surface: "local".to_string(),
                machine_output_default: "json".to_string(),
                work_surface_mode: "strict".to_string(),
            },
            activated_at: None,
        };
        let registry =
            StandingRegistry::from_system_definition(&sys).expect("registry construction");
        let dim_id = DimensionId::new("research:status");
        let def = registry.dimensions.get(&dim_id).expect("research:status");
        let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        assert!(valid.contains(&"draft"));
        assert!(valid.contains(&"verified"));
        assert!(!valid.contains(&"bogus"));
    }

    #[test]
    fn test_system_definition_without_standing_dimensions_defaults_empty() {
        let yaml = r#"
system_id: minimal
namespace: systems/minimal
title: Minimal System
classes: []
instructions: []
policies: []
workflows: []
compiled_contexts: []
provider_profiles: []
runtime_profile:
  execution_surface: local
  machine_output_default: json
  work_surface_mode: strict
"#;
        let sys: SystemDefinition =
            serde_yaml::from_str(yaml).expect("system without standing_dimensions should parse");
        assert!(sys.standing_dimensions.is_empty());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub values: BTreeMap<DimensionId, TokenId>,
}

impl Standing {
    pub fn kernel_defaults() -> Self {
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
        Self { values }
    }

    pub fn get(&self, dim: &DimensionId) -> Option<&TokenId> {
        self.values.get(dim)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&DimensionId, &TokenId)> {
        self.values.iter()
    }
}

impl Default for Standing {
    fn default() -> Self {
        Self::kernel_defaults()
    }
}

impl Serialize for Standing {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.values.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Standing {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw: BTreeMap<String, String> = BTreeMap::deserialize(deserializer)?;
        let mut values = BTreeMap::new();
        for (k, v) in raw {
            let dim = DimensionId::parse(&k).map_err(D::Error::custom)?;
            let token = TokenId::parse(&v).map_err(D::Error::custom)?;
            values.insert(dim, token);
        }
        Ok(Standing { values })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingDimensionDefinition {
    pub id: DimensionId,
    pub default: TokenId,
    pub tokens: Vec<StandingTokenDefinition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTokenDefinition {
    pub id: TokenId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<ProtocolBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolBinding {
    pub protocol: KernelProtocolId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, ScalarValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingRegistry {
    pub dimensions: BTreeMap<DimensionId, StandingDimensionDefinition>,
}

impl StandingRegistry {
    /// Validate internal coherence of the registry.
    pub fn validate(&self) -> Result<(), CoreError> {
        for (dim_id, def) in &self.dimensions {
            DimensionId::parse(dim_id.as_str())?;
            if def.default.as_str().is_empty() {
                return Err(CoreError::InvalidIdentifier(format!(
                    "dimension '{}' has empty default token",
                    dim_id.as_str(),
                )));
            }
            let token_ids: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
            if !token_ids.contains(&def.default.as_str()) {
                return Err(CoreError::InvalidIdentifier(format!(
                    "default token '{}' for dimension '{}' not found in its token list",
                    def.default.as_str(),
                    dim_id.as_str(),
                )));
            }

            let mut seen_tokens = std::collections::BTreeSet::new();
            for token in &def.tokens {
                TokenId::parse(token.id.as_str())?;
                if !seen_tokens.insert(token.id.as_str()) {
                    return Err(CoreError::InvalidIdentifier(format!(
                        "duplicate token '{}' in dimension '{}'",
                        token.id.as_str(),
                        dim_id.as_str(),
                    )));
                }
                for binding in &token.implements {
                    KernelProtocolId::parse(binding.protocol.as_str())?;
                }
            }
        }
        Ok(())
    }

    /// Build a registry from a `SystemDefinition` plus built-in kernel defaults.
    pub fn from_system_definition(system: &SystemDefinition) -> Result<Self, CoreError> {
        let mut dimensions = Self::kernel_defaults().dimensions;
        for dim_def in &system.standing_dimensions {
            if dimensions.contains_key(&dim_def.id) {
                return Err(CoreError::InvalidIdentifier(format!(
                    "duplicate dimension '{}': cannot override built-in or already declared dimension",
                    dim_def.id.as_str()
                )));
            }
            dimensions.insert(dim_def.id.clone(), dim_def.clone());
        }
        let registry = Self { dimensions };
        registry.validate()?;
        Ok(registry)
    }

    pub fn kernel_defaults() -> Self {
        let mut dimensions = BTreeMap::new();

        let epistemic = StandingDimensionDefinition {
            id: DimensionId::new("kernel:epistemic"),
            default: TokenId::new("working"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::new("unresolved"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("working"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("supported"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("contested"),
                    implements: vec![],
                },
                StandingTokenDefinition {
                    id: TokenId::new("superseded"),
                    implements: vec![],
                },
            ],
        };
        dimensions.insert(epistemic.id.clone(), epistemic);

        fn review_binding(state: &str) -> ProtocolBinding {
            ProtocolBinding {
                protocol: KernelProtocolId::from_static("kernel:review"),
                state: Some(state.to_string()),
                properties: BTreeMap::new(),
            }
        }

        let review = StandingDimensionDefinition {
            id: DimensionId::new("kernel:review"),
            default: TokenId::new("unreviewed"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::new("unreviewed"),
                    implements: vec![review_binding("unreviewed")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("pending"),
                    implements: vec![review_binding("pending")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("accepted"),
                    implements: vec![review_binding("accepted")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("rejected"),
                    implements: vec![review_binding("rejected")],
                },
            ],
        };
        dimensions.insert(review.id.clone(), review);

        fn process_binding(state: &str) -> ProtocolBinding {
            ProtocolBinding {
                protocol: KernelProtocolId::from_static("kernel:process"),
                state: Some(state.to_string()),
                properties: BTreeMap::new(),
            }
        }

        let process = StandingDimensionDefinition {
            id: DimensionId::new("kernel:process"),
            default: TokenId::new("active"),
            tokens: vec![
                StandingTokenDefinition {
                    id: TokenId::new("active"),
                    implements: vec![process_binding("active")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("blocked"),
                    implements: vec![process_binding("blocked")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("completed"),
                    implements: vec![process_binding("completed")],
                },
                StandingTokenDefinition {
                    id: TokenId::new("archived"),
                    implements: vec![process_binding("archived")],
                },
            ],
        };
        dimensions.insert(process.id.clone(), process);

        Self { dimensions }
    }
}

pub fn materialize_defaults(
    registry: &StandingRegistry,
    supplied: BTreeMap<DimensionId, TokenId>,
) -> Result<Standing, CoreError> {
    let mut values = BTreeMap::new();
    for (dim_id, token) in &supplied {
        let def = registry.dimensions.get(dim_id).ok_or_else(|| {
            CoreError::InvalidIdentifier(format!(
                "unknown dimension '{}' in supplied standing",
                dim_id.as_str()
            ))
        })?;
        let valid_tokens: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        if !valid_tokens.contains(&token.as_str()) {
            return Err(CoreError::InvalidIdentifier(format!(
                "unknown token '{}' for dimension '{}'",
                token.as_str(),
                dim_id.as_str(),
            )));
        }
        values.insert(dim_id.clone(), token.clone());
    }

    for (dim_id, def) in &registry.dimensions {
        if !values.contains_key(dim_id) {
            let default_token = &def.default;
            let valid_tokens: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
            if !valid_tokens.contains(&default_token.as_str()) {
                return Err(CoreError::InvalidIdentifier(format!(
                    "default token '{}' for dimension '{}' is not in its own token list",
                    default_token.as_str(),
                    dim_id.as_str(),
                )));
            }
            values.insert(dim_id.clone(), default_token.clone());
        }
    }

    Ok(Standing { values })
}

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ClassStandingRules {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub allowed_standing: BTreeMap<DimensionId, Vec<TokenId>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub required_protocols: BTreeMap<KernelProtocolId, BTreeMap<String, ScalarValue>>,
}

impl<'de> Deserialize<'de> for ClassStandingRules {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::collections::BTreeMap as Map;

        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            allowed_standing: Option<Map<String, Vec<String>>>,
            #[serde(default)]
            required_protocols: Option<Map<String, Map<String, serde_json::Value>>>,
        }

        let raw = Raw::deserialize(deserializer)?;

        let mut map = BTreeMap::new();
        if let Some(allowed_standing) = raw.allowed_standing {
            for (k, v) in allowed_standing {
                let dim = DimensionId::parse(&k).map_err(D::Error::custom)?;
                let mut tokens = Vec::new();
                for t in v {
                    tokens.push(TokenId::parse(&t).map_err(D::Error::custom)?);
                }
                map.insert(dim, tokens);
            }
        }
        let mut protocols = BTreeMap::new();
        if let Some(rp) = raw.required_protocols {
            for (k, v) in rp {
                let pid = KernelProtocolId::parse(&k).map_err(D::Error::custom)?;
                let props: BTreeMap<String, ScalarValue> = v
                    .into_iter()
                    .map(|(pk, pv)| {
                        let sv = serde_json::from_value(pv.clone())
                            .unwrap_or(ScalarValue::String(pv.to_string()));
                        (pk, sv)
                    })
                    .collect();
                protocols.insert(pid, props);
            }
        }

        Ok(ClassStandingRules {
            allowed_standing: map,
            required_protocols: protocols,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingConstraint {
    pub constraint_type: String,
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StandingFilter {
    #[serde(default)]
    pub allowed: BTreeMap<DimensionId, Vec<TokenId>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingTransitionRequest {
    pub target_object_id: ObjectId,
    pub dimension: String,
    pub from_value: String,
    pub to_value: String,
    pub rationale: Option<String>,
    pub status: StandingRequestStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandingRequestStatus {
    Proposed,
    Approved,
    Rejected,
    Applied,
    Superseded,
}

pub fn validate_standing_request(request: &StandingTransitionRequest) -> Result<(), String> {
    if request.dimension.is_empty() {
        return Err("dimension must be non-empty".to_string());
    }
    if request.from_value.is_empty() || request.to_value.is_empty() {
        return Err("from_value and to_value must be non-empty".to_string());
    }
    if request.from_value == request.to_value {
        return Err("to_value must differ from from_value".to_string());
    }
    Ok(())
}
