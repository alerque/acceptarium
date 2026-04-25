// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, to_value};
use tera::{Context, Tera};

#[cfg(feature = "git-annex")]
use crate::Config;
use crate::assets::Asset;
use crate::error::Error;

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
        log::info!("Rendering template for asset {}", &asset);
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
        log::info!("Finished rending template: {}", &output);
        Ok(output)
    }
}

fn build_context(config: &Config, asset: &Asset) -> Result<Value> {
    let mut context = Map::new();
    context.insert("config".to_string(), to_value(config)?);
    context.insert("asset".to_string(), to_value(asset)?);
    Ok(Value::Object(context))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommitMessage {
    template: TemplateString,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub trailers: Vec<String>,
}

impl CommitMessage {
    pub fn new(template: TemplateString) -> Self {
        let subject = None;
        let body = None;
        let trailers = vec![];
        Self {
            template,
            subject,
            body,
            trailers,
        }
    }

    pub fn render(&self) -> Result<String> {
        let mut tera = Tera::default();
        let mut context = Map::new();
        context.insert("msg".to_string(), to_value(self)?);
        let ctx = Context::from_value(Value::Object(context))?;
        let template = &self.template.0;
        let output = tera.render_str(template, &ctx)?;
        Ok(output)
    }
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
