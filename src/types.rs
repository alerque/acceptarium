// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "git-annex")]
use crate::Config;

use crate::assets::Asset;
use crate::error::Error;

use std::fmt::Debug;
use std::ops::Deref;

use glob::Pattern;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as SerializableValue, to_value};
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
    pub category: Option<String>,
    pub items: Option<Vec<TransactionItem>>,
    pub total: Option<f64>,
    pub currency: Option<String>,
    pub invoice_number: Option<String>,
    pub payment_type: Option<String>,
    pub payment_identifier: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TransactionItem {
    pub description: Option<String>,
    pub quantity: Option<f64>,
    pub amount: Option<f64>,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct GlobPattern(Pattern);

impl Default for GlobPattern {
    fn default() -> Self {
        Self(Pattern::new("*").unwrap())
    }
}

impl Deref for GlobPattern {
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
