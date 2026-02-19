// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use acceptarium::cli::{Cli, Commands, STYLES};
use acceptarium::config::Config;
use acceptarium::storage::Storage;
use acceptarium::types::{Result, StorageDriver};
use acceptarium::{run, status, storage};

use clap::{CommandFactory, FromArgMatches};

fn main() -> Result<()> {
    let version = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or_else(|| env!("CARGO_PKG_VERSION"));
    let app = Cli::command().version(version).styles(STYLES);
    let matches = app.get_matches();
    let args = Cli::from_arg_matches(&matches).expect("Unable to parse arguments");
    let config = Config::new(&args)?;
    let storage: Box<dyn Storage> = match config.storage {
        Some(StorageDriver::GitAnnex) => {
            Box::new(storage::git_annex::GitAnnexStorage::new(config.clone()))
        }
        Some(StorageDriver::Filesystem) => {
            Box::new(storage::filesystem::FilesystemStorage::new(config.clone()))
        }
        None => Box::new(storage::dummy::DummyStorage::new(config.clone())),
    };
    match Commands::from_arg_matches(&matches)? {
        Commands::Run { name, arguments } => run::run(&config, name, arguments),
        Commands::Status {} => status::run(&config),
        Commands::List {} => storage.list(),
        Commands::External(mut args) => {
            let name = args.pop().ok_or("external command without a name")?;
            run::run(&config, name, args)
        }
    }
}
