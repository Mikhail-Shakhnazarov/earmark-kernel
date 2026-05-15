//! Context manifest and related types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ObjectId, ObjectRef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassFilter {
    pub allowed_classes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeProvenance {
    pub actor: String,
    pub source_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectedContextManifest {
    pub root_object_ids: Vec<ObjectId>,
    pub object_refs: Vec<ObjectRef>,
    pub relation_refs: Vec<ObjectRef>,
    pub max_depth: usize,
    pub generated_at: DateTime<Utc>,
}
