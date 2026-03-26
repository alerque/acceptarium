// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "git-annex")]
use crate::ANNEX_META_PREFIX;
use crate::error::{
    DeserializeSnafu, HashSnafu, InvalidAssetIdSnafu, SerdeHjsonSnafu, SerdeJsonSnafu,
    SerdeXmlSnafu, SerdeYamlSnafu, UnknownMetaKeySnafu,
};
use crate::{ASSET_ID_CHARS, ASSET_ID_LEN};
use crate::{DumpFormat, Transaction};
use crate::{Error, Result};

use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};

use blake3::Hash as Blake3;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, from_value, to_value};
use snafu::ResultExt;
use snafu::ensure;
use struct_field_names_as_array::FieldNamesAsSlice;

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

#[derive(Clone, Debug, Deserialize, Serialize, FieldNamesAsSlice)]
pub struct Asset {
    id: AssetId,
    blake3: Option<Blake3Sum>,
    asset_path: Option<PathBuf>,
    transaction: Option<Transaction>,
    ocr: Option<String>,
    source_fname: Option<PathBuf>,
    #[field_names_as_slice(skip)]
    #[serde(default)]
    extra: Map<String, Value>,
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
            blake3,
            asset_path,
            transaction: None,
            ocr: None,
            source_fname,
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

    pub fn set_field(&mut self, format: DumpFormat, key: &str, value: &str) -> Result<()> {
        let fields = Asset::FIELD_NAMES_AS_SLICE;
        ensure!(
            fields.contains(&key),
            UnknownMetaKeySnafu {
                key: key.to_string(),
            }
        );
        let json_value = parse_value(format, value)?;
        match key {
            "blake3" => {
                let hash = blake3::Hash::from_hex(value).context(HashSnafu {})?;
                self.blake3 = Some(Blake3Sum::new(hash));
            }
            "asset_path" => {
                self.asset_path = from_value(json_value)?;
            }
            "source_fname" => {
                self.source_fname = from_value(json_value)?;
            }
            "transaction" => {
                self.transaction = from_value(json_value)?;
            }
            "ocr" => {
                self.ocr = from_value(json_value)?;
            }
            _ => {
                return UnknownMetaKeySnafu {
                    key: key.to_string(),
                }
                .fail();
            }
        }
        Ok(())
    }

    pub fn get_field(&self, format: DumpFormat, key: &str) -> Result<String> {
        let value = match key {
            "id" => to_value(&self.id)?,
            "blake3" => to_value(&self.blake3)?,
            "asset_path" => to_value(&self.asset_path)?,
            "source_fname" => to_value(&self.source_fname)?,
            "transaction" => to_value(&self.transaction)?,
            "ocr" => to_value(&self.ocr)?,
            _ => {
                return UnknownMetaKeySnafu {
                    key: key.to_string(),
                }
                .fail();
            }
        };
        crate::output::dump(format, &value)
    }

    #[cfg(feature = "git-annex")]
    pub fn to_annex_metadata(&self) -> Vec<String> {
        let mut result = Vec::new();
        let p = format!("{}.", ANNEX_META_PREFIX);
        if let Ok(value) = to_value(self)
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
    fn append_metadata(&self, result: &mut Vec<String>, prefix: &str, key: &str, value: &Value) {
        match value {
            Value::Null => {}
            Value::String(s) => {
                let field_key = if key == "ocr" {
                    key.to_string()
                } else {
                    format!("{}{}", prefix, key)
                };
                result.push(format!("{}={}", field_key, s));
            }
            Value::Number(n) => {
                result.push(format!("{}{}={}", prefix, key, n));
            }
            Value::Object(obj) => {
                for (k, v) in obj {
                    self.append_metadata(result, prefix, &format!("{}.{}", key, k), v);
                }
            }
            Value::Array(arr) => {
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
            fields: Map<String, Value>,
        }
        let annex: AnnexMetadata = serde_json::from_str(json)?;
        let prefix = format!("{}.", ANNEX_META_PREFIX);
        // Build a JSON object from the prefixed fields
        let mut asset_obj = Map::new();
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
                arr.first().cloned().unwrap_or(Value::Null)
            } else {
                values
            };
            // Reconstruct nested structure from dotted keys
            insert_nested_value(&mut asset_obj, local_key, value);
        }
        // Deserialize the reconstructed object
        let asset: Asset = from_value(Value::Object(asset_obj))?;
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

fn parse_value(format: DumpFormat, value: &str) -> Result<Value> {
    let parse_result = match format {
        DumpFormat::JSON => serde_json::from_str(value).context(SerdeJsonSnafu {}),
        DumpFormat::YAML => serde_yaml::from_str(value).context(SerdeYamlSnafu {}),
        DumpFormat::TOML => toml::from_str::<Value>(value).context(DeserializeSnafu {}),
        DumpFormat::HJSON => serde_hjson::from_str(value).context(SerdeHjsonSnafu {}),
        DumpFormat::XML => serde_xml_rs::from_str(value).context(SerdeXmlSnafu {}),
    };
    parse_result.or_else(|_| Ok(Value::String(value.to_string())))
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
        let mut items: Vec<_> = self.inner.iter().collect();
        items.sort_by(|a, b| {
            let datetime_a = a.1.transaction.as_ref().and_then(|t| t.datetime.as_ref());
            let datetime_b = b.1.transaction.as_ref().and_then(|t| t.datetime.as_ref());
            match (datetime_a, datetime_b) {
                (Some(d_a), Some(d_b)) => d_a.cmp(d_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => {
                    let fname_a =
                        a.1.source_fname
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str());
                    let fname_b =
                        b.1.source_fname
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str());
                    fname_a.cmp(&fname_b)
                }
            }
        });
        items.into_iter()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.inner)
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&AssetId, &Asset) -> bool,
    {
        self.inner.retain(|id, asset| f(id, asset))
    }
}

impl IntoIterator for Assets {
    type Item = Asset;
    type IntoIter = std::collections::hash_map::IntoValues<AssetId, Asset>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_values()
    }
}

impl<'a> IntoIterator for &'a Assets {
    type Item = (&'a AssetId, &'a Asset);
    type IntoIter = std::collections::hash_map::Iter<'a, AssetId, Asset>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
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
fn insert_nested_value(obj: &mut Map<String, Value>, key: &str, value: Value) {
    let numeric_fields = ["total", "quantity", "amount"];
    let value = if let Some(s) = value.as_str() {
        if numeric_fields.iter().any(|&field| key.ends_with(field)) {
            s.parse::<f64>()
                .map(|n| Value::Number(serde_json::Number::from_f64(n).unwrap()))
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
            .or_insert_with(|| Value::Object(Map::new()));
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
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Some(arr_vec) = arr.as_array_mut() {
                while arr_vec.len() <= idx {
                    arr_vec.push(Value::Object(Map::new()));
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
