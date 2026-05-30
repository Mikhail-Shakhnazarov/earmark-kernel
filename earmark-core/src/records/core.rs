/*
 * Copyright (c) 2026 Mikhail Shakhnazarov.
 * Dual-licensed under AGPL-3.0-or-later or commercial terms.
 */

use crate::ids::{ActorId, ClassId, ObjectId, RelationId, VersionId};
use crate::records::signal::SignalState;
use crate::standing::Standing;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectRecord {
    pub id: ObjectId,
    pub class_id: Option<ClassId>,
    pub latest_version_id: VersionId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationRecord {
    pub id: RelationId,
    pub source_id: ObjectId,
    pub target_id: ObjectId,
    pub relation_type: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<ActorId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionRecord {
    pub version_id: VersionId,
    pub object_id: ObjectId,
    pub payload: serde_json::Value,
    pub standing: Standing,
    pub signal: Option<SignalState>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<ActorId>,
}
