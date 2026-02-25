// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::error::Error;
use crate::error::{InvalidAssetIdSnafu, PathConvSnafu};
use crate::{ASSET_ID_CHARS, ASSET_ID_LEN};

use glob::Pattern;
use nanoid::nanoid;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use snafu::OptionExt;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};

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
    asset_path: Option<RelativePathBuf>,
    source_fname: Option<RelativePathBuf>,
}

impl Asset {
    pub fn new(asset_path: Option<&Path>, source_fname: Option<&Path>) -> Result<Self> {
        let id = AssetId::new();
        let asset_path = asset_path.and_then(|p| RelativePathBuf::from_path(p).ok());
        let source_fname = source_fname.and_then(|p| RelativePathBuf::from_path(p).ok());
        Ok(Self {
            id,
            asset_path,
            source_fname,
        })
    }
    pub fn id(&self) -> &AssetId {
        &self.id
    }
    pub fn asset_path(&self, project_dir: &Path) -> Option<PathBuf> {
        self.asset_path.as_ref().map(|asset_path| {
            let absolute = asset_path.as_relative_path().as_str().starts_with("/");
            let base = if absolute {
                Path::new("/")
            } else {
                project_dir
            };
            asset_path.as_relative_path().to_path(base)
        })
    }
    pub fn source_fname(&self) -> Option<PathBuf> {
        self.source_fname
            .as_ref()
            .map(|p| p.as_relative_path().to_path(""))
    }
    pub fn set_asset_path(&mut self, asset_path: Option<&Path>) -> Result<()> {
        self.asset_path = asset_path
            .map(|path| {
                if path.is_absolute() {
                    path.to_str()
                        .context(PathConvSnafu {})
                        .map(RelativePathBuf::from)
                } else {
                    RelativePathBuf::from_path(path).map_err(Into::into)
                }
            })
            .transpose()?;
        Ok(())
    }
    pub fn set_source_fname(&mut self, source_fname: Option<&Path>) -> Result<()> {
        self.source_fname = source_fname
            .map(|p| RelativePathBuf::from_path(p))
            .transpose()?;
        Ok(())
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let asset_path = self
            .asset_path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default();
        write!(f, "{}\t{}", self.id, asset_path)
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
