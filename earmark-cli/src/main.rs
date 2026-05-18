mod app;
mod cli;
mod config;
mod logging;
mod metrics;
mod output;

use clap::Parser;
use cli::Cli;
use config::{load_config, resolve_json, resolve_json_early, resolve_log_level};

fn pre_scan_json_flag() -> bool {
    let raw_args: Vec<String> = std::env::args().collect();
    raw_args.iter().any(|a| a == "--json")
}

fn main() {
    let as_json_early = pre_scan_json_flag()
        || std::env::var("EM_JSON")
            .is_ok_and(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"));

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                e.exit();
            }
            if as_json_early {
                output::emit_error_envelope(&e.to_string());
            } else {
                let _ = e.print();
            }
            std::process::exit(1);
        }
    };
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
            output::emit_error_envelope_with_kind(&error.to_string(), error.kind_str());
        } else {
            eprintln!("{}", error);
        }
        std::process::exit(1);
    }
}
