use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_core::{ClassDefinition, RelationRule, VersionRef};
use earmark_store::{CanonicalStore, StoredObject};

pub(crate) fn validate_relation_creation<S: CanonicalStore>(
    surface: &RuntimeToolSurface<'_, S>,
    source: &StoredObject,
    target: &StoredObject,
    relation_type: &str,
) -> Result<(), RuntimeToolError> {
    let source_class = load_class_definition_for_object(surface, source)?;
    let target_class = load_class_definition_for_object(surface, target)?;

    validate_relation_rule(&source_class, &target_class, relation_type)
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

fn validate_relation_rule(
    source_class: &ClassDefinition,
    target_class: &ClassDefinition,
    relation_type: &str,
) -> Result<(), RuntimeToolError> {
    if source_class.relation_rules.is_empty() {
        return Err(RuntimeToolError::RelationRuleViolation(format!(
            "class '{}' allows no outgoing relations",
            source_class.name
        )));
    }

    let matching_rules: Vec<&RelationRule> = source_class
        .relation_rules
        .iter()
        .filter(|r| r.relation_type == relation_type)
        .collect();

    if matching_rules.is_empty() {
        return Err(RuntimeToolError::RelationRuleViolation(format!(
            "class '{}' has no rule for relation type '{}'",
            source_class.name, relation_type
        )));
    }

    for rule in matching_rules {
        if rule.target_classes.is_empty() || !rule.target_classes.contains(&target_class.name) {
            continue;
        }

        if direction_allows_outgoing(rule.direction.as_deref())? {
            return Ok(());
        }
    }

    Err(RuntimeToolError::RelationRuleViolation(format!(
        "relation '{}' from '{}' to '{}' is not authorized by class definition",
        relation_type, source_class.name, target_class.name
    )))
}

fn direction_allows_outgoing(direction: Option<&str>) -> Result<bool, RuntimeToolError> {
    match direction.unwrap_or("outgoing") {
        "outgoing" | "bidirectional" => Ok(true),
        "incoming" => Ok(false),
        other => Err(RuntimeToolError::RelationRuleViolation(format!(
            "unknown relation direction: {}",
            other
        ))),
    }
}
