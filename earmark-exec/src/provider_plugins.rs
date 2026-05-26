use crate::error::ProviderFailure;
use crate::provider::{
    ProviderAdapter, ProviderCapability, ProviderCapabilityStatus, ProviderRegistry,
};
use earmark_core::{ProviderProfile, ProviderRequest, ProviderResponse, ScalarValue};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

const PROVIDER_PLUGIN_SCHEMA: &str = "earmark.provider_plugin.v1";

#[derive(Debug, Error)]
pub enum ProviderPluginLoadError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("invalid provider plugin manifest {path}: {message}")]
    Invalid { path: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LoadedProviderPlugin {
    pub path: String,
    pub name: String,
    pub version: String,
    pub provider_count: usize,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProviderPluginManifest {
    schema: String,
    name: String,
    version: String,
    description: Option<String>,
    providers: Vec<ProviderPluginProvider>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProviderPluginProvider {
    provider: String,
    adapter: String,
    #[serde(default)]
    required_env: Vec<String>,
    feature: Option<String>,
    message: Option<String>,
}

struct AliasedProviderAdapter {
    alias: String,
    base_adapter_key: String,
    plugin_name: String,
    plugin_version: String,
    description: Option<String>,
    feature: Option<String>,
    required_env: Vec<String>,
    message: Option<String>,
    inner: Arc<dyn ProviderAdapter>,
}

impl ProviderAdapter for AliasedProviderAdapter {
    fn provider_key(&self) -> &str {
        &self.alias
    }

    fn provide(
        &self,
        request: ProviderRequest,
        profile: &ProviderProfile,
        transition_operation: &str,
    ) -> Result<ProviderResponse, ProviderFailure> {
        let mut response = self.inner.provide(request, profile, transition_operation)?;
        response.provider = self.alias.clone();
        response.metadata.insert(
            "provider_plugin".to_string(),
            ScalarValue::String(self.plugin_name.clone()),
        );
        response.metadata.insert(
            "provider_plugin_version".to_string(),
            ScalarValue::String(self.plugin_version.clone()),
        );
        response.metadata.insert(
            "provider_plugin_adapter".to_string(),
            ScalarValue::String(self.base_adapter_key.clone()),
        );
        Ok(response)
    }

    fn capability(&self) -> ProviderCapability {
        let base = self.inner.capability();
        let missing_env = self
            .required_env
            .iter()
            .filter(|key| env::var(key.as_str()).is_err())
            .cloned()
            .collect::<Vec<_>>();

        let mut capability = ProviderCapability {
            provider: self.alias.clone(),
            status: if !missing_env.is_empty() {
                ProviderCapabilityStatus::MissingConfiguration
            } else {
                base.status
            },
            feature: self.feature.clone().or(base.feature),
            required_env: merge_unique(&base.required_env, &self.required_env),
            missing_env: merge_unique(&base.missing_env, &missing_env),
            message: self.message.clone().or(base.message),
        };

        if capability.message.is_none() {
            let mut parts = vec![format!(
                "provider plugin alias for adapter `{}` from {} {}",
                self.base_adapter_key, self.plugin_name, self.plugin_version
            )];
            if let Some(description) = &self.description {
                parts.push(description.clone());
            }
            capability.message = Some(parts.join("; "));
        }

        capability
    }

    fn can_satisfy_must_include_lineage(&self) -> bool {
        self.inner.can_satisfy_must_include_lineage()
    }

    fn can_satisfy_full_message_capture(&self) -> bool {
        self.inner.can_satisfy_full_message_capture()
    }
}

pub fn register_provider_plugins_from_dirs(
    registry: &mut ProviderRegistry,
    dirs: &[PathBuf],
) -> Result<Vec<LoadedProviderPlugin>, ProviderPluginLoadError> {
    let mut loaded = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if !dir.is_dir() {
            return Err(ProviderPluginLoadError::Invalid {
                path: dir.display().to_string(),
                message: "plugin path exists but is not a directory".to_string(),
            });
        }
        let mut entries = fs::read_dir(dir)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|entry| entry.path())
            .filter(|path| is_yaml(path))
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            let manifest = load_manifest(&path)?;
            apply_manifest(registry, &manifest, &path)?;
            loaded.push(LoadedProviderPlugin {
                path: path.display().to_string(),
                name: manifest.name,
                version: manifest.version,
                provider_count: manifest.providers.len(),
            });
        }
    }
    Ok(loaded)
}

