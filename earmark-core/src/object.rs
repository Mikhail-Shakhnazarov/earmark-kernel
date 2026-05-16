//! Object envelope and provenance material.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ids::{ObjectId, ObjectRef, PayloadRef, VersionId, VersionRef};
use crate::kind::Kind;
use crate::standing::Standing;
use crate::values::{HeaderValue, Timestamp};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageLink {
    pub rel: String,
    pub object: ObjectRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub actor: String,
    pub source_type: String,
    pub source_ref: Option<ObjectRef>,
    pub lineage: Vec<LineageLink>,
    pub import_path: Option<String>,
    pub captured_at: Timestamp,
}

impl Provenance {
    pub fn direct_input(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            source_type: "direct_input".to_string(),
            source_ref: None,
            lineage: vec![],
            import_path: None,
            captured_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub id: ObjectId,
    pub version_id: VersionId,
    pub kind: Kind,
    pub class: Option<String>,
    pub standing: Standing,
    pub provenance: Provenance,
    pub headers: BTreeMap<String, HeaderValue>,
    pub payload_ref: PayloadRef,
    pub parents: Vec<VersionRef>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Envelope {
    pub fn object_ref(&self) -> ObjectRef {
        ObjectRef::new(
            self.id.clone(),
            self.version_id.clone(),
            self.kind.clone(),
            self.class.clone(),
        )
    }

    pub fn version_ref(&self) -> VersionRef {
        VersionRef::new(self.id.clone(), self.version_id.clone())
    }

    pub fn title(&self) -> Option<String> {
        self.headers.get("title").and_then(HeaderValue::as_string)
    }
}
