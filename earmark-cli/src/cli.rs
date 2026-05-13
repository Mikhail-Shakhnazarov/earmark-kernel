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

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Doctor,
    System(SystemCommand),
    Deposit(DepositArgs),
    Query(QueryArgs),
    Review(ReviewArgs),
    Workflow(WorkflowCommand),
    Run(RunCommand),
    Declare(DeclareCommand),
    Assignment(AssignmentCommand),
    ChangeSet(ChangeSetCommand),
    Handoff(HandoffCommand),
    Failure(FailureCommand),
    Context(ContextCommand),
    Audit(AuditCommand),
    Report(ReportCommand),
    Provider(ProviderCommand),
    Completions { shell: CompletionShell },
    Status,
    Relation(RelationCommand),
    StandingRequest(StandingRequestCommand),
    Undo(UndoCommand),
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
        assignment_id: String,
    },
    Explain {
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
        change_set_id: String,
    },
    Explain {
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
        handoff_id: String,
    },
    Explain {
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
        failure_id: String,
    },
    Explain {
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
        relation_id: String,
    },
    Explain {
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
        target_id: String,
        #[arg(short, long)]
        output: PathBuf,
    },
    System {
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
        request_id: String,
    },
    Approve {
        request_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    Reject {
        request_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
    Apply {
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

pub fn command_for_completions() -> clap::Command {
    Cli::command()
}
