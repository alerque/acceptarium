// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use acceptarium::actions::instantiate_storage;
use acceptarium::cli::{Cli, Commands, STYLES};
#[cfg(feature = "tui")]
use acceptarium::tui;
use acceptarium::{Config, Result};
use acceptarium::{export, process, run, status, storage};

use clap::{CommandFactory, FromArgMatches};
use flexi_logger::{Logger, LoggerHandle};
use log::LevelFilter;

fn main() {
    let logger = Logger::with(LevelFilter::Error)
        .format_for_stderr(flexi_logger::colored_default_format)
        .log_to_stderr();
    let logger = logger
        .start()
        .unwrap_or_else(|e| panic!("Unable to start logger: {:?}", e));
    if let Err(e) = run(logger) {
        log::error!("{:?}", e);
        std::process::exit(1);
    }
}

fn run(logger: LoggerHandle) -> Result<()> {
    let version = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or_else(|| env!("CARGO_PKG_VERSION"));
    let app = Cli::command().version(version).styles(STYLES);
    let matches = app.get_matches();
    log::debug!("Attempting to parse CLI arguments, {:?}", matches);
    let args = Cli::from_arg_matches(&matches)?;
    log::debug!("Mixing up runtime config from defaults, config file, env vars, and CLI flags");
    log::debug!("CLI Args: {:?}", &args);
    let config = Config::new(&args)?;
    logger.set_new_spec(config.verbosity.into());
    log::debug!("Completed config: {:?}", &config);
    log::debug!("Passing subcommand to matched handler");
    let storage = instantiate_storage(&config)?;
    match Commands::from_arg_matches(&matches)? {
        Commands::Add { files, .. } => storage::add(&config, storage, files),
        Commands::List {
            json, selectors, ..
        } => {
            let assets = storage.select(&selectors)?;
            if json {
                println!("{}", assets.to_json()?);
            } else {
                print!("{}", assets);
            }
            Ok(())
        }
        Commands::Process { selectors, .. } => {
            let assets = storage.select(&selectors)?;
            process::process(&config, storage, assets)
        }
        Commands::Export { selectors, .. } => {
            let assets = storage.select(&selectors)?;
            export::run(&config, assets)
        }
        Commands::Get { id, key, .. } => storage::get(&config, storage, &id, &key),
        Commands::Set { id, key, value } => storage::set(&config, storage, id, &key, &value),
        Commands::Remove { selectors } => {
            let assets = storage.select(&selectors)?;
            storage::remove(&config, storage, assets)
        }
        Commands::Run { name, arguments } => run::run(&config, name, arguments),
        Commands::Status {} => status::run(&config),
        Commands::External(mut args) => {
            let name = args.pop().ok_or("external command without a name")?;
            run::run(&config, name, args)
        }
        #[cfg(feature = "tui")]
        Commands::Tui {} => tui::main(&config),
    }
}
