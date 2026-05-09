use std::{fs, path::Path};

use earmark_core::{
    parse_yaml, ClassDefinition, CompiledContextTemplate, InstructionPayload, Kind,
    ProviderProfile, StandingPolicy, SystemDefinition, VersionRef, WorkflowDefinition,
};
use earmark_index::{ActiveSystemRecord, DerivedIndex};
use earmark_store::CanonicalStore;
use thiserror::Error;

pub fn load_class_definition(path: impl AsRef<Path>) -> Result<ClassDefinition, DeriveError> {
    Ok(parse_yaml(&fs::read_to_string(path)?)?)
}

pub fn load_instruction(path: impl AsRef<Path>) -> Result<InstructionPayload, DeriveError> {
    Ok(InstructionPayload::parse_markdown(&fs::read_to_string(
        path,
    )?)?)
}

pub fn load_standing_policy(path: impl AsRef<Path>) -> Result<StandingPolicy, DeriveError> {
    Ok(parse_yaml(&fs::read_to_string(path)?)?)
}

pub fn load_workflow_definition(path: impl AsRef<Path>) -> Result<WorkflowDefinition, DeriveError> {
    Ok(parse_yaml(&fs::read_to_string(path)?)?)
}

pub fn load_compiled_context_template(
    path: impl AsRef<Path>,
) -> Result<CompiledContextTemplate, DeriveError> {
    Ok(parse_yaml(&fs::read_to_string(path)?)?)
}

pub fn load_provider_profile(path: impl AsRef<Path>) -> Result<ProviderProfile, DeriveError> {
    Ok(parse_yaml(&fs::read_to_string(path)?)?)
}

pub fn load_system_definition(path: impl AsRef<Path>) -> Result<SystemDefinition, DeriveError> {
    Ok(parse_yaml(&fs::read_to_string(path)?)?)
}

pub fn validate_class_definition(value: &ClassDefinition) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    if value.version.trim().is_empty() || value.kind.trim().is_empty() {
        return Err(DeriveError::Validation(
            "class definition requires version and kind".to_string(),
        ));
    }
    for rule in &value.relation_rules {
        validate_relation_type_token(&rule.relation_type)?;
        if rule.target_classes.is_empty() {
            return Err(DeriveError::Validation(
                "relation rule requires at least one target class".to_string(),
            ));
        }
        for class in &rule.target_classes {
            earmark_core::validate_class_name(class).map_err(|e| {
                DeriveError::Validation(format!("invalid relation target class: {}", e))
            })?;
        }
    }
    Ok(())
}

pub fn validate_instruction(value: &InstructionPayload) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    if value.body.as_str().trim().is_empty() {
        return Err(DeriveError::Validation(
            "instruction requires a body".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_standing_policy(value: &StandingPolicy) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    for rule in &value.transition_rules {
        let dimension = earmark_core::StandingDimension::parse(&rule.dimension)
            .map_err(|e| DeriveError::Validation(e.to_string()))?;
        for token in &rule.from {
            validate_standing_token_for_dimension(dimension, token)?;
        }
        for token in &rule.to {
            validate_standing_token_for_dimension(dimension, token)?;
        }
    }
    for req in &value.operation_requirements {
        for (dimension, token) in &req.minimums {
            let dimension = earmark_core::StandingDimension::parse(dimension)
                .map_err(|e| DeriveError::Validation(e.to_string()))?;
            validate_standing_token_for_dimension(dimension, token)?;
        }
        for (dimension, tokens) in &req.forbidden {
            let dimension = earmark_core::StandingDimension::parse(dimension)
                .map_err(|e| DeriveError::Validation(e.to_string()))?;
            for token in tokens {
                validate_standing_token_for_dimension(dimension, token)?;
            }
        }
    }
    Ok(())
}

pub fn validate_workflow_definition(value: &WorkflowDefinition) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    let mut ids = std::collections::BTreeSet::new();
    for op in &value.operations {
        validate_workflow_token(&op.id, "operation id")?;
        if !ids.insert(op.id.as_str()) {
            return Err(DeriveError::Validation(format!(
                "duplicate workflow operation id '{}'",
                op.id
            )));
        }
        if op.kind == "transform" && op.instruction.is_none() {
            return Err(DeriveError::Validation(format!(
                "workflow operation '{}' of kind transform requires an instruction reference",
                op.id
            )));
        }
    }
    let mut guard_ids = std::collections::BTreeSet::new();
    for guard in &value.guards {
        validate_workflow_token(&guard.id, "guard id")?;
        if !guard_ids.insert(guard.id.as_str()) {
            return Err(DeriveError::Validation(format!(
                "duplicate workflow guard id '{}'",
                guard.id
            )));
        }
    }
    for edge in &value.edges {
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            return Err(DeriveError::Validation(format!(
                "workflow edge references unknown operations: {} -> {}",
                edge.from, edge.to
            )));
        }
        if let Some(condition) = &edge.condition {
            if !guard_ids.contains(condition.as_str()) {
                return Err(DeriveError::Validation(format!(
                    "workflow edge references unknown guard '{}'",
                    condition
                )));
            }
        }
    }
    Ok(())
}

