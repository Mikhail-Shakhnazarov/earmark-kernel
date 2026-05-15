//! Relation material: constants, payloads, and rules.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ids::ObjectRef;
use crate::values::ScalarValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationCreationMode {
    Declared,
    PrivilegedSystem,
}

pub const REL_TYPE_USED_INSTRUCTION: &str = "used_instruction";
pub const REL_TYPE_USED_COMPILED_CONTEXT: &str = "used_compiled_context";
pub const REL_TYPE_REQUESTS_STANDING: &str = "requests_standing";
pub const REL_TYPE_RESULTED_IN_FAILURE: &str = "resulted_in_failure";
pub const REL_TYPE_DERIVED_FROM: &str = "derived_from";

pub const PRIVILEGED_RELATION_TYPES: &[&str] = &[
    REL_TYPE_USED_INSTRUCTION,
    REL_TYPE_USED_COMPILED_CONTEXT,
    REL_TYPE_REQUESTS_STANDING,
    REL_TYPE_RESULTED_IN_FAILURE,
    REL_TYPE_DERIVED_FROM,
];

pub fn is_privileged_relation(rel_type: &str) -> bool {
    PRIVILEGED_RELATION_TYPES.contains(&rel_type)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationPayload {
    pub source: ObjectRef,
    pub target: ObjectRef,
    pub relation_type: String,
    pub qualifiers: BTreeMap<String, ScalarValue>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationRule {
    pub relation_type: String,
    pub counterparty_classes: Vec<String>,
    pub direction: Option<String>,
    pub authorizing_endpoint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationFilter {
    pub allowed_types: Vec<String>,
}
