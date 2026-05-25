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
    InitExample(InitExampleArgs),
    CaptureGit(CaptureGitArgs),
    IngestManifest(IngestManifestArgs),
    IngestReport(IngestReportArgs),
    RecordGate(RecordGateArgs),
    Review(OrchReviewArgs),
    Show(ShowTaskArgs),
    Timeline(ShowTaskArgs),
    List(ListOrchestrationArgs),
    IngestTask(IngestTaskArgs),
    RecordContext(RecordContextArgs),
    ExplainDispatch(ExplainDispatchArgs),
}

#[derive(Args)]
pub struct InitExampleArgs {
    #[arg(long, help = "Optional path to the example root")]
    pub example_root: Option<PathBuf>,
}

#[derive(Args)]
pub struct CaptureGitArgs {
    #[arg(long)]
    pub task_id: String,
    #[arg(long)]
    pub dispatch_id: Option<String>,
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
    #[arg(long, help = "Optional path to the repository to capture from")]
    pub repo: Option<PathBuf>,
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
    #[arg(long)]
    pub context_id: Option<String>,
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
    pub dispatch_id: Option<String>,
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
    #[arg(long, default_value = "native-json")]
    pub source: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
}
#[derive(Args)]
pub struct RecordContextArgs {
    #[arg(long)]
    pub task_id: String,
    pub path: PathBuf,
}

#[derive(Args)]
pub struct ExplainDispatchArgs {
    #[arg(help = "durable ID of the dispatch or 'latest'")]
    pub dispatch_id: String,
}
