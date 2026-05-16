mod resolver;

pub use resolver::resolve_workflow_declaration;

use std::{fs, path::Path};

use earmark_core::{
    parse_yaml, ClassDefinition, ClassStandingRules, CompiledContextTemplate, InstructionPayload,
    Kind, ProviderProfile, StandingPolicy, StandingRegistry, SystemDefinition, VersionRef,
    WorkflowDeclaration, WorkflowDefinition,
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

pub fn load_workflow_definition(
    path: impl AsRef<Path>,
) -> Result<WorkflowDeclaration, DeriveError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;
    parse_yaml(&content).map_err(|e| {
        DeriveError::Validation(format!(
            "failed to load workflow declaration from {}: {}",
            path.display(),
            e
        ))
    })
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
    for (dim_id, tokens) in &rules.allowed_standing {
        earmark_core::DimensionId::parse(dim_id.as_str()).map_err(|e| {
            DeriveError::Validation(format!(
                "invalid dimension '{}' in standing rules: {}",
                dim_id.as_str(),
                e
            ))
        })?;
        for token in tokens {
            earmark_core::TokenId::parse(token.as_str()).map_err(|e| {
                DeriveError::Validation(format!(
                    "invalid token '{}' in standing rules for dimension '{}': {}",
                    token.as_str(),
                    dim_id.as_str(),
                    e
                ))
            })?;
        }
    }
    for (pid, props) in &rules.required_protocols {
        earmark_core::KernelProtocolId::parse(pid.as_str()).map_err(|e| {
            DeriveError::Validation(format!(
                "invalid protocol '{}' in required_protocols: {}",
                pid.as_str(),
                e
            ))
        })?;
        for k in props.keys() {
            if k.trim().is_empty() {
                return Err(DeriveError::Validation(
                    "protocol property key cannot be empty".to_string(),
                ));
            }
        }
    }
    Ok(())
}

