// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Config;
use crate::error::Error;
use crate::error::InvalidAssetIdSnafu;
use crate::{ASSET_ID_CHARS, ASSET_ID_LEN};

use blake3::Hash as Blake3;
use glob::Pattern;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as SerializableValue, to_value};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum OperationMode {
    JustCheck,
    JustRun,
    #[default]
    CheckAndRun,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TemplateString(String);

impl TemplateString {
    pub fn render(&self, config: &Config, asset: &Asset) -> Result<String> {
        let mut template = String::new();
        let mut output = self.0.clone();
        let max_iterations = 10;
        let mut tera = Tera::default();
        let context = build_context(config, asset)?;
        let ctx = Context::from_value(context)?;
        for i in 0..max_iterations {
            log::info!("Rendering Tera template, pass {i}");
            if output == template {
                break;
            }
            template = output.clone();
            output = tera.render_str(&template, &ctx)?;
        }
        Ok(output)
    }
}

fn build_context(config: &Config, asset: &Asset) -> Result<SerializableValue> {
    let mut context = Map::new();
    context.insert("config".to_string(), to_value(config)?);
    context.insert("asset".to_string(), to_value(asset)?);
    Ok(SerializableValue::Object(context))
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Transaction {
    pub payee: Option<String>,
    pub date: Option<String>,
    pub datetime: Option<String>,
    pub total: Option<f64>,
    pub payment_type: Option<String>,
    pub payment_identifier: Option<String>,
    pub category: Option<String>,
    pub invoice_number: Option<String>,
    pub items: Option<Vec<TransactionItem>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TransactionItem {
    pub description: Option<String>,
    pub quantity: Option<f64>,
    pub amount: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Blake3Sum(Blake3);

impl Blake3Sum {
    pub fn new(hash: Blake3) -> Self {
        Self(hash)
    }
}

impl From<Blake3> for Blake3Sum {
    fn from(hash: Blake3) -> Self {
        Self(hash)
    }
}

impl Display for Blake3Sum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_hex())
    }
}

impl Serialize for Blake3Sum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_hex())
    }
}

impl<'de> Deserialize<'de> for Blake3Sum {
    fn deserialize<D>(deserializer: D) -> Result<Blake3Sum, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Blake3::from_hex(&s)
            .map(Blake3Sum)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct GlobPattern(Pattern);

// impl GlobPattern {
//     pub fn new(pattern: &str) -> Result<Self> {
//         Ok(Self(Pattern::new(pattern)?))
//     }
// }

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

impl TryFrom<String> for AssetId {
    type Error = Error;
    fn try_from(s: String) -> Result<Self> {
        Self::parse(&s)
    }
}

impl TryFrom<&String> for AssetId {
    type Error = Error;
    fn try_from(s: &String) -> Result<Self> {
        Self::parse(s)
    }
}

impl From<AssetId> for String {
    fn from(id: AssetId) -> Self {
        id.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Asset {
    id: AssetId,
    asset_path: Option<PathBuf>,
    source_fname: Option<PathBuf>,
    blake3: Option<Blake3Sum>,
    ocr: Option<String>,
    transaction: Option<Transaction>,
    #[serde(default)]
    extra: Map<String, SerializableValue>,
}

impl Asset {
    pub fn new(
        asset_path: Option<&Path>,
        source_fname: Option<&Path>,
        blake3: Option<Blake3Sum>,
    ) -> Result<Self> {
        let id = AssetId::new();
        let asset_path = asset_path.map(Into::into);
        let source_fname = source_fname.map(Into::into);
        Ok(Self {
            id,
            asset_path,
            source_fname,
            blake3,
            ocr: None,
            transaction: None,
            extra: Map::new(),
        })
    }
    pub fn id(&self) -> &AssetId {
        &self.id
    }
    pub fn asset_path(&self, project_dir: &Path) -> Option<PathBuf> {
        self.asset_path.to_owned().map(|asset_path| {
            if asset_path.is_absolute() {
                asset_path.clone()
            } else {
                project_dir.join(asset_path)
            }
        })
    }
    pub fn source_fname(&self) -> Option<PathBuf> {
        self.source_fname.to_owned()
    }
    pub fn set_asset_path(&mut self, asset_path: Option<&Path>) {
        self.asset_path = asset_path.map(Into::into);
    }
    pub fn set_source_fname(&mut self, source_fname: Option<&Path>) {
        self.source_fname = source_fname.map(Into::into);
    }
    pub fn blake3(&self) -> Option<&Blake3Sum> {
        self.blake3.as_ref()
    }
    pub fn set_blake3(&mut self, blake3: Option<Blake3Sum>) {
        self.blake3 = blake3;
    }
    pub fn ocr(&self) -> Option<&String> {
        self.ocr.as_ref()
    }
    pub fn set_ocr(&mut self, ocr: Option<String>) {
        self.ocr = ocr;
    }
    pub fn transaction(&self) -> Option<&Transaction> {
        self.transaction.as_ref()
    }
    pub fn set_transaction(&mut self, transaction: Option<Transaction>) {
        self.transaction = transaction;
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let asset_path = self
            .asset_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let blake3 = self
            .blake3
            .as_ref()
            .map(|h| h.to_string())
            .unwrap_or_default();
        write!(f, "{}\t{}\t{}", self.id, asset_path, blake3)
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
