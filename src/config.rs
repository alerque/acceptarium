// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::{Result, StorageDriver};

use crate::cli::Cli;

use clap::ValueEnum;
use config::{Config as LayeredConfig, Environment, File};
use serde::Deserialize;
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
        // Keep track of potential project paths before config is ready so we can load relative configs from there
        let mut discovered_project = "./";
        // Setup default config values
        let mut builder = LayeredConfig::builder()
            .set_default("debug", false)?
            .set_default("quiet", false)?
            .set_default("verbose", false)?
            .set_default("project", discovered_project)?
            .set_default("storage", "filesystem")?;
        // Layer in project level or manually specified config file
        let config_file_path = if let Some(path) = &args.config {
            Some(path.clone())
        } else if let Ok(path) = std::env::var("ACCEPTARIUM_CONFIG") {
            Some(PathBuf::from(path))
        } else {
            None
        };
        if let Some(project) = args.project.to_str() {
            discovered_project = project;
        }
        let project_config = PathBuf::from(discovered_project).join("acceptarium.toml");
        if let Some(path) = config_file_path {
            builder = builder
                .set_default("config", Some(path.to_str()))?
                .add_source(File::from(path.as_path()).required(true));
        } else if project_config.exists() {
            builder = builder
                .set_default("config", Some(project_config.to_str()))?
                .add_source(File::from(project_config.as_path()).required(false));
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
        if let Some(project) = args.project.to_str() {
            builder = builder.set_override("project", project)?;
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
