use crate::app::common::{
    require_initialized_workspace, workspace_access_mode, BootstrappedServices, CliError,
    WorkspaceAccessMode,
};
use crate::cli::Cli;
use crate::config::{
    load_config, resolve_actor, resolve_json, resolve_provider_plugin_dirs, resolve_root,
    resolve_trusted_actors,
};
use earmark_exec::{default_provider_registry, register_provider_plugins_from_dirs};
use earmark_index::DerivedIndex;
use earmark_store::{GitCanonicalStore, WorkspaceLayout};

pub(crate) fn bootstrap(cli: &Cli) -> Result<BootstrappedServices, CliError> {
    // 1. Load config (handles CLI flag, ENV, and default paths)
    let config = load_config(cli)?;

    // 2. Resolve root using shared utility
    let root = resolve_root(cli, &config);

    let actor = resolve_actor(cli, &config);
    let trusted_actors = resolve_trusted_actors(&config);
    let store = if trusted_actors.is_empty() {
        GitCanonicalStore::new(&root)
    } else {
        GitCanonicalStore::with_authorized_actors(&root, trusted_actors)
    };

    let mode = workspace_access_mode(&cli.command);

    match mode {
        WorkspaceAccessMode::None => {}
        WorkspaceAccessMode::Init => store.init_layout()?,
        WorkspaceAccessMode::Write
        | WorkspaceAccessMode::ReadOnly
        | WorkspaceAccessMode::RepairIndex => {
            require_initialized_workspace(&store)?;
        }
    }

    let index = match mode {
        WorkspaceAccessMode::None => None,
        WorkspaceAccessMode::ReadOnly => Some(DerivedIndex::open_existing(&root)?),
        WorkspaceAccessMode::RepairIndex => Some(DerivedIndex::open(&root)?),
        WorkspaceAccessMode::Init => Some(DerivedIndex::open(&root)?),
        WorkspaceAccessMode::Write => Some(DerivedIndex::open(&root)?),
    };

    let mut provider_registry = default_provider_registry();
    let plugin_dirs = resolve_provider_plugin_dirs(&root, &config);
    let loaded_provider_plugins =
        register_provider_plugins_from_dirs(&mut provider_registry, &plugin_dirs)
            .map_err(|error| CliError::argument(error.to_string()))?;
    // 3. Resolve JSON using shared utility
    let as_json = resolve_json(cli, &config);

    Ok(BootstrappedServices {
        store,
        index,
        config,
        provider_registry,
        loaded_provider_plugins,
        as_json,
        root,
        actor,
    })
}
