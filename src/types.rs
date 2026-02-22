// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{ASSET_ID_CHARS, ASSET_ID_LEN};

use clap::error::Error as ClapError;
use config::ConfigError;
use glob::{Pattern, PatternError};
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeJsonError;
use snafu::prelude::*;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::{Debug, Display, Formatter};
use std::io::Error as IoError;
use std::path::PathBuf;
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

    #[snafu(display("Process stream error: {source}"))]
    Stream { source: IoError },

    #[snafu(display("IO buffer error in {stream} stream"))]
    Buffer { stream: String },

    #[snafu(display("{message}"))]
    ExternalCommand { message: String },

    #[snafu(display("No storage driver has been configured"))]
    NoStorageConfigured {},

    #[snafu(display("The storage driver`{driver}` requires a valid configuration"))]
    MissingStorageConfig { driver: String },

    #[snafu(display("This build is not compiled with support for the `{driver}` storage driver"))]
    UnsupportedStorage { driver: String },

    #[snafu(display("Invalid glob pattern for `{source}`"))]
    Glob { source: PatternError },

    #[snafu(display("Invalid asset ID: {message}"))]
    InvalidAssetId { message: String },
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

impl From<IoError> for Error {
    fn from(source: IoError) -> Self {
        Error::Stream { source }
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

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub type Args = Vec<OsString>;

// Re-export types also used by clap at build time in runtime modules
pub type StorageDriver = crate::cli::StorageDriver;

#[derive(Clone, Debug)]
pub struct GlobPattern(Pattern);

impl GlobPattern {
    pub fn new(pattern: &str) -> Result<Self> {
        Ok(Self(Pattern::new(pattern)?))
    }
}

impl Default for GlobPattern {
    fn default() -> Self {
        Self(Pattern::new("*").unwrap())
    }
}

impl std::ops::Deref for GlobPattern {
    type Target = Pattern;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for GlobPattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for GlobPattern {
    fn deserialize<D>(deserializer: D) -> Result<GlobPattern, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Pattern::new(&s)
            .map(GlobPattern)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetId(String);

impl AssetId {
    pub fn new() -> Self {
        let id = crate::new_id();
        Self(id)
    }
}

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AssetId {
    pub fn parse(s: &str) -> Result<Self> {
        if s.len() != ASSET_ID_LEN {
            return InvalidAssetIdSnafu {
                message: format!("Asset ID must be exactly {} characters", ASSET_ID_LEN),
            }
            .fail();
        }
        if !s.chars().all(|c| ASSET_ID_CHARS.contains(&c)) {
            return InvalidAssetIdSnafu {
                message: "Asset ID must only contain alphanumeric characters".to_string(),
            }
            .fail();
        }
        Ok(Self(s.to_string()))
    }
}

impl Serialize for AssetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AssetId {
    fn deserialize<D>(deserializer: D) -> Result<AssetId, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        AssetId::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Asset {
    pub id: AssetId,
    pub file: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub struct Assets {
    inner: HashMap<AssetId, Asset>,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, asset: Asset) {
        self.inner.insert(asset.id.clone(), asset);
    }

    pub fn get(&self, id: &AssetId) -> Option<&Asset> {
        self.inner.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &Asset)> {
        self.inner.iter()
    }
}

impl std::fmt::Display for Assets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (id, asset) in self.inner.iter() {
            let filename = asset
                .file
                .as_ref()
                .map(|p| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            writeln!(f, "{}\t{}", id, filename)?;
        }
        Ok(())
    }
}
