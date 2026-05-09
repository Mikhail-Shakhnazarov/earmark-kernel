use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeToolError {
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("index error: {0}")]
    Index(#[from] earmark_index::IndexError),
    #[error("derive error: {0}")]
    Derive(#[from] earmark_declarations::DeriveError),
    #[error("project error: {0}")]
    Project(#[from] earmark_connected_context::ProjectError),
    #[error("governance error: {0}")]
    Governance(#[from] earmark_governance::GovernanceError),
    #[error("execution error: {0}")]
    Exec(#[from] earmark_exec::ExecError),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("missing object: {0}")]
    MissingObject(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid payload shape: {0}")]
    InvalidPayloadShape(String),
    #[error("relation rule violation: {0}")]
    RelationRuleViolation(String),
    #[error("missing class definition for {0}")]
    MissingClassDefinition(String),
}