pub fn validate_compiled_context_template(
    value: &CompiledContextTemplate,
) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    if value.render.mode.trim().is_empty() {
        return Err(DeriveError::Validation(
            "compiled context template requires a render mode".to_string(),
        ));
    }
    for dimension in value.select.standing.keys() {
        earmark_core::StandingDimension::parse(dimension)
            .map_err(|e| DeriveError::Validation(e.to_string()))?;
    }
    for relation in &value.select.relations {
        validate_relation_type_token(relation)?;
    }
    Ok(())
}

fn validate_workflow_token(value: &str, field: &str) -> Result<(), DeriveError> {
    if value.is_empty() || value.len() > 64 {
        return Err(DeriveError::Validation(format!(
            "invalid {}: expected 1..=64 characters",
            field
        )));
    }
    let mut chars = value.chars();
    let first = chars.next().expect("checked non-empty");
    if !first.is_ascii_lowercase() {
        return Err(DeriveError::Validation(format!(
            "invalid {} '{}': must start with lowercase letter",
            field, value
        )));
    }
    if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(DeriveError::Validation(format!(
            "invalid {} '{}': only lowercase letters, digits, and underscores are allowed",
            field, value
        )));
    }
    Ok(())
}

fn validate_relation_type_token(value: &str) -> Result<(), DeriveError> {
    validate_workflow_token(value, "relation type")
}

fn validate_standing_token_for_dimension(
    dimension: earmark_core::StandingDimension,
    token: &str,
) -> Result<(), DeriveError> {
    let valid = match dimension {
        earmark_core::StandingDimension::Epistemic => {
            matches!(
                token,
                "unresolved" | "working" | "supported" | "contested" | "superseded"
            )
        }
        earmark_core::StandingDimension::Review => {
            matches!(token, "unreviewed" | "pending" | "accepted" | "rejected")
        }
        earmark_core::StandingDimension::Process => {
            matches!(token, "active" | "blocked" | "completed" | "archived")
        }
    };
    if valid {
        Ok(())
    } else {
        Err(DeriveError::Validation(format!(
            "invalid standing token '{}' for dimension '{}'",
            token,
            dimension.as_str()
        )))
    }
}

pub fn validate_provider_profile(value: &ProviderProfile) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    if value.provider.trim().is_empty() || value.model.trim().is_empty() {
        return Err(DeriveError::Validation(
            "provider profile requires provider and model".to_string(),
        ));
    }
    if let Some(auth_env) = &value.auth_env {
        earmark_core::validate_env_var_name(auth_env)
            .map_err(|e| DeriveError::Validation(e.to_string()))?;
    }
    if let Some(endpoint_env) = &value.endpoint_env {
        earmark_core::validate_env_var_name(endpoint_env)
            .map_err(|e| DeriveError::Validation(e.to_string()))?;
    }
    Ok(())
}

pub fn validate_system_definition<S: CanonicalStore>(
    store: &S,
    value: &SystemDefinition,
) -> Result<(), DeriveError> {
    earmark_core::SymbolicName::parse(value.system_id.clone())
        .map_err(|e| DeriveError::Validation(format!("invalid system_id: {}", e)))?;
    validate_namespace(&value.namespace)
        .map_err(|e| DeriveError::Validation(format!("invalid namespace: {}", e)))?;

    validate_system_reference_group(
        store,
        "class",
        &value.classes,
        Kind::Object,
        Some("class_definition"),
        |text| parse_yaml::<ClassDefinition>(text).map(|_| ()),
    )?;
    validate_system_reference_group(
        store,
        "instruction",
        &value.instructions,
        Kind::Instruction,
        None,
        |text| InstructionPayload::parse_markdown(text).map(|_| ()),
    )?;
    validate_system_reference_group(
        store,
        "policy",
        &value.policies,
        Kind::Policy,
        None,
        |text| parse_yaml::<StandingPolicy>(text).map(|_| ()),
    )?;
    validate_system_reference_group(
        store,
        "workflow",
        &value.workflows,
        Kind::Workflow,
        None,
        |text| parse_yaml::<WorkflowDefinition>(text).map(|_| ()),
    )?;
    validate_system_reference_group(
        store,
        "compiled_context",
        &value.compiled_contexts,
        Kind::CompiledContextTemplate,
        None,
        |text| parse_yaml::<CompiledContextTemplate>(text).map(|_| ()),
    )?;
    validate_system_reference_group(
        store,
        "provider_profile",
        &value.provider_profiles,
        Kind::ProviderProfile,
        None,
        |text| parse_yaml::<ProviderProfile>(text).map(|_| ()),
    )?;
    validate_system_reference_group(
        store,
        "default_compiled_context",
        &value
            .default_compiled_context
            .clone()
            .into_iter()
            .collect::<Vec<_>>(),
        Kind::CompiledContextTemplate,
        None,
        |text| parse_yaml::<CompiledContextTemplate>(text).map(|_| ()),
    )?;
    validate_system_reference_group(
        store,
        "default_provider_profile",
        &value
            .default_provider_profile
            .clone()
            .into_iter()
            .collect::<Vec<_>>(),
        Kind::ProviderProfile,
        None,
        |text| parse_yaml::<ProviderProfile>(text).map(|_| ()),
    )?;
    Ok(())
}

