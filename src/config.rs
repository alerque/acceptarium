// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::{Result, StorageDriver};

use crate::cli::Cli;

use clap::ValueEnum;
use config::{Config as LayeredConfig, Environment, File};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Config {
    debug: bool,
    quiet: bool,
    verbose: bool,
    project: String,
    config: Option<PathBuf>,
    storage: StorageDriver,
}

impl Config {
    pub fn new(args: &Cli) -> Result<Self> {
        // Keep track of potential project (and hence config file) paths before config is ready so we can load relative configs from there
        let discovered_project: PathBuf = args
            .project
            .clone()
            .or_else(|| {
                env::var("ACCEPTARIUM_PROJECT")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| PathBuf::from("./"));
        // Setup default config values
        let mut builder = LayeredConfig::builder()
            .set_default("debug", false)?
            .set_default("quiet", false)?
            .set_default("verbose", false)?
            .set_default("project", discovered_project.to_str().unwrap())?
            .set_default("storage", "filesystem")?;
        // Layer in project level or manually specified config file
        let project_config: Option<PathBuf> = args
            .config
            .clone()
            .or_else(|| {
                env::var("ACCEPTARIUM_CONFIG")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .or_else(|| {
                let path = discovered_project.join("acceptarium.toml");
                path.exists().then_some(path)
            });
        if let Some(path) = project_config {
            builder = builder
                .set_default("config", path.to_str().unwrap())?
                .add_source(File::from(path.as_path()).required(true));
        }
        // Layer in environment variables
        builder = builder.add_source(Environment::with_prefix("acceptarium"));
        // Layer in command line flags
        if args.debug {
            builder = builder.set_override("debug", true)?;
        }
        if args.quiet {
            builder = builder.set_override("quiet", true)?;
        }
        if args.verbose {
            builder = builder.set_override("verbose", true)?;
        }
        if let Some(storage) = &args.storage {
            let storage = storage.to_possible_value().unwrap();
            builder = builder.set_override("storage", storage.get_name())?;
        }
        // Put it all together and deserialize it to a config struct
        let sources = builder.build()?;
        let config = sources.try_deserialize()?;
        Ok(config)
    }
}
