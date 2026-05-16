use crate::cli::{Commands, DeclareAction, StandingRequestAction};
use crate::config::CliConfig;
use earmark_exec::ProviderRegistry;
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, WorkspaceLayoutStatus};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("store error: {0}")]
    Store(#[from] earmark_store::StoreError),
    #[error("index error: {0}")]
    Index(#[from] earmark_index::IndexError),
    #[error("derive error: {0}")]
    Derive(#[from] earmark_declarations::DeriveError),
    #[error("execution error: {0}")]
    Exec(#[from] earmark_exec::ExecError),
    #[error("governance error: {0}")]
    Governance(#[from] earmark_governance::GovernanceError),
    #[error("core error: {0}")]
    Core(#[from] earmark_core::CoreError),
    #[error("serde json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("serde yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("argument error: {0}")]
    Argument(String),
    #[error("workspace is not initialized; run `em init` before using this command")]
    WorkspaceNotInitialized { status: WorkspaceLayoutStatus },
    #[error("runtime error: {0}")]
    Runtime(#[from] earmark_runtime_tools::RuntimeToolError),
}

impl CliError {
    pub fn argument(message: impl Into<String>) -> Self {
        Self::Argument(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn workspace_not_initialized(status: WorkspaceLayoutStatus) -> Self {
        Self::WorkspaceNotInitialized { status }
    }
}

pub struct CommandContext<'a> {
    pub store: &'a GitCanonicalStore,
    pub index: &'a Option<DerivedIndex>,
    pub config: &'a CliConfig,
    pub provider_registry: &'a ProviderRegistry,
    pub as_json: bool,
    pub actor: &'a str,
}

pub struct BootstrappedServices {
    pub store: GitCanonicalStore,
    pub index: Option<DerivedIndex>,
    pub config: CliConfig,
    pub provider_registry: ProviderRegistry,
    pub as_json: bool,
    pub root: PathBuf,
    pub actor: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceAccessMode {
    None,
    ReadOnly,
    Write,
    Init,
    RepairIndex,
}

pub fn workspace_access_mode(command: &Commands) -> WorkspaceAccessMode {
    match command {
        Commands::Completions { .. } => WorkspaceAccessMode::None,
        Commands::Init => WorkspaceAccessMode::Init,
        Commands::Doctor(args) => {
            if args.repair_index {
                WorkspaceAccessMode::RepairIndex
            } else {
                WorkspaceAccessMode::None
            }
        }
        Commands::Status => WorkspaceAccessMode::ReadOnly,
        Commands::Query(_) => WorkspaceAccessMode::ReadOnly,
        Commands::Run(_) => WorkspaceAccessMode::ReadOnly,
        Commands::Assignment(_) => WorkspaceAccessMode::ReadOnly,
        Commands::ChangeSet(_) => WorkspaceAccessMode::ReadOnly,
        Commands::Handoff(_) => WorkspaceAccessMode::ReadOnly,
        Commands::Failure(_) => WorkspaceAccessMode::ReadOnly,
        Commands::Audit(_) => WorkspaceAccessMode::ReadOnly,
        Commands::Declare(cmd) => match cmd.action {
            DeclareAction::Validate(_)
            | DeclareAction::Explain(_)
            | DeclareAction::ListExamples => WorkspaceAccessMode::ReadOnly,
            DeclareAction::New(_) | DeclareAction::Register(_) => WorkspaceAccessMode::Write,
        },
        Commands::System(_) => WorkspaceAccessMode::Write,
        Commands::Deposit(_) => WorkspaceAccessMode::Write,
        Commands::Review(_) => WorkspaceAccessMode::Write,
        Commands::Workflow(_) => WorkspaceAccessMode::Write,
        Commands::Context(_) => WorkspaceAccessMode::Write,
        Commands::Report(_) => WorkspaceAccessMode::Write,
        Commands::Provider(_) => WorkspaceAccessMode::None,
        Commands::Relation(_) => WorkspaceAccessMode::ReadOnly,
        Commands::StandingRequest(cmd) => match cmd.action {
            StandingRequestAction::List { .. } | StandingRequestAction::Show { .. } => {
                WorkspaceAccessMode::ReadOnly
            }
            StandingRequestAction::Approve { .. }
            | StandingRequestAction::Reject { .. }
            | StandingRequestAction::Apply { .. } => WorkspaceAccessMode::Write,
        },
        Commands::Undo(_) => WorkspaceAccessMode::Write,
    }
}

pub fn require_initialized_workspace(store: &GitCanonicalStore) -> Result<(), CliError> {
    let status = store.layout_status();
    if status.is_initialized() {
        return Ok(());
    }
    Err(CliError::workspace_not_initialized(status))
}

pub fn command_family_name(command: &Commands) -> &'static str {
    match command {
        Commands::Init => "init",
        Commands::Doctor(_) => "doctor",
        Commands::System(_) => "system",
        Commands::Deposit(_) => "deposit",
        Commands::Query(_) => "query",
        Commands::Review(_) => "review",
        Commands::Workflow(_) => "workflow",
        Commands::Run(_) => "run",
        Commands::Declare(_) => "declare",
        Commands::Assignment(_) => "assignment",
        Commands::ChangeSet(_) => "changeset",
        Commands::Handoff(_) => "handoff",
        Commands::Failure(_) => "failure",
        Commands::Context(_) => "context",
        Commands::Audit(_) => "audit",
        Commands::Report(_) => "report",
        Commands::Provider(_) => "provider",
        Commands::Completions { .. } => "completions",
        Commands::Status => "status",
        Commands::Relation(_) => "relation",
        Commands::StandingRequest(_) => "standing-request",
        Commands::Undo(_) => "undo",
    }
}
