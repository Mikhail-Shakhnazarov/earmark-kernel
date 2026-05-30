/*
 * Copyright (c) 2026 Mikhail Shakhnazarov. Dual-licensed under AGPL-3.0-or-later or commercial terms.
 * PROPRIETARY AND INTERNAL. ONLY LOCALLY COMMITTED.
 * v0.1_internal kernel.
 */

use crate::errors::StoreError;
use crate::traits::CanonicalStore;
use chrono::Utc;
use earmark_core::{
    ActorId, ClassId, ObjectId, ObjectRecord, RelationId, RelationRecord, SignalState, Standing,
    VersionId, VersionRecord,
};

pub struct DepositObjectInput {
    pub id: Option<ObjectId>,
    pub class_id: Option<ClassId>,
    pub payload: serde_json::Value,
    pub standing: Standing,
    pub signal: Option<SignalState>,
}

pub fn deposit_object(
    store: &dyn CanonicalStore,
    registry: &dyn earmark_declarations::traits::DeclarationRegistry,
    actor: ActorId,
    input: DepositObjectInput,
) -> Result<ObjectRecord, StoreError> {
    let now = Utc::now();
    let version_id = VersionId::generate();

    // Strict sovereignty check: Validate class ID against the registry
    if let Some(ref class_id) = input.class_id {
        registry.get_class(class_id).map_err(|e| {
            StoreError::Sovereignty(format!(
                "Class {} is not registered. Error: {}",
                class_id.as_str(),
                e
            ))
        })?;
    }

    let (object_id, is_new) = if let Some(id) = input.id {
        if let Ok(existing) = store.get_object(&id) {
            (existing.id, false)
        } else {
            (id, true)
        }
    } else {
        (ObjectId::generate(), true)
    };

    let mut signal = input.signal;
    if signal.is_none() {
        if let Some(ref class_id) = input.class_id {
            if let Ok(class_decl) = registry.get_class(class_id) {
                if class_decl.intrinsic_signal {
                    signal = Some(earmark_core::SignalState {
                        signal_type: earmark_core::SignalType::Produced,
                        integrity_state: earmark_core::SignalIntegrityState::Unchecked,
                        production_context: None, // Direct deposition
                        acceptance_review: None,
                    });
                }
            }
        }
    }

    let object_record = if is_new {
        ObjectRecord {
            id: object_id.clone(),
            class_id: input.class_id.clone(),
            latest_version_id: version_id.clone(),
            created_at: now,
            updated_at: now,
        }
    } else {
        let mut obj = store.get_object(&object_id)?;
        obj.latest_version_id = version_id.clone();
        obj.updated_at = now;
        obj
    };

    let version_record = VersionRecord {
        version_id,
        object_id: object_id.clone(),
        payload: input.payload,
        standing: input.standing,
        signal,
        created_at: now,
        created_by: Some(actor),
    };

    store.deposit_object(object_record.clone(), version_record)?;
    Ok(object_record)
}

pub struct CreateRelationInput {
    pub source_id: ObjectId,
    pub target_id: ObjectId,
    pub relation_type: String,
}

pub fn create_relation(
    store: &dyn CanonicalStore,
    actor: ActorId,
    input: CreateRelationInput,
) -> Result<RelationRecord, StoreError> {
    // TODO: Validate against DeclarationRegistry in WP5
    let relation_id = RelationId::generate();
    let now = Utc::now();

    let record = RelationRecord {
        id: relation_id,
        source_id: input.source_id,
        target_id: input.target_id,
        relation_type: input.relation_type,
        created_at: now,
        created_by: Some(actor),
    };

    store.create_relation(record.clone())?;
    Ok(record)
}
