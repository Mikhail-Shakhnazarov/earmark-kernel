mod handlers;

use crate::app::common::{CliError, CommandContext};
use crate::cli::*;

pub fn dispatch(ctx: &CommandContext, cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Init => crate::app::commands::init_doctor::handle_init(ctx)?,
        Commands::Doctor(args) => crate::app::commands::init_doctor::handle_doctor(ctx, &args)?,
        Commands::System(command) => crate::app::commands::system::handle(ctx, &command)?,
        Commands::Deposit(args) => crate::app::commands::deposit::handle(ctx, &args)?,
        Commands::Query(args) => crate::app::commands::query::handle(ctx, &args)?,
        Commands::Review(args) => crate::app::commands::review::handle(ctx, &args)?,
        Commands::Workflow(command) => crate::app::commands::workflow::handle(ctx, &command)?,
        Commands::Context(command) => crate::app::commands::context::handle(ctx, &command)?,
        Commands::Undo(command) => crate::app::commands::undo::handle(ctx, &command)?,
        Commands::Run(command) => handlers::handle_run(ctx, command)?,
        Commands::Declare(command) => handlers::handle_declare(ctx, command)?,
        Commands::Assignment(command) => handlers::handle_assignment(ctx, command)?,
        Commands::ChangeSet(command) => handlers::handle_change_set(ctx, command)?,
        Commands::Handoff(command) => handlers::handle_handoff(ctx, command)?,
        Commands::Failure(command) => handlers::handle_failure(ctx, command)?,
        Commands::Audit(command) => handlers::handle_audit(ctx, command)?,
        Commands::Report(command) => handlers::handle_report(ctx, command)?,
        Commands::Provider(command) => handlers::handle_provider(ctx, command)?,
        Commands::Completions { .. } => {}
        Commands::Status => handlers::handle_status(ctx)?,
        Commands::Relation(command) => handlers::handle_relation(ctx, command)?,
        Commands::StandingRequest(command) => handlers::handle_standing_request(ctx, command)?,
        Commands::Orchestration(command) => {
            crate::app::commands::orchestration::handle(ctx, &command)?
        }
    }
    Ok(())
}
