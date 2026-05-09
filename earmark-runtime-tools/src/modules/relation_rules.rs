use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_core::{ClassDefinition, RelationRule, VersionRef};
use earmark_store::{CanonicalStore, StoredObject};

pub(crate) struct RelationAuthorization {
    pub endpoint: AuthorizingEndpoint,
    pub class_name: String,
    #[allow(dead_code)]
    pub relation_type: String,
    pub direction: String,
    pub authorizing_endpoint: String,
}

pub(crate) enum AuthorizingEndpoint {
    Source,
    Target,
}

pub(crate) fn authorize_relation_creation<S: CanonicalStore>(
    surface: &RuntimeToolSurface<'_, S>,
    source: &StoredObject,
    target: &StoredObject,
    relation_type: &str,
) -> Result<RelationAuthorization, RuntimeToolError> {
    let source_class = load_class_definition_for_object(surface, source)?;
    let target_class = load_class_definition_for_object(surface, target)?;

    // 1. Try source-side authorization
    for rule in &source_class.relation_rules {
        if source_rule_authorizes(rule, &target_class.name, relation_type)? {
            return Ok(RelationAuthorization {
                endpoint: AuthorizingEndpoint::Source,
                class_name: source_class.name.clone(),
                relation_type: relation_type.to_string(),
                direction: normalized_direction(rule)?.to_string(),
                authorizing_endpoint: normalized_authorizing_endpoint(rule)?.to_string(),
            });
        }
    }

    // 2. Try target-side authorization
    for rule in &target_class.relation_rules {
        if target_rule_authorizes(rule, &source_class.name, relation_type)? {
            return Ok(RelationAuthorization {
                endpoint: AuthorizingEndpoint::Target,
                class_name: target_class.name.clone(),
                relation_type: relation_type.to_string(),
                direction: normalized_direction(rule)?.to_string(),
                authorizing_endpoint: normalized_authorizing_endpoint(rule)?.to_string(),
            });
        }
    }

    Err(RuntimeToolError::RelationRuleViolation(format!(
        "relation '{}' from '{}' to '{}' is not authorized by class definition",
        relation_type, source_class.name, target_class.name
    )))
}

fn load_class_definition_for_object<S: CanonicalStore>(
    surface: &RuntimeToolSurface<'_, S>,
    object: &StoredObject,
) -> Result<ClassDefinition, RuntimeToolError> {
    let class_name = object.envelope.class.as_deref().ok_or_else(|| {
        RuntimeToolError::RelationRuleViolation("object has no class".to_string())
    })?;

    let (class_id, version_id) = surface
        .index
        .find_class_definition(class_name)?
        .ok_or_else(|| RuntimeToolError::MissingClassDefinition(class_name.to_string()))?;

    let class_ref = VersionRef::new(
        earmark_core::ObjectId::parse(class_id)?,
        earmark_core::VersionId::parse(version_id)?,
    );

    let stored = surface.store.read_version(&class_ref)?;
    let text = stored.payload.as_utf8()?;

    Ok(earmark_core::parse_yaml(&text)?)
}

fn normalized_direction(rule: &RelationRule) -> Result<&str, RuntimeToolError> {
    let d = rule.direction.as_deref().unwrap_or("outgoing");
    match d {
        "outgoing" | "incoming" | "bidirectional" => Ok(d),
        other => Err(RuntimeToolError::RelationRuleViolation(format!(
            "unknown relation direction: {}",
            other
        ))),
    }
}

fn normalized_authorizing_endpoint(rule: &RelationRule) -> Result<&str, RuntimeToolError> {
    let a = rule.authorizing_endpoint.as_deref().unwrap_or("source");
    match a {
        "source" | "target" | "either_endpoint" => Ok(a),
        other => Err(RuntimeToolError::RelationRuleViolation(format!(
            "unknown authorizing endpoint: {}",
            other
        ))),
    }
}


fn source_rule_authorizes(
    rule: &RelationRule,
    target_class_name: &str,
    relation_type: &str,
) -> Result<bool, RuntimeToolError> {
    if rule.relation_type != relation_type {
        return Ok(false);
    }

    if !rule.counterparty_classes.contains(&target_class_name.to_string()) {
        return Ok(false);
    }

    let direction = normalized_direction(rule)?;
    let auth = normalized_authorizing_endpoint(rule)?;

    match (direction, auth) {
        ("outgoing", "source") | ("outgoing", "either_endpoint") => Ok(true),
        ("bidirectional", "source") | ("bidirectional", "either_endpoint") => Ok(true),
        ("incoming", "target") | ("incoming", "either_endpoint") => Ok(false), // matches rel type but source is not source
        ("bidirectional", "target") => Ok(false), // target auth only
        ("outgoing", "target") | ("incoming", "source") => {
            Err(RuntimeToolError::RelationRuleViolation(format!(
                "malformed matching rule: direction {} with authorizing_endpoint {}",
                direction, auth
            )))
        }
        _ => Ok(false),
    }
}

fn target_rule_authorizes(
    rule: &RelationRule,
    source_class_name: &str,
    relation_type: &str,
) -> Result<bool, RuntimeToolError> {
    if rule.relation_type != relation_type {
        return Ok(false);
    }

    if !rule.counterparty_classes.contains(&source_class_name.to_string()) {
        return Ok(false);
    }

    let direction = normalized_direction(rule)?;
    let auth = normalized_authorizing_endpoint(rule)?;

    match (direction, auth) {
        ("incoming", "target") | ("incoming", "either_endpoint") => Ok(true),
        ("bidirectional", "target") | ("bidirectional", "either_endpoint") => Ok(true),
        ("outgoing", "source") | ("outgoing", "either_endpoint") => Ok(false), // matches rel type but target is not source
        ("bidirectional", "source") => Ok(false), // source auth only
        ("outgoing", "target") | ("incoming", "source") => {
            Err(RuntimeToolError::RelationRuleViolation(format!(
                "malformed matching rule: direction {} with authorizing_endpoint {}",
                direction, auth
            )))
        }
        _ => Ok(false),
    }
}
