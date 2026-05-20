use clap::{Args, Subcommand};

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
