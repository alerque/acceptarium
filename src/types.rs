// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "git-annex")]
use crate::ANNEX_META_PREFIX;
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
        log::info!("Rendering template {} for asset {}", &output, &asset);
        let max_iterations = 10;
        let mut tera = Tera::default();
        let context = build_context(config, asset)?;
        let ctx = Context::from_value(context)?;
        for i in 0..max_iterations {
            if output == template {
                break;
            }
            template = output.clone();
            log::debug!("Rendering Tera pass {i} template: {}", &template);
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

    #[cfg(feature = "git-annex")]
    pub fn to_annex_metadata(&self) -> Vec<String> {
        let mut result = Vec::new();
        let p = format!("{}.", ANNEX_META_PREFIX);
        if let Ok(value) = serde_json::to_value(self)
            && let Some(obj) = value.as_object()
        {
            for (key, val) in obj {
                if key == "extra" {
                    continue;
                }
                self.append_metadata(&mut result, &p, key, val);
            }
        }
        result
    }

    #[cfg(feature = "git-annex")]
    fn append_metadata(
        &self,
        result: &mut Vec<String>,
        prefix: &str,
        key: &str,
        value: &serde_json::Value,
    ) {
        match value {
            serde_json::Value::Null => {}
            serde_json::Value::String(s) => {
                let field_key = if key == "ocr" {
                    key.to_string()
                } else {
                    format!("{}{}", prefix, key)
                };
                result.push(format!("{}={}", field_key, s));
            }
            serde_json::Value::Number(n) => {
                result.push(format!("{}{}={}", prefix, key, n));
            }
            serde_json::Value::Object(obj) => {
                for (k, v) in obj {
                    self.append_metadata(result, prefix, &format!("{}.{}", key, k), v);
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    if let Some(obj) = item.as_object() {
                        for (k, v) in obj {
                            self.append_metadata(result, prefix, &format!("{}{}_{}", key, i, k), v);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    #[cfg(feature = "git-annex")]
    pub fn from_annex_metadata_json(json: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct AnnexMetadata {
            fields: Map<String, SerializableValue>,
        }
        let annex: AnnexMetadata = serde_json::from_str(json)?;
        let prefix = format!("{}.", ANNEX_META_PREFIX);
        // Build a JSON object from the prefixed fields
        let mut asset_obj = serde_json::Map::new();
        for (key, values) in annex.fields {
            // Handle OCR special case (no prefix)
            if key == "ocr" {
                if let Some(arr) = values.as_array()
                    && let Some(first) = arr.first()
                {
                    asset_obj.insert(key, first.clone());
                }
                continue;
            }
            // Only process keys with our prefix
            if !key.starts_with(&prefix) {
                continue;
            }
            let local_key = &key[prefix.len()..];
            // Extract first value from array (git-annex stores values as arrays)
            let value = if let Some(arr) = values.as_array() {
                arr.first().cloned().unwrap_or(serde_json::Value::Null)
            } else {
                values
            };
            // Reconstruct nested structure from dotted keys
            insert_nested_value(&mut asset_obj, local_key, value);
        }
        // Deserialize the reconstructed object
        let asset: Asset = serde_json::from_value(serde_json::Value::Object(asset_obj))?;
        Ok(asset)
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

#[cfg(feature = "git-annex")]
fn insert_nested_value(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: serde_json::Value,
) {
    let numeric_fields = ["total", "quantity", "amount"];
    let value = if let Some(s) = value.as_str() {
        if numeric_fields.iter().any(|&field| key.ends_with(field)) {
            s.parse::<f64>()
                .map(|n| serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap()))
                .unwrap_or(value)
        } else {
            value
        }
    } else {
        value
    };
    if let Some(dot_pos) = key.find('.') {
        let (first, rest) = key.split_at(dot_pos);
        let rest = &rest[1..]; // skip the dot
        let nested = obj
            .entry(first.to_string())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(nested_obj) = nested.as_object_mut() {
            insert_nested_value(nested_obj, rest, value);
        }
    } else if key.contains(|c: char| c.is_ascii_digit() && key.contains('_')) {
        // Handle array items like "items0_description"
        if let Some(underscore_pos) = key.rfind('_') {
            let (array_part, field) = key.split_at(underscore_pos);
            let field = &field[1..]; // skip underscore
            // Extract array name and index
            let idx_start = array_part
                .chars()
                .position(|c| c.is_ascii_digit())
                .unwrap_or(0);
            let array_name = &array_part[..idx_start];
            let idx: usize = array_part[idx_start..].parse().unwrap_or(0);
            let arr = obj
                .entry(array_name.to_string())
                .or_insert_with(|| serde_json::Value::Array(Vec::new()));
            if let Some(arr_vec) = arr.as_array_mut() {
                while arr_vec.len() <= idx {
                    arr_vec.push(serde_json::Value::Object(serde_json::Map::new()));
                }
                if let Some(item_obj) = arr_vec[idx].as_object_mut() {
                    item_obj.insert(field.to_string(), value);
                }
            }
        }
    } else {
        obj.insert(key.to_string(), value);
    }
}
