use crate::{StoredObject, StoreError};
use earmark_core::Kind;

pub fn is_sensitive_kind(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::SystemDefinition | Kind::Policy | Kind::ProviderProfile
    )
}

pub fn check_write_authorized(
    object: &StoredObject,
    trusted_actors: &[String],
) -> Result<(), StoreError> {
    if !is_sensitive_kind(&object.envelope.kind) {
        return Ok(());
    }
    if trusted_actors.is_empty() {
        return Ok(());
    }
    let actor = &object.envelope.provenance.actor;
    if trusted_actors.iter().any(|a| a == actor) {
        Ok(())
    } else {
        Err(StoreError::Unauthorized(format!(
            "actor '{actor}' is not authorized to register '{}' declarations",
            object.envelope.kind.as_str()
        )))
    }
}
