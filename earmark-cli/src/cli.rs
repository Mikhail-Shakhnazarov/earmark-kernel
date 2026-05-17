use std::path::PathBuf;

use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "em")]
#[command(about = "Earmark operator shell")]
pub struct Cli {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub log_level: Option<String>,
    #[arg(long, action = ArgAction::Count)]
    pub verbose: u8,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStability {
    Stable,
    Beta,
    Experimental,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "[STABLE] Initialize a new earmark workspace")]
    Init,
    #[command(about = "[BETA] Diagnose and repair workspace issues")]
    Doctor(DoctorArgs),
    #[command(about = "[EXPERIMENTAL] Manage system registration")]
    System(SystemCommand),
    #[command(about = "[STABLE] Deposit an object into the store")]
    Deposit(DepositArgs),
    #[command(about = "[STABLE] Query the object store")]
    Query(QueryArgs),
    #[command(about = "[STABLE] Review an object")]
    Review(ReviewArgs),
    #[command(about = "[STABLE] Manage workflows")]
    Workflow(WorkflowCommand),
    #[command(about = "[STABLE] Manage runs")]
    Run(RunCommand),
    #[command(about = "[BETA] Declare and register declarations")]
    Declare(DeclareCommand),
    #[command(about = "[STABLE] Manage assignments")]
    Assignment(AssignmentCommand),
    #[command(about = "[STABLE] Manage change sets")]
    ChangeSet(ChangeSetCommand),
    #[command(about = "[STABLE] Manage handoffs")]
    Handoff(HandoffCommand),
    #[command(about = "[STABLE] Manage failures")]
    Failure(FailureCommand),
    #[command(about = "[EXPERIMENTAL] Compile context")]
    Context(ContextCommand),
    #[command(about = "[BETA] Audit workspace events")]
    Audit(AuditCommand),
    #[command(about = "[STABLE] Generate reports")]
    Report(ReportCommand),
    #[command(about = "[BETA] Manage providers")]
    Provider(ProviderCommand),
    #[command(about = "[BETA] Generate shell completions")]
    Completions { shell: CompletionShell },
    #[command(about = "[STABLE] Show workspace status")]
    Status,
    #[command(about = "[EXPERIMENTAL] Manage relations")]
    Relation(RelationCommand),
    #[command(about = "[EXPERIMENTAL] Manage standing requests")]
    StandingRequest(StandingRequestCommand),
    #[command(about = "[BETA] Undo a run")]
    Undo(UndoCommand),
    #[command(about = "[EXPERIMENTAL] Manage orchestration tasks")]
    Orchestration(OrchestrationCommand),
}

