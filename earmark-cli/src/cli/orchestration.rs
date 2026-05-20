use std::path::PathBuf;

use clap::{Args, Subcommand};

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
    Timeline(ShowTaskArgs),
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
    pub task_id: String,
    #[arg(long)]
    pub decision: String,
    #[arg(long)]
    pub comment: Option<String>,
}

#[derive(Args)]
pub struct ShowTaskArgs {
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
