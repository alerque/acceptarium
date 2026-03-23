// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;
use crate::{Assets, Config, DumpFormat, ExportFormat};

use std::fmt::Write;

use serde_hjson::ser::to_string as to_hjson_string;
use serde_json::to_string_pretty as to_json_string;
use serde_yaml::to_string as to_yaml_string;
use toml::to_string as to_toml_string;
use xml_serde::to_string as to_xml_string;

pub fn export(config: &Config, assets: &Assets) -> Result<String> {
    let format = &config.export_format;
    let mut output = String::new();
    for (_, asset) in assets {
        log::debug!("Attempting to render {} as {:?}", &asset.id(), &format);
        match format {
            ExportFormat::HLedger => {
                log::debug!(
                    "Using template {:?} for format {:?}",
                    &config.template,
                    &format
                );
                let transaction = config.template.render(config, asset)?;
                writeln!(output, "{transaction}")?;
            }
            ExportFormat::LedgerCli => unimplemented!(),
            ExportFormat::BeanCount => unimplemented!(),
        };
    }
    Ok(output)
}

pub fn dump(config: &Config, assets: &Assets) -> Result<String> {
    let format = &config.dump_format;
    log::debug!("Attempting to dump assets as {:?}", format);
    let output = match format {
        DumpFormat::JSON => to_json_string(assets).unwrap_or_default(),
        DumpFormat::TOML => to_toml_string(assets).unwrap_or_default(),
        DumpFormat::YAML => to_yaml_string(assets).unwrap_or_default(),
        DumpFormat::HJSON => to_hjson_string(assets).unwrap_or_default(),
        DumpFormat::XML => to_xml_string(assets).unwrap_or_default(),
    };
    Ok(output)
}
