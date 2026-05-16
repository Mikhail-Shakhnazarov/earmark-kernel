use crate::modules::error::RuntimeToolError;
use crate::modules::surface::RuntimeToolSurface;
use earmark_core::{
    HeaderValue, Kind, ObjectId, ObjectRef, Provenance, RuntimeProvenance, Standing, VersionId,
    VersionRef,
};
use earmark_exec::persistence_helpers::write_object_and_index;
use earmark_store::{CanonicalStore, PayloadEncoding, StoredObject, StoredPayload};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct DepositValidationContext {
    pub namespace: Option<String>,
    pub headers: BTreeMap<String, HeaderValue>,
}

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
            DepositValidationContext::default(),
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
        validation_context: DepositValidationContext,
    ) -> Result<ObjectRef, RuntimeToolError> {
        earmark_core::validate_class_name(&class)?;
        if let Some(title) = &title {
            earmark_core::validate_title(title)?;
        }

        // 1. Resolve admission context
        let mut admitted_class_definition: Option<earmark_core::ClassDefinition> = None;

        if let Some(namespace) = &validation_context.namespace {
            if let Some(active) = self.index.get_active_system(namespace)? {
                // Load active system definition
                let system_ref = VersionRef::new(
                    ObjectId::parse(active.object_id)?,
                    VersionId::parse(active.version_id)?,
                );
                let system_obj = self.store.read_version(&system_ref)?;
                let system_def: earmark_core::SystemDefinition =
                    earmark_core::parse_yaml(&system_obj.payload.as_utf8()?)?;

                // Resolve admitted classes
                let mut found_admitted = false;
                for class_ref in &system_def.classes {
                    let class_obj = self.store.read_version(class_ref).map_err(|_| {
                        RuntimeToolError::SystemIntegrity(format!(
                            "admitted class reference {:?} not found",
                            class_ref
                        ))
                    })?;
                    let class_def: earmark_core::ClassDefinition =
                        earmark_core::parse_yaml(&class_obj.payload.as_utf8()?).map_err(|_| {
                            RuntimeToolError::SystemIntegrity(format!(
                                "failed to decode admitted class definition {:?}",
                                class_ref
                            ))
                        })?;

                    if class_def.name == class {
                        earmark_declarations::validate_class_definition(&class_def).map_err(
                            |e| {
                                RuntimeToolError::SystemIntegrity(format!(
                                    "admitted class definition '{}' is inconsistent: {}",
                                    class_def.name, e
                                ))
                            },
                        )?;
                        admitted_class_definition = Some(class_def);
                        found_admitted = true;
                        break;
                    }
                }

                if !found_admitted {
                    return Err(RuntimeToolError::AdmissionError {
                        requested_class: class,
                        namespace: namespace.clone(),
                        system_id: active.system_id,
                    });
                }
            }
        }

        // If no active system admitted a specific version, look up the latest by name (scratch/registered behavior)
        if admitted_class_definition.is_none() {
            if let Some((obj_id, version_id)) = self.index.find_class_definition(&class)? {
                let class_ref =
                    VersionRef::new(ObjectId::parse(obj_id)?, VersionId::parse(version_id)?);
                let class_obj = self.store.read_version(&class_ref)?;
                let class_def: earmark_core::ClassDefinition =
                    earmark_core::parse_yaml(&class_obj.payload.as_utf8()?)?;
                earmark_declarations::validate_class_definition(&class_def).map_err(|e| {
                    RuntimeToolError::SystemIntegrity(format!(
                        "class definition '{}' is inconsistent: {}",
                        class_def.name, e
                    ))
                })?;
                admitted_class_definition = Some(class_def);
            }
        }

        let mut headers = BTreeMap::new();
        if let Some(title) = title {
            headers.insert("title".to_string(), HeaderValue::String(title));
        }
        for (k, v) in validation_context.headers {
            headers.insert(k, v);
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

        // 2. Apply schema validation if a class definition was resolved
        if let Some(class_def) = admitted_class_definition {
            if !class_def.payload_schema.0.is_empty() && class_def.payload_schema.0 != "inline:any"
            {
                let schema_str = if class_def.payload_schema.0.starts_with("inline:") {
                    &class_def.payload_schema.0[7..]
                } else {
                    &class_def.payload_schema.0
                };

                let schema: Value = serde_json::from_str(schema_str)?;

                let payload_json = match &stored_payload.format {
                    PayloadEncoding::Json => serde_json::from_slice(&stored_payload.bytes)?,
                    _ => {
                        let text = stored_payload.as_utf8()?;
                        serde_json::from_str(&text).unwrap_or(json!(text))
                    }
                };
                earmark_core::validate_schema(&payload_json, &schema)?;
            }

            // Check required headers
            for req in &class_def.required_headers {
                if !headers.contains_key(req) {
                    return Err(RuntimeToolError::InvalidPayloadShape(format!(
                        "missing required header '{}' for class '{}'",
                        req, class_def.name
                    )));
                }
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
        let version_ref = write_object_and_index(self.store, self.index, &object)?;
        Ok(ObjectRef::new(
            version_ref.id,
            version_ref.version_id,
            k,
            Some(class),
        ))
    }
}
