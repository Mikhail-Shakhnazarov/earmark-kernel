use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_core::{ClassDefinition, VersionRef};
use earmark_exec::{
    RelationAuthorizationDecision, RelationAuthorizationReason, RelationAuthorizationResolver,
    RelationEndpointFacts,
};
use earmark_store::{CanonicalStore, StoredObject};

pub(crate) fn authorize_relation_creation<S: CanonicalStore>(
    surface: &RuntimeToolSurface<'_, S>,
    source: &StoredObject,
    target: &StoredObject,
    relation_type: &str,
) -> Result<RelationAuthorizationReason, RuntimeToolError> {
    let source_class_def = load_class_definition_for_object(surface, source)?;
    let target_class_def = load_class_definition_for_object(surface, target)?;

    let source_facts = RelationEndpointFacts {
        id: source.envelope.id.clone(),
        version_id: source.envelope.version_id.clone(),
        kind: source.envelope.kind.clone(),
        class: source.envelope.class.clone(),
    };
    let target_facts = RelationEndpointFacts {
        id: target.envelope.id.clone(),
        version_id: target.envelope.version_id.clone(),
        kind: target.envelope.kind.clone(),
        class: target.envelope.class.clone(),
    };

    let mut declared_classes = std::collections::HashMap::new();
    declared_classes.insert(source_class_def.name.clone(), source_class_def.clone());
    declared_classes.insert(target_class_def.name.clone(), target_class_def.clone());

    let resolver = RelationAuthorizationResolver {
        relation_type,
        source: &source_facts,
        target: &target_facts,
        source_definition: Some(&source_class_def),
        target_definition: Some(&target_class_def),
        creation_mode: Some("declared"), // creation via runtime tool is typically "declared"
        is_trusted_provenance: false,    // runtime tool is user-facing
    };

    match resolver.resolve() {
        RelationAuthorizationDecision::Allowed(reason) => Ok(reason),
        RelationAuthorizationDecision::Blocked(failure) => {
            Err(RuntimeToolError::RelationRuleViolation(format!(
                "relation '{}' from '{}' to '{}' is not authorized: {}",
                relation_type, source_class_def.name, target_class_def.name, failure
            )))
        }
    }
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
