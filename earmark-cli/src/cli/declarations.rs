use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

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