impl Commands {
    pub fn stability(&self) -> CommandStability {
        match self {
            Self::Init
            | Self::Status
            | Self::Query(_)
            | Self::Deposit(_)
            | Self::Run(_)
            | Self::Workflow(_)
            | Self::Assignment(_)
            | Self::ChangeSet(_)
            | Self::Handoff(_)
            | Self::Failure(_)
            | Self::Report(_)
            | Self::Review(_) => CommandStability::Stable,

            Self::Doctor(_)
            | Self::Declare(_)
            | Self::Audit(_)
            | Self::Provider(_)
            | Self::Completions { .. }
            | Self::Undo(_) => CommandStability::Beta,

            Self::System(_)
            | Self::Context(_)
            | Self::Relation(_)
            | Self::StandingRequest(_)
            | Self::Orchestration(_) => CommandStability::Experimental,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

#[derive(Args)]
pub struct SystemCommand {
    #[command(subcommand)]
    pub action: SystemAction,
}

#[derive(Args)]
pub struct DoctorArgs {
    #[arg(long, help = "rebuild the derived index from canonical store")]
    pub repair_index: bool,
}

#[derive(Subcommand)]
pub enum SystemAction {
    Register { manifest: PathBuf },
    Activate { system_id: Option<String> },
}

#[derive(Args)]
pub struct WorkflowCommand {
    #[command(subcommand)]
    pub action: WorkflowAction,
}

#[derive(Subcommand)]
pub enum WorkflowAction {
    Run(RunWorkflowArgs),
}

#[derive(Args)]
pub struct RunCommand {
    #[command(subcommand)]
    pub action: RunAction,
}

#[derive(Subcommand)]
pub enum RunAction {
    Show {
        #[arg(help = "Run ID or 'latest'")]
        run_id: String,
    },
    List,
    Timeline {
        #[arg(help = "Run ID or 'latest'")]
        run_id: String,
    },
    Artifacts {
        #[arg(help = "Run ID or 'latest'")]
        run_id: String,
    },
    Explain {
        #[arg(help = "Run ID or 'latest'")]
        run_id: String,
    },
    Graph {
        #[arg(help = "Run ID or 'latest'")]
        run_id: String,
    },
}

#[derive(Args)]
pub struct DeclareCommand {
    #[command(subcommand)]
    pub action: DeclareAction,
}

#[derive(Subcommand)]
pub enum DeclareAction {
    New(DeclareNewArgs),
    Validate(DeclareFileArgs),
    Explain(DeclareFileArgs),
    Register(DeclareFileArgs),
    ListExamples,
}

#[derive(Args)]
pub struct DeclareFileArgs {
    #[arg(long, value_enum)]
    pub kind: DeclarationKind,
    pub path: PathBuf,
}

#[derive(Args)]
pub struct DeclareNewArgs {
    #[arg(long, value_enum)]
    pub kind: DeclarationKind,
    pub name: String,
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DeclarationKind {
    Class,
    Instruction,
    StandingPolicy,
    Workflow,
    CompiledContext,
    ProviderProfile,
    System,
}

#[derive(Args)]
pub struct AssignmentCommand {
    #[command(subcommand)]
    pub action: AssignmentAction,
}

#[derive(Subcommand)]
pub enum AssignmentAction {
    Show {
        #[arg(help = "durable ID of the assignment to display")]
        assignment_id: String,
    },
    Explain {
        #[arg(help = "durable ID of the assignment to explain")]
        assignment_id: String,
    },
    List {
        #[arg(long, help = "Run ID or 'latest'")]
        run_id: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
}

#[derive(Args)]
pub struct ChangeSetCommand {
    #[command(subcommand)]
    pub action: ChangeSetAction,
}

#[derive(Subcommand)]
pub enum ChangeSetAction {
    Show {
        #[arg(help = "durable ID of the change set to display")]
        change_set_id: String,
    },
    Explain {
        #[arg(help = "durable ID of the change set to explain")]
        change_set_id: String,
    },
    List {
        #[arg(long, help = "Run ID or 'latest'")]
        run_id: Option<String>,
    },
}

#[derive(Args)]
pub struct HandoffCommand {
    #[command(subcommand)]
    pub action: HandoffAction,
}

#[derive(Subcommand)]
pub enum HandoffAction {
    Show {
        #[arg(help = "durable ID of the handoff to display")]
        handoff_id: String,
    },
    Explain {
        #[arg(help = "durable ID of the handoff to explain")]
        handoff_id: String,
    },
    List {
        #[arg(long, help = "Run ID or 'latest'")]
        run_id: Option<String>,
    },
}

#[derive(Args)]
pub struct FailureCommand {
    #[command(subcommand)]
    pub action: FailureAction,
}

#[derive(Subcommand)]
pub enum FailureAction {
    Show {
        #[arg(help = "durable ID of the failure to show")]
        failure_id: String,
    },
    Explain {
        #[arg(help = "durable ID of the failure to explain")]
        failure_id: String,
    },
    List {
        #[arg(long, help = "Run ID or 'latest'")]
        run_id: Option<String>,
        #[arg(long)]
        transition_id: Option<String>,
    },
}

#[derive(Args)]
pub struct RelationCommand {
    #[command(subcommand)]
    pub action: RelationAction,
}

#[derive(Subcommand)]
pub enum RelationAction {
    Show {
        #[arg(help = "durable ID of the relation to display")]
        relation_id: String,
    },
    Explain {
        #[arg(help = "durable ID of the relation to explain")]
        relation_id: String,
    },
    List {
        #[arg(long)]
        source_id: Option<String>,
        #[arg(long)]
        target_id: Option<String>,
        #[arg(long)]
        relation_type: Option<String>,
    },
}

#[derive(Args)]
pub struct ContextCommand {
    #[command(subcommand)]
    pub action: ContextAction,
}

#[derive(Subcommand)]
pub enum ContextAction {
    Compile(CompileContextArgs),
}

#[derive(Args)]
pub struct AuditCommand {
    #[command(subcommand)]
    pub action: AuditAction,
}

#[derive(Subcommand)]
pub enum AuditAction {
    Failures {
        #[arg(long, help = "Run ID or 'latest'")]
        run_id: Option<String>,
        #[arg(long)]
        transition_id: Option<String>,
    },
    Show {
        #[arg(help = "durable ID of the failure to show")]
        failure_id: String,
    },
}

#[derive(Args)]
pub struct ReportCommand {
    #[command(subcommand)]
    pub action: ReportAction,
}

#[derive(Subcommand)]
pub enum ReportAction {
    Run {
        #[arg(help = "Run ID or 'latest'")]
        target_id: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    Handoff {
        #[arg(help = "handoff ID to report on")]
        target_id: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    System {
        #[arg(help = "system ID to report on")]
        target_id: String,
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Args)]
pub struct DepositArgs {
    #[arg(long)]
    pub class: String,
    #[arg(long, default_value = "object")]
    pub kind: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub body: Option<String>,
    #[arg(long)]
    pub payload_file: Option<PathBuf>,
    #[arg(long)]
    pub json_payload: Option<String>,
    #[arg(long = "header", help = "custom header in key=value format")]
    pub headers: Vec<String>,
}

#[derive(Args)]
pub struct QueryArgs {
    #[arg(long)]
    pub class: Option<String>,
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long)]
    pub object_id: Option<String>,
}

#[derive(Args)]
pub struct ReviewArgs {
    #[arg(help = "durable ID of the object to review")]
    pub object_id: String,
    #[arg(long)]
    pub version_id: Option<String>,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long)]
    pub reject: bool,
}

#[derive(Args)]
pub struct RunWorkflowArgs {
    #[arg(help = "symbolic name or durable ID of the workflow")]
    pub workflow_id: String,
    #[arg(long)]
    pub version_id: Option<String>,
    #[arg(long)]
    pub system_id: Option<String>,
    #[arg(long = "with")]
    pub inputs: Vec<String>,
    #[arg(long)]
    pub handoff: Option<String>,
    #[arg(long)]
    pub assignment: Option<String>,
    #[arg(long)]
    pub approve_review: bool,
}

#[derive(Args)]
pub struct CompileContextArgs {
    #[arg(long = "root")]
    pub roots: Vec<String>,
    #[arg(long, default_value_t = 1)]
    pub depth: usize,
    #[arg(long = "relation-type")]
    pub relation_types: Vec<String>,
    #[arg(long = "class")]
    pub classes: Vec<String>,
    #[arg(long = "epistemic")]
    pub epistemic: Vec<String>,
}

#[derive(Args)]
pub struct ProviderCommand {
    #[command(subcommand)]
    pub action: ProviderAction,
}

#[derive(Subcommand)]
pub enum ProviderAction {
    Capabilities,
}

#[derive(Args)]
pub struct StandingRequestCommand {
    #[command(subcommand)]
    pub action: StandingRequestAction,
}

#[derive(Subcommand)]
pub enum StandingRequestAction {
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        target: Option<String>,
    },
    Show {
        #[arg(help = "durable ID of the standing request")]
        request_id: String,
    },
    Approve {
        #[arg(help = "durable ID of the standing request")]
        request_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    Reject {
        #[arg(help = "durable ID of the standing request")]
        request_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    Apply {
        #[arg(help = "durable ID of the standing request")]
        request_id: String,
        #[arg(long)]
        policy: Option<String>,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Args)]
pub struct UndoCommand {
    #[command(subcommand)]
    pub action: UndoAction,
}

#[derive(Subcommand)]
pub enum UndoAction {
    Run {
        #[arg(help = "Run ID or 'latest'")]
        run_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Args)]
pub struct OrchestrationCommand {
    #[command(subcommand)]
    pub action: OrchestrationAction,
}

#[derive(Subcommand)]
pub enum OrchestrationAction {
    #[command(name = "init-example")]
    InitExample,
    CaptureGit(CaptureGitArgs),
    IngestManifest(IngestManifestArgs),
    IngestReport(IngestReportArgs),
    RecordGate(RecordGateArgs),
    Review(OrchReviewArgs),
    Show(ShowTaskArgs),
    List(ListOrchestrationArgs),
    IngestTask(IngestTaskArgs),
}

#[derive(Args)]
pub struct CaptureGitArgs {
    #[arg(long)]
    pub task_id: String,
    #[arg(long)]
    pub phase: String,
    #[arg(long)]
    pub base: Option<String>,
    #[arg(long)]
    pub head: Option<String>,
    #[arg(long)]
    pub include_diff_stat: bool,
    #[arg(long)]
    pub commit: Option<String>,
}

#[derive(Args)]
pub struct IngestManifestArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub task_id: Option<String>,
    #[arg(long)]
    pub attempt: Option<usize>,
    #[arg(long)]
    pub executor: Option<String>,
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Args)]
pub struct IngestReportArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub task_id: Option<String>,
    #[arg(long)]
    pub manifest: Option<String>,
    #[arg(long)]
    pub attempt: Option<usize>,
}

#[derive(Args)]
pub struct RecordGateArgs {
    #[arg(long)]
    pub task_id: String,
    #[arg(long)]
    pub command: String,
    #[arg(long)]
    pub status: String,
    #[arg(long)]
    pub log: Option<PathBuf>,
}

#[derive(Args)]
pub struct OrchReviewArgs {
    #[arg(long)]
    pub task_id: String,
    #[arg(long)]
    pub decision: String,
    #[arg(long)]
    pub note: String,
    #[arg(long)]
    pub reviewer: Option<String>,
    #[arg(long)]
    pub commit: Option<String>,
}

#[derive(Args)]
pub struct ShowTaskArgs {
    #[arg(long)]
    pub task_id: String,
}

#[derive(Args)]
pub struct ListOrchestrationArgs {
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub include_closed: bool,
}

#[derive(Args)]
pub struct IngestTaskArgs {
    pub task_id: String,
    #[arg(long, default_value = "engram")]
    pub source: String,
}

pub fn command_for_completions() -> clap::Command {
    Cli::command()
}
