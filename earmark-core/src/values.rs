//! General scalar, body, and header value wrappers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::ObjectRef;

pub type Timestamp = DateTime<Utc>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HeaderValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    Strings(Vec<String>),
}

impl HeaderValue {
    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.clone()),
            Self::Integer(v) => Some(v.to_string()),
            Self::Float(v) => Some(v.to_string()),
            Self::Bool(v) => Some(v.to_string()),
            Self::Strings(v) => Some(v.join(", ")),
        }
    }
}

impl From<String> for HeaderValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for HeaderValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarValue {
    String(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarOrRef {
    Scalar(ScalarValue),
    Object(ObjectRef),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownBody(String);

impl MarkdownBody {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MarkdownBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonSchemaRef(pub String);
