// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::Result;

use crate::cli::Cli;

use config::{Config as LayeredConfig, Environment};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Config {
    debug: bool,
    quiet: bool,
    verbose: bool,
    project: String,
}

impl Config {
    pub fn new(args: &Cli) -> Result<Self> {
        let mut builder = LayeredConfig::builder()
            .set_default("debug", false)?
            .set_default("quiet", false)?
            .set_default("verbose", false)?
            .set_default("project", "./")?
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
        let sources = builder.build()?;
        let config = sources.try_deserialize()?;
        Ok(config)
    }
}
