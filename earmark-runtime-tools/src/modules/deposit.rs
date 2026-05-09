use std::collections::BTreeMap;
use serde_json::{json, Value};
use earmark_core::{
    HeaderValue, Kind, ObjectId, ObjectRef, Provenance, RuntimeProvenance, Standing, VersionId, VersionRef,
};
use earmark_store::{CanonicalStore, StoredObject, StoredPayload, PayloadEncoding};
use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;

impl<'a, S: CanonicalStore> RuntimeToolSurface<'a, S> {
    pub fn deposit_prose(
        &self,
        class: String,
        title: Option<String>,
        body: String,
    ) -> Result<VersionRef, RuntimeToolError> {
        let prov = RuntimeProvenance {
            actor: "runtime".to_string(),
            source_type: "prose_deposit".to_string(),
        };
        let object_ref = self.deposit_object(
            class,
            Some("object".to_string()),
            title,
            json!(body),
            prov,
        )?;
        Ok(VersionRef::new(object_ref.id, object_ref.version_id))
    }

    pub fn deposit_object(
        &self,
        class: String,
        kind: Option<String>,
        title: Option<String>,
        payload: Value,
        provenance: RuntimeProvenance,
    ) -> Result<ObjectRef, RuntimeToolError> {
        earmark_core::validate_class_name(&class)?;
        if let Some(title) = &title {
            earmark_core::validate_title(title)?;
        }
        let mut headers = BTreeMap::new();
        if let Some(title) = title {
            headers.insert("title".to_string(), HeaderValue::String(title));
        }
        let k = kind
            .and_then(|k| k.parse::<Kind>().ok())
            .unwrap_or(Kind::Object);

        let stored_payload = match k {
            Kind::Instruction | Kind::Object if payload.is_string() => {
                StoredPayload::from_markdown(
                    payload
                        .as_str()
                        .ok_or_else(|| {
                            RuntimeToolError::InvalidPayloadShape(
                                "Expected string payload for markdown".to_string(),
                            )
                        })?
                        .to_string(),
                )
            }
            Kind::Workflow
            | Kind::Policy
            | Kind::CompiledContextTemplate
            | Kind::ProviderProfile
            | Kind::SystemDefinition => {
                if payload.is_string() {
                    StoredPayload::from_yaml(
                        payload
                            .as_str()
                            .ok_or_else(|| {
                                RuntimeToolError::InvalidPayloadShape(
                                    "Expected string payload for yaml".to_string(),
                                )
                            })?
                            .to_string(),
                    )
                } else {
                    StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&payload)?)
                }
            }
            _ => StoredPayload::from_json_bytes(serde_json::to_vec_pretty(&payload)?),
        };

        earmark_core::validate_payload_size(stored_payload.bytes.len())?;

        if let Some((obj_id, version_id)) = self.index.find_class_definition(&class)? {
            let class_ref = VersionRef::new(
                ObjectId::parse(obj_id)?,
                VersionId::parse(version_id)?,
            );
            let class_obj = self.store.read_version(&class_ref)?;
            let class_def: earmark_core::ClassDefinition =
                earmark_core::parse_yaml(&class_obj.payload.as_utf8()?)?;

            if !class_def.payload_schema.0.is_empty() && class_def.payload_schema.0 != "inline:any" {
                let schema_str = if class_def.payload_schema.0.starts_with("inline:") {
                    &class_def.payload_schema.0[7..]
                } else {
                    &class_def.payload_schema.0
                };
                
                let schema: Value = serde_json::from_str(schema_str)?;

                let payload_json = match &stored_payload.format {
                    PayloadEncoding::Json => {
                        serde_json::from_slice(&stored_payload.bytes)?
                    }
                    _ => {
                        let text = stored_payload.as_utf8()?;
                        serde_json::from_str(&text).unwrap_or(json!(text))
                    }
                };
                earmark_core::validate_schema(&payload_json, &schema)?;
            }
        }

        let object = StoredObject::new(
            k.clone(),
            Some(class.clone()),
            Standing::default(),
            Provenance {
                actor: provenance.actor,
                source_type: provenance.source_type,
                source_ref: None,
                lineage: vec![],
                import_path: None,
                captured_at: chrono::Utc::now(),
            },
            headers,
            stored_payload,
            vec![],
        );
        let version_ref = self.store.write_object(&object)?;
        self.index
            .upsert_head_object_from_store(self.store, &version_ref.id)?;
        Ok(ObjectRef::new(
            version_ref.id,
            version_ref.version_id,
            k,
            Some(class),
        ))
    }
}
