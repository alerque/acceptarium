// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use snafu::prelude::*;
use std::fmt::{Debug, Display, Formatter};

#[derive(Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Configuration error: {source}"))]
    Config { source: config::ConfigError },

    #[snafu(display("CLI argument error: {source}"))]
    Clap { source: clap::Error },
}

// Clap CLI errors are reported using the Debug trait, but Snafu sets up the Display trait.
// So we delegate. c.f. https://github.com/shepmaster/snafu/issues/110
impl Debug for Error {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        Display::fmt(self, fmt)
    }
}

impl From<config::ConfigError> for Error {
    fn from(source: config::ConfigError) -> Self {
        Error::Config { source }
    }
}

impl From<clap::error::Error> for Error {
    fn from(source: clap::error::Error) -> Self {
        Error::Clap { source }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
