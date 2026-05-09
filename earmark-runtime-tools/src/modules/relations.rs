use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_core::{
    ObjectId, ObjectRef, Provenance, RelationCreationMode, RelationFilter, RuntimeProvenance,
};
use earmark_store::CanonicalStore;
use serde_json::Value;
use std::collections::BTreeMap;

impl<'a, S: CanonicalStore> RuntimeToolSurface<'a, S> {
    pub fn create_relation(
        &self,
        source_id: ObjectId,
        target_id: ObjectId,
        relation_type: String,
        metadata: Value,
        provenance: RuntimeProvenance,
    ) -> Result<ObjectRef, RuntimeToolError> {
        let source_head = self
            .store
            .read_head(&source_id)?
            .ok_or_else(|| RuntimeToolError::MissingObject(source_id.as_str().to_string()))?;
        let target_head = self
            .store
            .read_head(&target_id)?
            .ok_or_else(|| RuntimeToolError::MissingObject(target_id.as_str().to_string()))?;

        let authorization = crate::modules::relation_rules::authorize_relation_creation(
            self,
            &source_head,
            &target_head,
            &relation_type,
        )?;

        let mut headers = BTreeMap::new();
        headers.insert(
            "relation_auth_endpoint".to_string(),
            earmark_core::HeaderValue::String(match authorization.endpoint {
                crate::modules::relation_rules::AuthorizingEndpoint::Source => "source".to_string(),
                crate::modules::relation_rules::AuthorizingEndpoint::Target => "target".to_string(),
            }),
        );
        headers.insert(
            "relation_auth_class".to_string(),
            earmark_core::HeaderValue::String(authorization.class_name),
        );
        headers.insert(
            "relation_auth_authority".to_string(),
            earmark_core::HeaderValue::String(authorization.authorizing_endpoint),
        );
        headers.insert(
            "relation_auth_direction".to_string(),
            earmark_core::HeaderValue::String(authorization.direction),
        );

        let mut qualifiers = BTreeMap::new();
        if let Value::Object(map) = metadata {
            for (k, v) in map {
                let scalar = serde_json::from_value(v)?;
                qualifiers.insert(k, scalar);
            }
        }

        let relation = earmark_core::RelationPayload {
            source: source_head.object_ref(),
            target: target_head.object_ref(),
            relation_type,
            qualifiers,
            scope: None,
        };

        let provenance = Provenance {
            actor: provenance.actor,
            source_type: provenance.source_type,
            source_ref: None,
            lineage: vec![],
            import_path: None,
            captured_at: chrono::Utc::now(),
        };

        Ok(earmark_exec::persist_relation_canonical(
            self.store,
            self.index,
            relation,
            provenance,
            RelationCreationMode::Declared,
            Some(headers),
        )?)
    }
}

pub(crate) fn relation_type_allowed(relation_type: &str, filter: Option<&RelationFilter>) -> bool {
    filter
        .map(|filter| {
            filter.allowed_types.is_empty()
                || filter
                    .allowed_types
                    .iter()
                    .any(|allowed| allowed == relation_type)
        })
        .unwrap_or(true)
}
