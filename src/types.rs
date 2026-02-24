// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::error::Error;
use crate::error::InvalidAssetIdSnafu;
use crate::{ASSET_ID_CHARS, ASSET_ID_LEN};

use glob::Pattern;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

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
        let id = nanoid!(ASSET_ID_LEN, &ASSET_ID_CHARS);
        Self(id.to_string())
    }

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

impl Default for AssetId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    id: AssetId,
    file: Option<PathBuf>,
    source_fname: Option<PathBuf>,
}

impl Asset {
    pub fn new(file: Option<&PathBuf>, source_fname: Option<&PathBuf>) -> Result<Self> {
        let id = AssetId::new();
        Ok(Self {
            id,
            file: file.cloned(),
            source_fname: source_fname.cloned(),
        })
    }
    pub fn id(&self) -> &AssetId {
        &self.id
    }
    pub fn file(&self) -> Option<&PathBuf> {
        self.file.as_ref()
    }
    pub fn source_fname(&self) -> Option<&PathBuf> {
        self.source_fname.as_ref()
    }
    pub fn set_file(&mut self, file: Option<&PathBuf>) {
        self.file = file.cloned();
    }
    pub fn set_source_fname(&mut self, source_fname: Option<&PathBuf>) {
        self.source_fname = source_fname.cloned();
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file = self
            .file
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        write!(f, "{}\t{}", self.id, file)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Assets {
    inner: HashMap<AssetId, Asset>,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, asset: Asset) {
        self.inner.insert(asset.id.clone(), asset);
    }

    pub fn get(&self, id: &AssetId) -> Option<&Asset> {
        self.inner.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &Asset)> {
        self.inner.iter()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.inner)
    }
}

impl Display for Assets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for asset in self.inner.values() {
            writeln!(f, "{}", asset)?;
        }
        Ok(())
    }
}
