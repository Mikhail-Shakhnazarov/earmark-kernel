mod app;
mod cli;
mod config;
mod logging;
mod metrics;
mod output;

use clap::Parser;
use cli::Cli;
use config::{load_config, resolve_json, resolve_json_early, resolve_log_level};

fn main() {
    let cli = Cli::parse();
    let as_json_early = resolve_json_early(&cli);

    let config = match load_config(&cli) {
        Ok(config) => config,
        Err(error) => {
            if as_json_early {
                output::emit_error_envelope(&error.to_string());
            } else {
                eprintln!("{}", error);
            }
            std::process::exit(1);
        }
    };

    let as_json = resolve_json(&cli, &config);
    let log_level = resolve_log_level(&cli, &config);
    logging::init_logging(log_level.as_deref());
    tracing::debug!(
        config_path = ?cli.config,
        root_cli = ?cli.root,
        json = as_json,
        log_level = ?log_level,
        "cli bootstrap resolved"
    );

    if let Err(error) = app::run(cli) {
        if as_json {
            output::emit_error_envelope(&error.to_string());
        } else {
            eprintln!("{}", error);
        }
        std::process::exit(1);
    }
}
