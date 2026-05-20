use std::path::PathBuf;

use clap::{Args, Subcommand};

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
