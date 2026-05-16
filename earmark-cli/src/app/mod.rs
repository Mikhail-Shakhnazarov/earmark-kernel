use clap_complete::{generate, shells};

use crate::cli::*;

pub(crate) mod common;
pub(crate) use common::CliError;
mod bootstrap;
pub(crate) mod commands;
pub(crate) use commands::declarations::{
    explain_declaration_file, register_declaration_file, resolve_version_ref,
    resolve_workflow_version_ref, validate_declaration_file,
};
pub(crate) mod emitter;
pub(crate) mod graph;
pub(crate) mod listing;
pub(crate) mod loaders;
pub(crate) mod reports;
pub(crate) mod resolve;
pub(crate) mod scaffold;
pub(crate) mod suggestions;
mod dispatch;

// Re-exports for convenience
pub(crate) use emitter::emit;
pub(crate) use graph::build_run_graph;
pub(crate) use listing::{
    list_assignments, list_assignments_by_run, list_change_sets, list_change_sets_by_run,
    list_failures, list_failures_by_run, list_handoffs, list_handoffs_by_run,
    list_provider_records_by_run, list_run_records, load_run_record_by_id, run_related_artifacts,
};
pub(crate) use loaders::{
    change_set_synthetic_marker, load_change_set_by_id, load_current_assignment_by_id,
    load_failure_by_id, load_handoff_by_id, load_relation_object_by_id,
};
pub(crate) use reports::{generate_handoff_report, generate_run_report, generate_system_report};
pub(crate) use resolve::{resolve_object_ref, resolve_run_id, resolve_optional_run_id, resolve_system_version_ref};
pub(crate) use scaffold::{
    collect_paths_with_extensions, mirror_surface, scaffold_declaration,
};
pub(crate) use suggestions::{
    next_commands_for_assignment, next_commands_for_change_set, next_commands_for_failure,
    next_commands_for_handoff, next_commands_for_run,
};

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

    let bootstrapped = bootstrap::bootstrap(&cli)?;
    let ctx = common::CommandContext {
        store: &bootstrapped.store,
        index: &bootstrapped.index,
        config: &bootstrapped.config,
        as_json: bootstrapped.as_json,
        provider_registry: &bootstrapped.provider_registry,
    };

    let command_name = common::command_family_name(&cli.command);
    let started = std::time::Instant::now();

    tracing::debug!(root = %bootstrapped.root.display(), command = %command_name, "starting command");

    let result = dispatch::dispatch(&ctx, cli);
    crate::metrics::record_command_result(command_name, result.is_ok(), started.elapsed());
    result
}
