use std::{collections::BTreeMap, fs, path::Path};

use earmark_core::{
    parse_yaml, ClassDefinition, CompiledContextTemplate, HeaderValue, InstructionPayload, Kind,
    ProviderProfile, Standing, StandingPolicy, SystemDefinition, VersionRef, WorkflowDefinition,
};
use earmark_index::{ActiveSystemRecord, DerivedIndex};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
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
    if value.name.trim().is_empty()
        || value.version.trim().is_empty()
        || value.kind.trim().is_empty()
    {
        return Err(DeriveError::Validation(
            "class definition requires name, version, and kind".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_instruction(value: &InstructionPayload) -> Result<(), DeriveError> {
    if value.name.trim().is_empty() || value.body.0.trim().is_empty() {
        return Err(DeriveError::Validation(
            "instruction requires name and body".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_standing_policy(value: &StandingPolicy) -> Result<(), DeriveError> {
    if value.name.trim().is_empty() {
        return Err(DeriveError::Validation(
            "standing policy requires a name".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_workflow_definition(value: &WorkflowDefinition) -> Result<(), DeriveError> {
    if value.name.trim().is_empty() {
        return Err(DeriveError::Validation(
            "workflow requires a name".to_string(),
        ));
    }
    let ids = value
        .operations
        .iter()
        .map(|op| op.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for edge in &value.edges {
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            return Err(DeriveError::Validation(format!(
                "workflow edge references unknown operations: {} -> {}",
                edge.from, edge.to
            )));
        }
    }
    Ok(())
}

pub fn validate_compiled_context_template(
    value: &CompiledContextTemplate,
) -> Result<(), DeriveError> {
    if value.name.trim().is_empty() || value.render.mode.trim().is_empty() {
        return Err(DeriveError::Validation(
            "compiled context template requires name and render mode".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_provider_profile(value: &ProviderProfile) -> Result<(), DeriveError> {
    if value.provider.trim().is_empty() || value.model.trim().is_empty() {
        return Err(DeriveError::Validation(
            "provider profile requires provider and model".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_system_definition<S: CanonicalStore>(
    store: &S,
    value: &SystemDefinition,
) -> Result<(), DeriveError> {
    if value.system_id.trim().is_empty() || value.namespace.trim().is_empty() {
        return Err(DeriveError::Validation(
            "system definition requires system_id and namespace".to_string(),
        ));
    }

    for reference in value
        .classes
        .iter()
        .chain(value.instructions.iter())
        .chain(value.policies.iter())
        .chain(value.workflows.iter())
        .chain(value.compiled_contexts.iter())
        .chain(value.provider_profiles.iter())
        .chain(value.default_compiled_context.iter())
        .chain(value.default_provider_profile.iter())
    {
        let to_read = if reference.version_id.0 == "latest" {
            store
                .read_head_ref(&reference.id)
                .map_err(DeriveError::Store)?
                .ok_or_else(|| {
                    DeriveError::Validation(format!("object head not found for {}", reference.id))
                })?
        } else {
            reference.clone()
        };

        store.read_version(&to_read).map_err(|_| {
            DeriveError::Validation(format!(
                "missing referenced version {}",
                reference.version_id.0
            ))
        })?;
    }
    Ok(())
}

pub fn register_system_definition_file<S: CanonicalStore>(
    store: &S,
    path: impl AsRef<Path>,
) -> Result<VersionRef, DeriveError> {
    let system = load_system_definition(path)?;
    let payload = StoredPayload::from_yaml(earmark_core::to_yaml(&system)?);
    let object = StoredObject::new(
        Kind::SystemDefinition,
        Some("system_definition".to_string()),
        Standing::default(),
        earmark_core::Provenance::direct_input("operator"),
        BTreeMap::from([(
            "title".to_string(),
            HeaderValue::String(system.title.clone()),
        )]),
        payload,
        vec![],
    );
    Ok(store.write_object(&object)?)
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
        earmark_core::ObjectId(found.0),
        earmark_core::VersionId(found.1),
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
