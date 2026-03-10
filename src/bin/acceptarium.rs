// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use acceptarium::cli::{Cli, Commands, STYLES};
use acceptarium::{Config, Result};
use acceptarium::{process, run, status, storage};

use clap::{CommandFactory, FromArgMatches};
use flexi_logger::Logger;

fn main() -> Result<()> {
    let version = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or_else(|| env!("CARGO_PKG_VERSION"));
    let app = Cli::command().version(version).styles(STYLES);
    let matches = app.get_matches();
    let args = Cli::from_arg_matches(&matches).expect("Unable to parse arguments");
    let config = Config::new(&args)?;
    let logger = Logger::with(config.verbosity)
        .format_for_stderr(flexi_logger::colored_default_format)
        .log_to_stderr();
    logger.start()?;
    log::debug!("Args: {:?}", &args);
    log::info!("Mapped defaults, config file, env vars, and CLI flags to runtime configuration");
    log::debug!("Completed config: {:?}", &config);
    log::info!("Passing subcommand to matched handler");
    let result = match Commands::from_arg_matches(&matches)? {
        Commands::Add { files, .. } => storage::add(&config, files),
        Commands::List { json, .. } => storage::list(&config, json),
        Commands::Process { id, .. } => process::process(&config, &id),
        Commands::Get { id, key, .. } => storage::get(&config, &id, &key),
        Commands::Set { id, key, value } => storage::set(&config, id, &key, &value),
        Commands::Remove { id, .. } => storage::remove(&config, &id),
        Commands::Run { name, arguments } => run::run(&config, name, arguments),
        Commands::Status {} => status::run(&config),
        Commands::External(mut args) => {
            let name = args.pop().ok_or("external command without a name")?;
            run::run(&config, name, args)
        }
    };
    if let Err(e) = result {
        log::error!("{:#}", e);
        std::process::exit(1);
    }
    Ok(())
}
