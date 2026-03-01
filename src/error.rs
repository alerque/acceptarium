// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::AssetId;

use clap::error::Error as ClapError;
use config::ConfigError;
use glob::PatternError;
use serde_json::Error as SerdeJsonError;
use snafu::Snafu;
use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};
use std::io::Error as IoError;
use std::path::StripPrefixError;
use toml::de::Error as DeserializeError;
use toml::ser::Error as SerializeError;
use which::Error as WhichError;

#[derive(Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Configuration error: {source}"))]
    Config { source: ConfigError },

    #[snafu(display("CLI argument error: {source}"))]
    Clap { source: ClapError },

    #[snafu(display("JSON serialization error: {source}"))]
    SerdeJson { source: SerdeJsonError },

    #[snafu(display("Which error: {source}"))]
    Which { source: WhichError },

    #[snafu(display("IO buffer error in {stream} stream"))]
    Buffer { stream: String },

    #[snafu(display("{message}"))]
    ExternalCommand { message: String },

    #[snafu(display("{message}"))]
    ConfigKeyValue { message: String },

    #[snafu(display("No storage driver has been configured"))]
    NoStorageConfigured {},

    #[snafu(display("The storage driver`{driver}` requires a valid configuration"))]
    MissingStorageConfig { driver: String },

    #[snafu(display("This build is not compiled with support for the `{driver}` storage driver"))]
    UnsupportedStorage { driver: String },

    #[snafu(display("Invalid glob pattern for `{source}`"))]
    Glob { source: PatternError },

    #[snafu(display("Unable to convert non-Unicode paths"))]
    NonUnicodePath {},

    #[snafu(display("Unable determine current executable path"))]
    CurrentExecutable { source: IoError },

    #[snafu(display("Invalid asset ID: {message}"))]
    InvalidAssetId { message: String },

    #[snafu(display("IO error: {source}"))]
    Io { source: IoError },

    #[snafu(display("Filesystem error: {message}"))]
    Filesystem { message: String },

    #[snafu(display("Deserialize error: {source}"))]
    Deserialize { source: DeserializeError },

    #[snafu(display("Serialize error: {source}"))]
    Serialize { source: SerializeError },

    #[snafu(display("Unable te strip prefix: {source}"))]
    StripPrefix { source: StripPrefixError },

    #[snafu(display("The checksum is already used by '{id}'"))]
    AssetHashExists { id: AssetId },

    #[snafu(display("The asset ID '{id}' is not in the storage"))]
    UnknownAssetId { id: AssetId },

    #[snafu(display("The field '{key}' is not a known meta data key"))]
    UnknownMetaKey { key: String },
}

// Clap CLI errors are reported using the Debug trait, but Snafu sets up the Display trait.
// So we delegate. c.f. https://github.com/shepmaster/snafu/issues/110
impl Debug for Error {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        Display::fmt(self, fmt)
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Self {
        unreachable!();
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

impl From<SerdeJsonError> for Error {
    fn from(source: SerdeJsonError) -> Self {
        Error::SerdeJson { source }
    }
}

impl From<WhichError> for Error {
    fn from(source: WhichError) -> Self {
        Error::Which { source }
    }
}

impl From<DeserializeError> for Error {
    fn from(source: DeserializeError) -> Self {
        Error::Deserialize { source }
    }
}

impl From<SerializeError> for Error {
    fn from(source: SerializeError) -> Self {
        Error::Serialize { source }
    }
}

impl From<IoError> for Error {
    fn from(source: IoError) -> Self {
        Error::Io { source }
    }
}

impl From<&str> for Error {
    fn from(source: &str) -> Self {
        Error::ExternalCommand {
            message: source.to_string(),
        }
    }
}

impl From<PatternError> for Error {
    fn from(source: PatternError) -> Self {
        Error::Glob { source }
    }
}

impl From<StripPrefixError> for Error {
    fn from(source: StripPrefixError) -> Self {
        Error::StripPrefix { source }
    }
}