fn apply_manifest(
    registry: &mut ProviderRegistry,
    manifest: &ProviderPluginManifest,
    path: &Path,
) -> Result<(), ProviderPluginLoadError> {
    for provider in &manifest.providers {
        let Some(inner) = registry.get(&provider.adapter) else {
            return Err(ProviderPluginLoadError::Invalid {
                path: path.display().to_string(),
                message: format!(
                    "provider `{}` references unknown adapter `{}`",
                    provider.provider, provider.adapter
                ),
            });
        };
        let adapter = AliasedProviderAdapter {
            alias: provider.provider.clone(),
            base_adapter_key: provider.adapter.clone(),
            plugin_name: manifest.name.clone(),
            plugin_version: manifest.version.clone(),
            description: manifest.description.clone(),
            feature: provider.feature.clone(),
            required_env: provider.required_env.clone(),
            message: provider.message.clone(),
            inner,
        };
        registry.register(Arc::new(adapter));
    }
    Ok(())
}

fn load_manifest(path: &Path) -> Result<ProviderPluginManifest, ProviderPluginLoadError> {
    let text = fs::read_to_string(path)?;
    let manifest: ProviderPluginManifest = serde_yaml::from_str(&text)?;
    validate_manifest(&manifest, path)?;
    Ok(manifest)
}

fn validate_manifest(
    manifest: &ProviderPluginManifest,
    path: &Path,
) -> Result<(), ProviderPluginLoadError> {
    if manifest.schema != PROVIDER_PLUGIN_SCHEMA {
        return Err(ProviderPluginLoadError::Invalid {
            path: path.display().to_string(),
            message: format!(
                "expected schema `{}`, found `{}`",
                PROVIDER_PLUGIN_SCHEMA, manifest.schema
            ),
        });
    }
    if manifest.name.trim().is_empty() {
        return Err(ProviderPluginLoadError::Invalid {
            path: path.display().to_string(),
            message: "plugin name must not be empty".to_string(),
        });
    }
    if manifest.providers.is_empty() {
        return Err(ProviderPluginLoadError::Invalid {
            path: path.display().to_string(),
            message: "plugin must declare at least one provider alias".to_string(),
        });
    }

    let mut seen = BTreeMap::new();
    for provider in &manifest.providers {
        if provider.provider.trim().is_empty() {
            return Err(ProviderPluginLoadError::Invalid {
                path: path.display().to_string(),
                message: "provider alias must not be empty".to_string(),
            });
        }
        if provider.adapter.trim().is_empty() {
            return Err(ProviderPluginLoadError::Invalid {
                path: path.display().to_string(),
                message: format!(
                    "provider `{}` must reference a non-empty adapter key",
                    provider.provider
                ),
            });
        }
        if seen.insert(provider.provider.clone(), ()).is_some() {
            return Err(ProviderPluginLoadError::Invalid {
                path: path.display().to_string(),
                message: format!(
                    "provider alias `{}` is declared more than once",
                    provider.provider
                ),
            });
        }
    }
    Ok(())
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yaml" | "yml")
    )
}

fn merge_unique(left: &[String], right: &[String]) -> Vec<String> {
    let mut merged = left.to_vec();
    for item in right {
        if !merged.iter().any(|existing| existing == item) {
            merged.push(item.clone());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::default_provider_registry;
    use tempfile::tempdir;

    #[test]
    fn registers_provider_alias_from_plugin_manifest() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("openai_http.yaml");
        fs::write(
            &path,
            r#"
schema: earmark.provider_plugin.v1
name: openai-http
version: 0.1.0
providers:
  - provider: mock_alias
    adapter: mock
    required_env:
      - OPENAI_API_KEY
"#,
        )
        .unwrap();

        let mut registry = default_provider_registry();
        let loaded =
            register_provider_plugins_from_dirs(&mut registry, &[dir.path().to_path_buf()])
                .unwrap();

        assert_eq!(loaded.len(), 1);
        let capability = registry
            .capabilities()
            .into_iter()
            .find(|capability| capability.provider == "mock_alias")
            .expect("plugin provider should be registered");
        assert_eq!(
            capability.status,
            ProviderCapabilityStatus::MissingConfiguration
        );
        assert!(capability
            .required_env
            .contains(&"OPENAI_API_KEY".to_string()));
    }

    #[test]
    fn rejects_unknown_base_adapter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("broken.yaml");
        fs::write(
            &path,
            r#"
schema: earmark.provider_plugin.v1
name: broken
version: 0.1.0
providers:
  - provider: imaginary_http
    adapter: imaginary_base
"#,
        )
        .unwrap();

        let mut registry = default_provider_registry();
        let error = register_provider_plugins_from_dirs(&mut registry, &[dir.path().to_path_buf()])
            .unwrap_err();

        match error {
            ProviderPluginLoadError::Invalid { message, .. } => {
                assert!(message.contains("unknown adapter"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
