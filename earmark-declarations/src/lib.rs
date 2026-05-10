use std::{fs, path::Path};

use earmark_core::{
    parse_yaml, ClassDefinition, ClassStandingRules, CompiledContextTemplate, InstructionPayload,
    Kind, ProviderProfile, StandingPolicy, SystemDefinition, VersionRef, WorkflowDefinition,
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
    if value.version.trim().is_empty() {
        return Err(DeriveError::Validation(
            "class definition requires non-empty version".to_string(),
        ));
    }
    match value.kind.as_str() {
        "object" | "relation" => {}
        _ => {
            return Err(DeriveError::Validation(format!(
                "class definition has invalid kind '{}': expected 'object' or 'relation'",
                value.kind
            )));
        }
    }
    validate_standing_rules(&value.standing_rules)?;
    for rule in &value.relation_rules {
        validate_relation_type_token(&rule.relation_type)?;
        if rule.counterparty_classes.is_empty() {
            return Err(DeriveError::Validation(
                "relation rule requires at least one counterparty class".to_string(),
            ));
        }
        for class in &rule.counterparty_classes {
            earmark_core::validate_class_name(class).map_err(|e| {
                DeriveError::Validation(format!("invalid relation counterparty class: {}", e))
            })?;
        }
        if let Some(direction) = &rule.direction {
            match direction.as_str() {
                "outgoing" | "incoming" | "bidirectional" => {}
                _ => {
                    return Err(DeriveError::Validation(
                        "relation rule has invalid direction: expected outgoing, incoming, or bidirectional".to_string(),
                    ));
                }
            }
        }
        if let Some(endpoint) = &rule.authorizing_endpoint {
            match endpoint.as_str() {
                "source" | "target" | "either_endpoint" => {}
                _ => {
                    return Err(DeriveError::Validation(
                        "relation rule has invalid authorizing_endpoint: expected source, target, or either_endpoint".to_string(),
                    ));
                }
            }
        }

        // Consistency checks
        let direction = rule.direction.as_deref().unwrap_or("outgoing");
        let authorizing_endpoint = rule.authorizing_endpoint.as_deref().unwrap_or("source");

        match (direction, authorizing_endpoint) {
            ("outgoing", "target") => {
                return Err(DeriveError::Validation(
                    "relation rule has dead combination: outgoing direction with target authorization".to_string(),
                ));
            }
            ("incoming", "source") => {
                return Err(DeriveError::Validation(
                    "relation rule has dead combination: incoming direction with source authorization".to_string(),
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_standing_rules(rules: &ClassStandingRules) -> Result<(), DeriveError> {
    let valid_epistemic = [
        "unresolved",
        "working",
        "supported",
        "contested",
        "superseded",
    ];
    for token in &rules.allowed_epistemic {
        let s = token.as_str();
        if !valid_epistemic.contains(&s) {
            return Err(DeriveError::Validation(format!(
                "invalid epistemic standing token '{}'",
                s
            )));
        }
    }
    let valid_review = ["unreviewed", "pending", "accepted", "rejected"];
    for token in &rules.allowed_review {
        let s = token.as_str();
        if !valid_review.contains(&s) {
            return Err(DeriveError::Validation(format!(
                "invalid review standing token '{}'",
                s
            )));
        }
    }
    let valid_process = ["active", "blocked", "completed", "archived"];
    for token in &rules.allowed_process {
        let s = token.as_str();
        if !valid_process.contains(&s) {
            return Err(DeriveError::Validation(format!(
                "invalid process standing token '{}'",
                s
            )));
        }
    }
    Ok(())
}

pub fn validate_instruction(value: &InstructionPayload) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    if value.version.trim().is_empty() {
        return Err(DeriveError::Validation(
            "instruction requires non-empty version".to_string(),
        ));
    }
    if value.purpose.trim().is_empty() {
        return Err(DeriveError::Validation(
            "instruction requires a purpose".to_string(),
        ));
    }
    if value.body.as_str().trim().is_empty() {
        return Err(DeriveError::Validation(
            "instruction requires a body".to_string(),
        ));
    }
    for class in &value.input_classes {
        earmark_core::validate_class_name(class).map_err(|e| {
            DeriveError::Validation(format!("instruction has invalid input class token: {}", e))
        })?;
    }
    for class in &value.output_classes {
        earmark_core::validate_class_name(class).map_err(|e| {
            DeriveError::Validation(format!("instruction has invalid output class token: {}", e))
        })?;
    }
    Ok(())
}

pub fn validate_standing_policy(value: &StandingPolicy) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    if value.version.trim().is_empty() {
        return Err(DeriveError::Validation(
            "standing policy requires non-empty version".to_string(),
        ));
    }
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
    for escalation in &value.escalations {
        if escalation.trigger.trim().is_empty() {
            return Err(DeriveError::Validation(
                "escalation rule must have non-empty trigger".to_string(),
            ));
        }
        if escalation.message.trim().is_empty() {
            return Err(DeriveError::Validation(
                "escalation rule must have non-empty message".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn validate_workflow_definition(value: &WorkflowDefinition) -> Result<(), DeriveError> {
    earmark_core::validate_class_name(&value.name)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;
    let valid_kinds = ["compile_context", "transform", "nop"];
    let mut ids = std::collections::BTreeSet::new();
    for op in &value.operations {
        validate_workflow_token(&op.id, "operation id")?;
        if !valid_kinds.contains(&op.kind.as_str()) {
            return Err(DeriveError::Validation(format!(
                "workflow operation '{}' has invalid kind '{}': expected compile_context, transform, or nop",
                op.id, op.kind
            )));
        }
        if !ids.insert(op.id.as_str()) {
            return Err(DeriveError::Validation(format!(
                "duplicate workflow operation id '{}'",
                op.id
            )));
        }
        if op.kind == "compile_context" && op.compiled_context.is_none() {
            return Err(DeriveError::Validation(format!(
                "workflow operation '{}' of kind compile_context requires a compiled_context reference",
                op.id
            )));
        }
        if op.kind == "transform" && op.instruction.is_none() {
            return Err(DeriveError::Validation(format!(
                "workflow operation '{}' of kind transform requires an instruction reference",
                op.id
            )));
        }
        if op.kind == "transform" && op.output_contracts.len() > 1 {
            return Err(DeriveError::Validation(format!(
                "multi-output transform operations are not implemented; declare one output contract for operation '{}'",
                op.id
            )));
        }
        for class in &op.input_contracts {
            earmark_core::validate_class_name(class).map_err(|e| {
                DeriveError::Validation(format!(
                    "workflow operation '{}' has invalid input contract class: {}",
                    op.id, e
                ))
            })?;
        }
        for class in &op.output_contracts {
            earmark_core::validate_class_name(class).map_err(|e| {
                DeriveError::Validation(format!(
                    "workflow operation '{}' has invalid output contract class: {}",
                    op.id, e
                ))
            })?;
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
    if value.version.trim().is_empty() {
        return Err(DeriveError::Validation(
            "compiled context template requires non-empty version".to_string(),
        ));
    }
    for class in &value.select.classes {
        earmark_core::validate_class_name(class).map_err(|e| {
            DeriveError::Validation(format!(
                "compiled context template has invalid selected class token: {}",
                e
            ))
        })?;
    }
    if value.render.mode.trim().is_empty() {
        return Err(DeriveError::Validation(
            "compiled context template requires a render mode".to_string(),
        ));
    }
    for (dimension, tokens) in &value.select.standing {
        let dim = earmark_core::StandingDimension::parse(dimension)
            .map_err(|e| DeriveError::Validation(e.to_string()))?;
        for token in tokens {
            validate_standing_token_for_dimension(dim, token)?;
        }
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
    if value.version.trim().is_empty() {
        return Err(DeriveError::Validation(
            "provider profile requires non-empty version".to_string(),
        ));
    }
    if value.provider.trim().is_empty() || value.model.trim().is_empty() {
        return Err(DeriveError::Validation(
            "provider profile requires provider and model".to_string(),
        ));
    }
    if value.response_contract.format.trim().is_empty() {
        return Err(DeriveError::Validation(
            "provider profile response contract format must be non-empty".to_string(),
        ));
    }
    if let Some(max_cost) = value.budget.max_cost_usd {
        if max_cost.is_sign_negative() {
            return Err(DeriveError::Validation(
                "provider profile max_cost_usd must be non-negative".to_string(),
            ));
        }
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

    earmark_core::validate_title(&value.title)
        .map_err(|e| DeriveError::Validation(format!("invalid system title: {}", e)))?;

    validate_runtime_profile(&value.runtime_profile)?;

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

fn validate_runtime_profile(profile: &earmark_core::RuntimeProfile) -> Result<(), DeriveError> {
    if profile.execution_surface.trim().is_empty() {
        return Err(DeriveError::Validation(
            "runtime_profile execution_surface must be non-empty".to_string(),
        ));
    }
    if profile.machine_output_default.trim().is_empty() {
        return Err(DeriveError::Validation(
            "runtime_profile machine_output_default must be non-empty".to_string(),
        ));
    }
    if profile.work_surface_mode.trim().is_empty() {
        return Err(DeriveError::Validation(
            "runtime_profile work_surface_mode must be non-empty".to_string(),
        ));
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
