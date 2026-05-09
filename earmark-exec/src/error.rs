// Imports are used via full paths in thiserror macros, so explicit imports are redundant.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderFailureKind {
    ProviderUnavailable,
    RateLimited,
    AuthenticationFailed,
    BudgetExceeded,
    Timeout,
    MalformedResponse,
    PolicyViolation,
    AdapterNotRegistered,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderFailure {
    pub kind: ProviderFailureKind,
    pub message: String,
}

impl std::fmt::Display for ProviderFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl ProviderFailure {
    pub fn new(kind: ProviderFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("invalid workflow: {0}")]
    InvalidWorkflow(String),
    #[error("conflicting continuation sources: {0}")]
    ConflictingContinuationSources(String),
    #[error("missing transition assignment: {0}")]
    MissingTransitionAssignment(String),
    #[error("invalid transition assignment: {0}")]
    InvalidTransitionAssignment(String),
    #[error("missing work surface: {0}")]
    MissingWorkSurface(String),
    #[error("missing input: {0}")]
    MissingInput(String),
    #[error("missing handoff manifest: {0}")]
    MissingHandoffManifest(String),
    #[error("unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("incomplete execution: {0}")]
    IncompleteExecution(String),
    #[error("provider failure: {0}")]
    Provider(ProviderFailure),
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("index error: {0}")]
    Index(#[from] earmark_index::IndexError),
    #[error("project error: {0}")]
    Project(#[from] earmark_connected_context::ProjectError),
    #[error("governance error: {0}")]
    Governance(#[from] earmark_governance::GovernanceError),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("serde json error: {0}")]
    Json(#[from] serde_json::Error),
}
