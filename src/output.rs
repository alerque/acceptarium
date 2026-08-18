// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt::Write;

use serde::Serialize;
use serde_hjson::ser::to_string as to_hjson_string;
use serde_json::to_string_pretty as to_json_string;
use serde_xml_rs::to_string as to_xml_string;
use serde_yaml::to_string as to_yaml_string;
use toml::to_string as to_toml_string;

use crate::Result;
use crate::{Assets, Config, ExportFormat, InfoFormat};

fn serialize_to_string<T: Serialize>(format: InfoFormat, data: &T) -> Result<String> {
    match format {
        InfoFormat::JSON => to_json_string(data).map_err(Into::into),
        InfoFormat::TOML => to_toml_string(data).map_err(Into::into),
        InfoFormat::YAML => to_yaml_string(data).map_err(Into::into),
        InfoFormat::HJSON => to_hjson_string(data).map_err(Into::into),
        InfoFormat::XML => to_xml_string(data).map_err(Into::into),
    }
}

pub fn export(config: &Config, format: ExportFormat, assets: &Assets) -> Result<String> {
    let mut output = String::new();
    for (_, asset) in assets {
        log::debug!("Attempting to render {} as {:?}", asset.id(), format);
        let template = match format {
            ExportFormat::HLedger => &config.templates.hledger,
            ExportFormat::LedgerCli => &config.templates.ledger_cli,
            ExportFormat::Beancount => &config.templates.beancount,
            ExportFormat::Custom => &config.templates.custom,
        };
        let transaction = template.render(config, asset)?;
        writeln!(output, "{transaction}")?;
    }
    Ok(output)
}

pub fn dump<T: Serialize>(format: InfoFormat, data: &T) -> Result<String> {
    log::debug!("Attempting to dump data as {:?}", format);
    serialize_to_string(format, data)
}
