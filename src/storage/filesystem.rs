// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::{Asset, AssetId, Result};

use super::Storage;

use glob::glob;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct FilesystemStorage {
    config: Config,
}

impl FilesystemStorage {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl Storage for FilesystemStorage {
    fn list(&self) -> Result<HashMap<AssetId, Asset>> {
        let mut directory = self.config.project.clone();
        let fs_config = self.config.filesystem.as_ref().unwrap();
        directory.push(fs_config.directory.clone());

        let glob_pattern = fs_config.glob.as_str();
        let matcher = directory.join(glob_pattern);
        let entries: Vec<PathBuf> = glob(matcher.to_str().unwrap())?.flatten().collect();

        let mut assets = HashMap::new();

        for entry in entries {
            let content =
                fs::read_to_string(&entry).map_err(|e| crate::types::Error::ExternalCommand {
                    message: format!("Failed to read asset file: {}", e),
                })?;

            let asset: Asset =
                toml::from_str(&content).map_err(|e| crate::types::Error::ExternalCommand {
                    message: format!("Failed to parse asset file: {}", e),
                })?;

            assets.insert(asset.id.clone(), asset);
        }

        Ok(assets)
    }
}
