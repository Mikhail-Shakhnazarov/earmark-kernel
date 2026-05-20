use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ProviderCommand {
    #[command(subcommand)]
    pub action: ProviderAction,
}

#[derive(Subcommand)]
pub enum ProviderAction {
    Capabilities,
}
