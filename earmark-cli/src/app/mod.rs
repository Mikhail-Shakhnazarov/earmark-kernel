use clap_complete::{generate, shells};

use crate::cli::*;

pub(crate) mod common;
pub(crate) use common::CliError;
mod bootstrap;
pub(crate) mod commands;
pub(crate) use commands::declarations::{
    register_declaration_file, resolve_version_ref, resolve_workflow_version_ref,
};
mod dispatch;
pub(crate) mod emitter;
mod graph;
pub(crate) mod listing;
mod loaders;
mod reports;
pub(crate) mod resolve;
mod scaffold;
mod suggestions;

// Re-exports for command files that import via crate::app::{...}
pub(crate) use emitter::emit;
pub(crate) use listing::{
    list_assignments_by_run, list_change_sets, list_change_sets_by_run, list_failures_by_run,
    list_handoffs_by_run, load_run_record_by_id,
};
pub(crate) use resolve::{resolve_object_ref, resolve_system_version_ref};
pub(crate) use scaffold::mirror_surface;

pub fn run(cli: Cli) -> Result<(), common::CliError> {
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = command_for_completions();
        match shell {
            CompletionShell::Bash => generate(shells::Bash, &mut cmd, "em", &mut std::io::stdout()),
            CompletionShell::Zsh => generate(shells::Zsh, &mut cmd, "em", &mut std::io::stdout()),
            CompletionShell::Fish => generate(shells::Fish, &mut cmd, "em", &mut std::io::stdout()),
        }
        return Ok(());
    }

    let mut bootstrapped = bootstrap::bootstrap(&cli)?;
    let mut ctx = common::CommandContext {
        store: &bootstrapped.store,
        index: &mut bootstrapped.index,
        config: &bootstrapped.config,
        as_json: bootstrapped.as_json,
        provider_registry: &bootstrapped.provider_registry,
        loaded_provider_plugins: &bootstrapped.loaded_provider_plugins,
        actor: &bootstrapped.actor,
        root: &bootstrapped.root,
    };

    let command_name = common::command_family_name(&cli.command);
    let started = std::time::Instant::now();

    tracing::debug!(root = %bootstrapped.root.display(), command = %command_name, "starting command");

    crate::output::init_context(crate::output::CliContext {
        command_name,
        as_json: ctx.as_json,
    });

    let result = dispatch::dispatch(&mut ctx, cli);
    crate::metrics::record_command_result(command_name, result.is_ok(), started.elapsed());
    result
}