fn validate_system_reference_group<S: CanonicalStore, F>(
    store: &S,
    role: &str,
    refs: &[VersionRef],
    expected_kind: Kind,
    expected_class: Option<&str>,
    decode: F,
) -> Result<(), DeriveError>
where
    F: Fn(&str) -> Result<(), earmark_core::CoreError>,
{
    for reference in refs {
        let to_read = if reference.version_id.as_str() == "ver_00000000000000000000000000000000" {
            store
                .read_head_ref(&reference.id)
                .map_err(DeriveError::Store)?
                .ok_or_else(|| {
                    DeriveError::Validation(format!("object head not found for {}", reference.id))
                })?
        } else {
            reference.clone()
        };

        let stored = store.read_version(&to_read).map_err(|_| {
            DeriveError::Validation(format!(
                "missing referenced version {} for {}",
                reference.version_id.as_str(),
                role
            ))
        })?;
        if stored.envelope.kind != expected_kind {
            return Err(DeriveError::Validation(format!(
                "{} reference {} has wrong envelope kind '{}', expected '{}'",
                role,
                reference.id.as_str(),
                stored.envelope.kind.as_str(),
                expected_kind.as_str()
            )));
        }
        if let Some(expected_class) = expected_class {
            if stored.envelope.class.as_deref() != Some(expected_class) {
                return Err(DeriveError::Validation(format!(
                    "{} reference {} has wrong class marker '{:?}', expected '{}'",
                    role,
                    reference.id.as_str(),
                    stored.envelope.class,
                    expected_class
                )));
            }
        }
        let text = stored.payload.as_utf8().map_err(|e| {
            DeriveError::Validation(format!("{} reference is not UTF-8 decodable: {}", role, e))
        })?;
        decode(&text).map_err(|e| {
            DeriveError::Validation(format!(
                "{} reference {} is wrong kind or malformed: {}",
                role,
                reference.id.as_str(),
                e
            ))
        })?;
    }
    Ok(())
}

fn validate_namespace(namespace: &str) -> Result<(), String> {
    if namespace.is_empty() || namespace.len() > 128 {
        return Err("namespace length must be between 1 and 128 characters".to_string());
    }
    if namespace.starts_with('/') || namespace.ends_with('/') {
        return Err("namespace cannot start or end with '/'".to_string());
    }
    if namespace.starts_with('.') || namespace.ends_with('.') {
        return Err("namespace cannot start or end with '.'".to_string());
    }
    if namespace.contains("//") || namespace.contains("..") {
        return Err("namespace cannot contain empty path/dot segments".to_string());
    }
    if !namespace.chars().all(|c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.' || c == '/'
    }) {
        return Err(
            "namespace can only contain lowercase letters, digits, '-', '_', '.', '/'".to_string(),
        );
    }
    Ok(())
}

pub fn activate_system_definition<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    system_id: &str,
) -> Result<ActiveSystemRecord, DeriveError> {
    let found = index.find_system_definition(system_id)?.ok_or_else(|| {
        DeriveError::NotFound(format!("system definition not found: {}", system_id))
    })?;
    let reference = VersionRef::new(
        earmark_core::ObjectId::parse(found.0)?,
        earmark_core::VersionId::parse(found.1)?,
    );
    let loaded = store.read_version(&reference)?;
    let system: SystemDefinition = parse_yaml(&loaded.payload.as_utf8()?)?;
    Ok(index.activate_system(&system.namespace, &system.system_id, &reference)?)
}

#[derive(Debug, Error)]
pub enum DeriveError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("index error: {0}")]
    Index(#[from] earmark_index::IndexError),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
}
