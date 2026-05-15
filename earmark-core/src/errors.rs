//! Core error types for the earmark system.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("serde yaml error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("invalid kind: {0}")]
    InvalidKind(String),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("payload too large: {0} bytes (max {1} bytes)")]
    PayloadTooLarge(usize, usize),
    #[error("unknown class: {0}")]
    UnknownClass(String),
    #[error("schema violation: {0}")]
    SchemaViolation(String),
    #[error("schema unavailable: {0}")]
    SchemaUnavailable(String),
    #[error("security violation: {0}")]
    SecurityViolation(String),
}