/// Validate class standing rules against a registry, checking that all
/// referenced dimensions and tokens exist in the registry.
pub fn validate_class_standing_rules_against_registry(
    rules: &ClassStandingRules,
    registry: &earmark_core::StandingRegistry,
) -> Result<(), DeriveError> {
    for (dim_id, tokens) in &rules.allowed_standing {
        let def = registry.dimensions.get(dim_id).ok_or_else(|| {
            DeriveError::Validation(format!(
                "unknown dimension '{}' in class standing rules",
                dim_id.as_str()
            ))
        })?;
        let valid_tokens: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        for token in tokens {
            if !valid_tokens.contains(&token.as_str()) {
                return Err(DeriveError::Validation(format!(
                    "unknown token '{}' for dimension '{}' in class standing rules",
                    token.as_str(),
                    dim_id.as_str()
                )));
            }
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

fn parse_standing_dim_id(value: &str) -> Result<String, DeriveError> {
    // Try namespaced format first
    if let Ok(_dim) = earmark_core::DimensionId::parse(value) {
        return Ok(value.to_string());
    }
    // Fall back to legacy short names for backward compatibility
    match value {
        "epistemic" | "review" | "process" => Ok(value.to_string()),
        _ => Err(DeriveError::Validation(format!(
            "invalid standing dimension '{}'",
            value
        ))),
    }
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
        let _dim = parse_standing_dim_id(&rule.dimension)?;
        for token in &rule.from {
            if token.trim().is_empty() {
                return Err(DeriveError::Validation(format!(
                    "empty token in transition rule '{}' from list",
                    rule.dimension
                )));
            }
        }
        for token in &rule.to {
            if token.trim().is_empty() {
                return Err(DeriveError::Validation(format!(
                    "empty token in transition rule '{}' to list",
                    rule.dimension
                )));
            }
        }
    }
    for req in &value.operation_requirements {
        for (dimension, token) in &req.required_standing {
            let _dim = parse_standing_dim_id(dimension)?;
            if token.trim().is_empty() {
                return Err(DeriveError::Validation(format!(
                    "empty token in operation requirement for dimension '{}'",
                    dimension
                )));
            }
        }
        for (dimension, tokens) in &req.forbidden_standing {
            let _dim = parse_standing_dim_id(dimension)?;
            for token in tokens {
                if token.trim().is_empty() {
                    return Err(DeriveError::Validation(format!(
                        "empty token in operation requirement forbidden list for dimension '{}'",
                        dimension
                    )));
                }
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

/// Validate standing policy references against a registry, checking that all
/// referenced dimensions and tokens exist in the registry.
pub fn validate_standing_policy_against_registry(
    value: &StandingPolicy,
    registry: &earmark_core::StandingRegistry,
) -> Result<(), DeriveError> {
    fn resolve_dim<'a>(
        d: &str,
        registry: &'a earmark_core::StandingRegistry,
    ) -> Result<&'a earmark_core::StandingDimensionDefinition, DeriveError> {
        // Try namespaced, then legacy short name
        if let Ok(dim_id) = earmark_core::DimensionId::parse(d) {
            registry.dimensions.get(&dim_id).ok_or_else(|| {
                DeriveError::Validation(format!("unknown dimension '{}' in standing policy", d))
            })
        } else {
            let mapped = match d {
                "epistemic" => "kernel:epistemic",
                "review" => "kernel:review",
                "process" => "kernel:process",
                _ => {
                    return Err(DeriveError::Validation(format!(
                        "unknown dimension '{}' in standing policy",
                        d
                    )));
                }
            };
            let dim_id = earmark_core::DimensionId::from_static(mapped);
            registry.dimensions.get(&dim_id).ok_or_else(|| {
                DeriveError::Validation(format!("unknown dimension '{}' in standing policy", d))
            })
        }
    }

    fn validate_tokens(
        dim_id_str: &str,
        tokens: &[String],
        def: &earmark_core::StandingDimensionDefinition,
    ) -> Result<(), DeriveError> {
        let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        for token in tokens {
            if !valid.contains(&token.as_str()) {
                return Err(DeriveError::Validation(format!(
                    "unknown token '{}' for dimension '{}' in standing policy",
                    token, dim_id_str
                )));
            }
        }
        Ok(())
    }

    for rule in &value.transition_rules {
        let def = resolve_dim(&rule.dimension, registry)?;
        validate_tokens(&rule.dimension, &rule.from, def)?;
        validate_tokens(&rule.dimension, &rule.to, def)?;
    }
    for req in &value.operation_requirements {
        for (dim_str, token) in &req.required_standing {
            let def = resolve_dim(dim_str, registry)?;
            let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
            if !valid.contains(&token.as_str()) {
                return Err(DeriveError::Validation(format!(
                    "unknown token '{}' for dimension '{}' in operation requirement",
                    token, dim_str
                )));
            }
        }
        for (dim_str, tokens) in &req.forbidden_standing {
            let def = resolve_dim(dim_str, registry)?;
            let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
            for token in tokens {
                if !valid.contains(&token.as_str()) {
                    return Err(DeriveError::Validation(format!(
                        "unknown token '{}' for dimension '{}' in operation requirement forbidden list",
                        token, dim_str
                    )));
                }
            }
        }
    }
    Ok(())
}

pub fn validate_workflow_definition(value: &WorkflowDeclaration) -> Result<(), DeriveError> {
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
        match op.kind {
            earmark_core::WorkflowOperationKind::CompileContext => {
                if op.compiled_context.is_none() {
                    return Err(DeriveError::Validation(format!(
                        "workflow operation '{}' of kind compile_context requires a compiled_context reference",
                        op.id
                    )));
                }
            }
            earmark_core::WorkflowOperationKind::Transform => {
                if op.instruction.is_none() {
                    return Err(DeriveError::Validation(format!(
                        "workflow operation '{}' of kind transform requires an instruction reference",
                        op.id
                    )));
                }
            }
            earmark_core::WorkflowOperationKind::Review
            | earmark_core::WorkflowOperationKind::Export => {
                // Future packet: validate review/export-specific requirements
            }
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
        // Accept both namespaced and legacy short names
        match earmark_core::DimensionId::parse(dimension) {
            Ok(_) => {}
            Err(_) => match dimension.as_str() {
                "epistemic" | "review" | "process" => {}
                _ => {
                    return Err(DeriveError::Validation(format!(
                        "invalid standing dimension '{}' in compiled context template select",
                        dimension
                    )));
                }
            },
        }
        for token in tokens {
            if token.trim().is_empty() {
                return Err(DeriveError::Validation(format!(
                    "empty token in compiled context template standing filter for dimension '{}'",
                    dimension
                )));
            }
        }
    }
    for relation in &value.select.relations {
        validate_relation_type_token(relation)?;
    }
    Ok(())
}

/// Validate compiled context template standing filters against a registry.
pub fn validate_compiled_context_template_against_registry(
    value: &CompiledContextTemplate,
    registry: &earmark_core::StandingRegistry,
) -> Result<(), DeriveError> {
    fn resolve_dim<'a>(
        d: &str,
        registry: &'a earmark_core::StandingRegistry,
    ) -> Result<&'a earmark_core::StandingDimensionDefinition, DeriveError> {
        if let Ok(dim_id) = earmark_core::DimensionId::parse(d) {
            registry.dimensions.get(&dim_id).ok_or_else(|| {
                DeriveError::Validation(format!(
                    "unknown dimension '{}' in compiled context template",
                    d
                ))
            })
        } else {
            let mapped = match d {
                "epistemic" => "kernel:epistemic",
                "review" => "kernel:review",
                "process" => "kernel:process",
                _ => {
                    return Err(DeriveError::Validation(format!(
                        "unknown dimension '{}' in compiled context template",
                        d
                    )));
                }
            };
            let dim_id = earmark_core::DimensionId::from_static(mapped);
            registry.dimensions.get(&dim_id).ok_or_else(|| {
                DeriveError::Validation(format!(
                    "unknown dimension '{}' in compiled context template",
                    d
                ))
            })
        }
    }

    for (dim_str, tokens) in &value.select.standing {
        let def = resolve_dim(dim_str, registry)?;
        let valid: Vec<&str> = def.tokens.iter().map(|t| t.id.as_str()).collect();
        for token in tokens {
            if !valid.contains(&token.as_str()) {
                return Err(DeriveError::Validation(format!(
                    "unknown token '{}' for dimension '{}' in compiled context template",
                    token, dim_str
                )));
            }
        }
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

    if value.provider == "http_generation" {
        let http = value.http.as_ref().ok_or_else(|| {
            DeriveError::Validation(
                "provider profile with provider 'http_generation' requires an 'http' block"
                    .to_string(),
            )
        })?;

        if http.url_template.trim().is_empty() {
            return Err(DeriveError::Validation(
                "http url_template must be non-empty".to_string(),
            ));
        }
        validate_http_template(&http.url_template)?;
        validate_rendered_url(&http.url_template)?;

        if let Some(method) = &http.method {
            if method != "POST" {
                return Err(DeriveError::Validation(format!(
                    "unsupported http method '{}'; only POST is supported",
                    method
                )));
            }
        }

        match http.auth.kind {
            earmark_core::HttpAuthKind::None => {}
            earmark_core::HttpAuthKind::Header => {
                if http
                    .auth
                    .header_name
                    .as_ref()
                    .is_none_or(|s| s.trim().is_empty())
                {
                    return Err(DeriveError::Validation(
                        "http auth kind 'header' requires a header_name".to_string(),
                    ));
                }
                let env = http.auth.env.as_ref().or(value.auth_env.as_ref()).ok_or_else(|| {
                    DeriveError::Validation(
                        "http auth kind 'header' requires an 'env' variable name or top-level 'auth_env'".to_string(),
                    )
                })?;
                earmark_core::validate_env_var_name(env)
                    .map_err(|e| DeriveError::Validation(e.to_string()))?;
            }
            earmark_core::HttpAuthKind::Bearer => {
                let env = http.auth.env.as_ref().or(value.auth_env.as_ref()).ok_or_else(|| {
                    DeriveError::Validation(
                        "http auth kind 'bearer' requires an 'env' variable name or top-level 'auth_env'".to_string(),
                    )
                })?;
                earmark_core::validate_env_var_name(env)
                    .map_err(|e| DeriveError::Validation(e.to_string()))?;
            }
            earmark_core::HttpAuthKind::QueryParameter => {
                if http
                    .auth
                    .param_name
                    .as_ref()
                    .is_none_or(|s| s.trim().is_empty())
                {
                    return Err(DeriveError::Validation(
                        "http auth kind 'query_parameter' requires a param_name".to_string(),
                    ));
                }
                let env = http.auth.env.as_ref().or(value.auth_env.as_ref()).ok_or_else(|| {
                    DeriveError::Validation(
                        "http auth kind 'query_parameter' requires an 'env' variable name or top-level 'auth_env'".to_string(),
                    )
                })?;
                earmark_core::validate_env_var_name(env)
                    .map_err(|e| DeriveError::Validation(e.to_string()))?;
            }
        }

        if http.response.text_path.trim().is_empty() {
            return Err(DeriveError::Validation(
                "http response text_path must be non-empty".to_string(),
            ));
        }

        if !http.request.body.is_object() && !http.request.body.is_array() {
            return Err(DeriveError::Validation(
                "http request body must be a JSON object or array".to_string(),
            ));
        }
        validate_json_value_templates(&http.request.body)?;
    }

    Ok(())
}

fn validate_http_template(template: &str) -> Result<(), DeriveError> {
    let allowlist = [
        "model",
        "input_text",
        "instruction_text",
        "system_text",
        "context_text",
        "max_output_tokens",
    ];
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let end_rel = rest[start..].find("}}").ok_or_else(|| {
            DeriveError::Validation(format!("malformed template variable in '{}'", template))
        })?;
        let var = &rest[start + 2..start + end_rel];
        if !allowlist.contains(&var.trim()) {
            return Err(DeriveError::Validation(format!(
                "unsupported template variable '{{{{{}}}}}' in '{}'; allowed: {:?}",
                var, template, allowlist
            )));
        }
        rest = &rest[start + end_rel + 2..];
    }
    Ok(())
}

fn validate_rendered_url(template: &str) -> Result<(), DeriveError> {
    let rendered = template.replace("{{model}}", "test-model");
    if !rendered.starts_with("http://") && !rendered.starts_with("https://") {
        return Err(DeriveError::Validation(format!(
            "invalid http url_template '{}'; must start with http:// or https://",
            template
        )));
    }
    Ok(())
}

fn validate_json_value_templates(value: &serde_json::Value) -> Result<(), DeriveError> {
    match value {
        serde_json::Value::String(s) => validate_http_template(s),
        serde_json::Value::Array(arr) => {
            for v in arr {
                validate_json_value_templates(v)?;
            }
            Ok(())
        }
        serde_json::Value::Object(obj) => {
            for v in obj.values() {
                validate_json_value_templates(v)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
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

    // Build the registry from system definitions + built-in defaults and
    // validate it (handles duplicate dims, missing defaults, etc.)
    let registry = earmark_core::StandingRegistry::from_system_definition(value)
        .map_err(|e| DeriveError::Validation(e.to_string()))?;

    // Validate that referenced declarations use known dimensions/tokens.
    // We need to load and re-validate classes, policies, and compiled
    // contexts against the registry.
    validate_references_against_registry::<S>(store, value, &registry)?;

    Ok(())
}

/// After constructing the registry from a system definition, re-validate
/// all referenced class, policy, and compiled context declarations against
/// the registry to catch unknown dimensions and tokens.
fn validate_references_against_registry<S: CanonicalStore>(
    store: &S,
    system: &SystemDefinition,
    registry: &StandingRegistry,
) -> Result<(), DeriveError> {
    for reference in &system.classes {
        let stored = load_stored_ref(store, reference)?;
        if stored.envelope.kind != Kind::Object {
            continue;
        }
        if let Ok(text) = stored.payload.as_utf8() {
            if let Ok(class_def) = parse_yaml::<ClassDefinition>(&text) {
                validate_class_standing_rules_against_registry(
                    &class_def.standing_rules,
                    registry,
                )?;
            }
        }
    }
    for reference in &system.policies {
        let stored = load_stored_ref(store, reference)?;
        if let Ok(text) = stored.payload.as_utf8() {
            if let Ok(policy) = parse_yaml::<StandingPolicy>(&text) {
                validate_standing_policy_against_registry(&policy, registry)?;
            }
        }
    }
    for reference in &system.compiled_contexts {
        let stored = load_stored_ref(store, reference)?;
        if let Ok(text) = stored.payload.as_utf8() {
            if let Ok(ctx) = parse_yaml::<CompiledContextTemplate>(&text) {
                validate_compiled_context_template_against_registry(&ctx, registry)?;
            }
        }
    }
    Ok(())
}

fn load_stored_ref<S: CanonicalStore>(
    store: &S,
    reference: &VersionRef,
) -> Result<earmark_store::StoredObject, DeriveError> {
    let to_read = if reference.version_id.is_latest_sentinel() {
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
            reference.version_id.as_str()
        ))
    })
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
        let to_read = if reference.version_id.is_latest_sentinel() {
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
