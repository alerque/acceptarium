// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;
use crate::{Assets, Config, DumpFormat, ExportFormat};

use std::fmt::Write;

use serde::Serialize;
use serde_hjson::ser::to_string as to_hjson_string;
use serde_json::to_string_pretty as to_json_string;
use serde_xml_rs::to_string as to_xml_string;
use serde_yaml::to_string as to_yaml_string;
use toml::to_string as to_toml_string;

pub fn export(config: &Config, assets: &Assets) -> Result<String> {
    let format = &config.export_format;
    let mut output = String::new();
    for (_, asset) in assets {
        log::debug!("Attempting to render {} as {:?}", &asset.id(), &format);
        let template = match format {
            ExportFormat::HLedger => &config.templates.hledger,
            ExportFormat::LedgerCli => &config.templates.ledger_cli,
            ExportFormat::BeanCount => &config.templates.beancount,
            ExportFormat::Custom => &config.templates.custom,
        };
        let transaction = template.render(config, asset)?;
        writeln!(output, "{transaction}")?;
    }
    Ok(output)
}

pub fn dump<T: Serialize>(config: &Config, data: &T) -> Result<String> {
    let format = &config.dump_format;
    log::debug!("Attempting to dump data as {:?}", format);
    let output = match format {
        DumpFormat::JSON => to_json_string(data).unwrap_or_default(),
        DumpFormat::TOML => to_toml_string(data).unwrap_or_default(),
        DumpFormat::YAML => to_yaml_string(data).unwrap_or_default(),
        DumpFormat::HJSON => to_hjson_string(data).unwrap_or_default(),
        DumpFormat::XML => to_xml_string(data).unwrap_or_default(),
    };
    Ok(output)
}
