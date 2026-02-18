// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use clap::error::Error as ClapError;
use config::ConfigError;
use serde_json::Error as SerdeJsonError;
use snafu::prelude::*;
use std::fmt::{Debug, Display, Formatter};
use subprocess::PopenError;

#[derive(Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Configuration error: {source}"))]
    Config { source: ConfigError },

    #[snafu(display("CLI argument error: {source}"))]
    Clap { source: ClapError },

    #[snafu(display("Process spawning error: {source}"))]
    Popen { source: PopenError },

    #[snafu(display("JSON serialization error: {source}"))]
    SerdeJson { source: SerdeJsonError },
}

// Clap CLI errors are reported using the Debug trait, but Snafu sets up the Display trait.
// So we delegate. c.f. https://github.com/shepmaster/snafu/issues/110
impl Debug for Error {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        Display::fmt(self, fmt)
    }
}

impl From<ConfigError> for Error {
    fn from(source: ConfigError) -> Self {
        Error::Config { source }
    }
}

impl From<ClapError> for Error {
    fn from(source: ClapError) -> Self {
        Error::Clap { source }
    }
}

impl From<PopenError> for Error {
    fn from(source: PopenError) -> Self {
        Error::Popen { source }
    }
}

impl From<SerdeJsonError> for Error {
    fn from(source: SerdeJsonError) -> Self {
        Error::SerdeJson { source }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// Re-export types also used by clap at build time in runtime modules
pub type StorageDriver = crate::cli::StorageDriver;
