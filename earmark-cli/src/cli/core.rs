use std::path::PathBuf;

use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand, ValueEnum};

use super::declarations::DeclareCommand;
use super::execution::{
    AssignmentCommand, ChangeSetCommand, ContextCommand, DepositArgs, FailureCommand,
    HandoffCommand, QueryArgs, RelationCommand, ReviewArgs, RunCommand, SystemCommand, UndoCommand,
    WorkflowCommand,
};
use super::governance::{AuditCommand, StandingRequestCommand};
use super::orchestration::OrchestrationCommand;
use super::provider::ProviderCommand;
use super::reports::ReportCommand;

#[derive(Parser)]
#[command(name = "em")]
#[command(about = "Earmark operator shell")]
pub struct Cli {
    #[arg(long)]
    pub root: Option<PathBuf>,
    #[arg(long, global = true)]
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

#[derive(Args)]
pub struct DoctorArgs {
    #[arg(long, help = "rebuild the derived index from canonical store")]
    pub repair_index: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStability {
    Stable,
    Beta,
    Experimental,
}

impl CommandStability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandDescriptor {
    pub name: &'static str,
    pub stability: CommandStability,
    pub summary: &'static str,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "[STABLE] Initialize a new earmark workspace")]
    Init,
    #[command(about = "[BETA] Diagnose and repair workspace issues")]
    Doctor(DoctorArgs),
    #[command(about = "[STABLE] Manage system registration")]
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
    #[command(about = "[STABLE] Compile context")]
    Context(ContextCommand),
    #[command(about = "[BETA] Audit workspace events")]
    Audit(AuditCommand),
    #[command(about = "[STABLE] Generate reports")]
    Report(ReportCommand),
    #[command(about = "[BETA] Manage providers")]
    Provider(ProviderCommand),
    #[command(about = "[BETA] Generate shell completions")]
    Completions { shell: CompletionShell },
    #[command(name = "commands", about = "[STABLE] Show command catalog")]
    Catalog,
    #[command(about = "[STABLE] Show workspace status")]
    Status,
    #[command(about = "[STABLE] Manage relations")]
    Relation(RelationCommand),
    #[command(about = "[STABLE] Manage standing requests")]
    StandingRequest(StandingRequestCommand),
    #[command(about = "[BETA] Undo a run")]
    Undo(UndoCommand),
    #[command(about = "[STABLE] Manage native orchestration tasks")]
    Orchestration(OrchestrationCommand),
}

impl Commands {
    #[allow(dead_code)]
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
            | Self::Review(_)
            | Self::System(_)
            | Self::Context(_)
            | Self::Relation(_)
            | Self::StandingRequest(_)
            | Self::Catalog => CommandStability::Stable,

            Self::Doctor(_)
            | Self::Declare(_)
            | Self::Audit(_)
            | Self::Provider(_)
            | Self::Completions { .. }
            | Self::Undo(_)
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

pub fn command_catalog() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
            name: "init",
            stability: CommandStability::Stable,
            summary: "Initialize a new earmark workspace",
        },
        CommandDescriptor {
            name: "doctor",
            stability: CommandStability::Beta,
            summary: "Diagnose and repair workspace issues",
        },
        CommandDescriptor {
            name: "system",
            stability: CommandStability::Stable,
            summary: "Manage system registration",
        },
        CommandDescriptor {
            name: "deposit",
            stability: CommandStability::Stable,
            summary: "Deposit an object into the store",
        },
        CommandDescriptor {
            name: "query",
            stability: CommandStability::Stable,
            summary: "Query the object store",
        },
        CommandDescriptor {
            name: "review",
            stability: CommandStability::Stable,
            summary: "Review an object",
        },
        CommandDescriptor {
            name: "workflow",
            stability: CommandStability::Stable,
            summary: "Manage workflows",
        },
        CommandDescriptor {
            name: "run",
            stability: CommandStability::Stable,
            summary: "Manage runs",
        },
        CommandDescriptor {
            name: "declare",
            stability: CommandStability::Beta,
            summary: "Declare and register declarations",
        },
        CommandDescriptor {
            name: "assignment",
            stability: CommandStability::Stable,
            summary: "Manage assignments",
        },
        CommandDescriptor {
            name: "changeset",
            stability: CommandStability::Stable,
            summary: "Manage change sets",
        },
        CommandDescriptor {
            name: "handoff",
            stability: CommandStability::Stable,
            summary: "Manage handoffs",
        },
        CommandDescriptor {
            name: "failure",
            stability: CommandStability::Stable,
            summary: "Manage failures",
        },
        CommandDescriptor {
            name: "context",
            stability: CommandStability::Stable,
            summary: "Compile context",
        },
        CommandDescriptor {
            name: "audit",
            stability: CommandStability::Beta,
            summary: "Audit workspace events",
        },
        CommandDescriptor {
            name: "report",
            stability: CommandStability::Stable,
            summary: "Generate reports",
        },
        CommandDescriptor {
            name: "provider",
            stability: CommandStability::Beta,
            summary: "Manage providers",
        },
        CommandDescriptor {
            name: "completions",
            stability: CommandStability::Beta,
            summary: "Generate shell completions",
        },
        CommandDescriptor {
            name: "commands",
            stability: CommandStability::Stable,
            summary: "Show command catalog",
        },
        CommandDescriptor {
            name: "status",
            stability: CommandStability::Stable,
            summary: "Show workspace status",
        },
        CommandDescriptor {
            name: "relation",
            stability: CommandStability::Stable,
            summary: "Manage relations",
        },
        CommandDescriptor {
            name: "standing-request",
            stability: CommandStability::Stable,
            summary: "Manage standing requests",
        },
        CommandDescriptor {
            name: "undo",
            stability: CommandStability::Beta,
            summary: "Undo a run",
        },
        CommandDescriptor {
            name: "orchestration",
            stability: CommandStability::Stable,
            summary: "Manage native orchestration tasks",
        },
    ]
}

pub fn command_for_completions() -> clap::Command {
    Cli::command()
}
