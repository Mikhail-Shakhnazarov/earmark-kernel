use std::path::PathBuf;

use clap::{Args, Subcommand};

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
    Show { run_id: String },
    List,
    Timeline { run_id: String },
    Artifacts { run_id: String },
    Explain { run_id: String },
    Graph { run_id: String },
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
