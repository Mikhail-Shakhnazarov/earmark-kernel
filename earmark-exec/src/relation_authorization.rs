use crate::error::ExecError;
use crate::relation_logic::{
    RelationAuthorizationDecision, RelationAuthorizationReason, RelationAuthorizationResolver,
    RelationEndpointFacts,
};
use earmark_core::{ObjectRef, Provenance, RelationCreationMode, VersionRef};
use earmark_index::DerivedIndex;
use earmark_store::CanonicalStore;

/// Authorizes a relation creation request against the system class rules.
pub fn authorize_relation_creation<S: CanonicalStore>(
    store: &S,
    index: &DerivedIndex,
    relation_type: &str,
    source: &ObjectRef,
    target: &ObjectRef,
    mode: RelationCreationMode,
    provenance: &Provenance,
) -> Result<RelationAuthorizationReason, ExecError> {
    // 1. Load source and target objects from the canonical store
    let source_ref = VersionRef::new(source.id.clone(), source.version_id.clone());
    let target_ref = VersionRef::new(target.id.clone(), target.version_id.clone());

    let source_stored = store.read_version(&source_ref).map_err(|e| {
        ExecError::IncompleteExecution(format!("failed to load source endpoint: {}", e))
    })?;
    let target_stored = store.read_version(&target_ref).map_err(|e| {
        ExecError::IncompleteExecution(format!("failed to load target endpoint: {}", e))
    })?;

    // 2. Extract endpoint facts: object id, version id, kind, class
    let source_facts = RelationEndpointFacts {
        id: source_stored.envelope.id.clone(),
        version_id: source_stored.envelope.version_id.clone(),
        kind: source_stored.envelope.kind.clone(),
        class: source_stored.envelope.class.clone(),
    };
    let target_facts = RelationEndpointFacts {
        id: target_stored.envelope.id.clone(),
        version_id: target_stored.envelope.version_id.clone(),
        kind: target_stored.envelope.kind.clone(),
        class: target_stored.envelope.class.clone(),
    };

    // 3. Load class definitions for source and target classes through the derived index
    let source_definition = if let Some(class_name) = &source_facts.class {
        match index.resolve_class_definition_symbolic_latest(class_name) {
            Ok(Some(resolved_ref)) => Some(
                crate::resolution::load_class_definition(store, index, &resolved_ref).map_err(
                    |e| {
                        ExecError::IncompleteExecution(format!(
                            "failed to load source class definition '{}': {}",
                            class_name, e
                        ))
                    },
                )?,
            ),
            Ok(None) => None,
            Err(e) => {
                return Err(ExecError::IncompleteExecution(format!(
                    "failed to resolve source class definition '{}': {}",
                    class_name, e
                )));
            }
        }
    } else {
        None
    };

    let target_definition = if let Some(class_name) = &target_facts.class {
        match index.resolve_class_definition_symbolic_latest(class_name) {
            Ok(Some(resolved_ref)) => Some(
                crate::resolution::load_class_definition(store, index, &resolved_ref).map_err(
                    |e| {
                        ExecError::IncompleteExecution(format!(
                            "failed to load target class definition '{}': {}",
                            class_name, e
                        ))
                    },
                )?,
            ),
            Ok(None) => None,
            Err(e) => {
                return Err(ExecError::IncompleteExecution(format!(
                    "failed to resolve target class definition '{}': {}",
                    class_name, e
                )));
            }
        }
    } else {
        None
    };

    // 4. Determine creation mode string
    let mode_str = match mode {
        RelationCreationMode::Declared => "declared",
        RelationCreationMode::PrivilegedSystem => "privileged_system",
    };

    // 5. Determine trusted provenance from configured trusted actors or explicit privileged system path
    let is_trusted_provenance = earmark_governance::is_trusted_actor(&provenance.actor);

    // 6. Call RelationAuthorizationResolver
    let resolver = RelationAuthorizationResolver {
        relation_type,
        source: &source_facts,
        target: &target_facts,
        source_definition: source_definition.as_ref(),
        target_definition: target_definition.as_ref(),
        creation_mode: Some(mode_str),
        is_trusted_provenance,
    };

    // 7. Return either an authorization reason or a typed execution error
    match resolver.resolve() {
        RelationAuthorizationDecision::Allowed(reason) => Ok(reason),
        RelationAuthorizationDecision::Blocked(failure) => {
            Err(ExecError::InvalidRelationMode(failure.to_string()))
        }
    }
}
