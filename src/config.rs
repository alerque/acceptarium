// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::{Result, StorageDriver};

use crate::cli::Cli;

use config::{Config as LayeredConfig, Environment};

use clap::ValueEnum;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Config {
    debug: bool,
    quiet: bool,
    verbose: bool,
    project: String,
    storage: StorageDriver,
}

impl Config {
    pub fn new(args: &Cli) -> Result<Self> {
        let mut builder = LayeredConfig::builder()
            .set_default("debug", false)?
            .set_default("quiet", false)?
            .set_default("verbose", false)?
            .set_default("project", "./")?
            .set_default("storage", "filesystem")?
            .add_source(Environment::with_prefix("acceptarium"));
        if args.debug {
            builder = builder.set_override("debug", true)?;
        }
        if args.quiet {
            builder = builder.set_override("quiet", true)?;
        }
        if args.verbose {
            builder = builder.set_override("verbose", true)?;
        }
        if let Some(project) = args.project.to_str() {
            builder = builder.set_override("project", project)?;
        }
        if let Some(storage) = &args.storage {
            let storage = storage.to_possible_value().unwrap();
            builder = builder.set_override("storage", storage.get_name())?;
        }
        let sources = builder.build()?;
        let config = sources.try_deserialize()?;
        Ok(config)
    }
}
