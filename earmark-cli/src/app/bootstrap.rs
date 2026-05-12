use std::path::PathBuf;
use std::env;
use crate::app::common::{CliError, WorkspaceAccessMode, workspace_access_mode, require_initialized_workspace, BootstrappedServices};
use earmark_store::{GitCanonicalStore, CanonicalStore};
use earmark_index::DerivedIndex;
use earmark_exec::default_provider_registry;
use crate::cli::Cli;
use crate::config::load_config;

pub(crate) fn bootstrap(cli: &Cli) -> Result<BootstrappedServices, CliError> {
    // 1. Load config (handles CLI flag, ENV, and default paths)
    let config = load_config(cli)?;

    // 2. Resolve root
    let root: PathBuf = if let Some(r) = &cli.root {
        r.clone()
    } else if let Ok(r) = env::var("EM_ROOT") {
        PathBuf::from(r)
    } else if let Some(r) = &config.root {
        r.clone()
    } else {
        PathBuf::from(".")
    };

    let store = GitCanonicalStore::new(&root);
    
    let mode = workspace_access_mode(&cli.command);
    
    match mode {
        WorkspaceAccessMode::None => {}
        WorkspaceAccessMode::Init | WorkspaceAccessMode::Write => store.init_layout()?,
        WorkspaceAccessMode::ReadOnly => require_initialized_workspace(&store)?,
    }

    let index = match mode {
        WorkspaceAccessMode::None => None,
        WorkspaceAccessMode::ReadOnly => Some(DerivedIndex::open_existing(&root)?),
        WorkspaceAccessMode::Init | WorkspaceAccessMode::Write => Some(DerivedIndex::open(&root)?),
    };

    let provider_registry = default_provider_registry();
    let as_json = cli.json;

    Ok(BootstrappedServices {
        store,
        index,
        config,
        provider_registry,
        as_json,
        root,
    })
}
