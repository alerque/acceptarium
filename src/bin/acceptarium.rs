// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use clap::{CommandFactory, FromArgMatches};

use acceptarium::cli::{Cli, Commands, STYLES};
use acceptarium::config::Config;
use acceptarium::status;
use acceptarium::types::Result;

fn main() -> Result<()> {
    let version = option_env!("VERGEN_GIT_DESCRIBE").unwrap_or_else(|| env!("CARGO_PKG_VERSION"));
    let app = Cli::command().version(version).styles(STYLES);
    let matches = app.get_matches();
    let args = Cli::from_arg_matches(&matches).expect("Unable to parse arguments");
    let config = Config::new(&args)?;
    let subcommand = Commands::from_arg_matches(&matches)?;
    match subcommand {
        Commands::Status {} => status::run(&config),
    }
}
