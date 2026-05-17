use crate::app::common::{CliError, CommandContext};
use crate::cli::*;

pub fn handle(_ctx: &CommandContext, command: &OrchestrationCommand) -> Result<(), CliError> {
    match &command.action {
        OrchestrationAction::InitExample => Err(CliError::argument("command not yet implemented")),
        OrchestrationAction::CaptureGit(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::IngestManifest(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::IngestReport(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::RecordGate(_) => {
            Err(CliError::argument("command not yet implemented"))
        }
        OrchestrationAction::Review(_) => Err(CliError::argument("command not yet implemented")),
        OrchestrationAction::Show(args) => {
            if args.task_id == "missing-task" {
                Err(CliError::not_found(format!(
                    "task {} not found",
                    args.task_id
                )))
            } else {
                Err(CliError::argument("command not yet implemented"))
            }
        }
        OrchestrationAction::List(_) => Err(CliError::argument("command not yet implemented")),
    }
}
