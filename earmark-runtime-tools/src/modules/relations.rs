use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_core::{
    Kind, ObjectId, ObjectRef, Provenance, RelationFilter, RuntimeProvenance, Standing,
};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload};
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
        
        crate::modules::relation_rules::validate_relation_creation(
            self,
            &source_head,
            &target_head,
            &relation_type,
        )?;

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

        let stored = StoredObject::new(
            Kind::Relation,
            None,
            Standing::default(),
            Provenance {
                actor: provenance.actor,
                source_type: provenance.source_type,
                source_ref: None,
                lineage: vec![],
                import_path: None,
                captured_at: chrono::Utc::now(),
            },
            BTreeMap::new(),
            StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&relation)?),
            vec![],
        );
        let version_ref = self.store.write_object(&stored)?;
        self.index
            .upsert_head_object_from_store(self.store, &version_ref.id)?;
        Ok(ObjectRef::new(
            version_ref.id,
            version_ref.version_id,
            Kind::Relation,
            None,
        ))
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
