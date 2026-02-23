// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::{Asset, Assets, Result};
use crate::types::{FilesystemSnafu, MissingStorageConfigSnafu};

use super::Storage;

use glob::glob;
use snafu::OptionExt;
use std::fs::read_to_string;
use std::ops::Not;
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
    fn init(&self) -> Result<()> {
        let fs_config = self
            .config
            .filesystem
            .as_ref()
            .context(MissingStorageConfigSnafu {
                driver: "filesystem",
            })?;
        let project_dir = self.config.project.canonicalize()?;
        let fs_dir = project_dir.join(&fs_config.directory);
        fs_dir
            .starts_with(&project_dir)
            .then_some(())
            .context(FilesystemSnafu {
                message: format!(
                    "Storage directory '{}' is not inside project root '{}'",
                    fs_config.directory.display(),
                    project_dir.display()
                ),
            })?;
        let metadata = std::fs::metadata(&fs_dir)?;
        metadata
            .permissions()
            .readonly()
            .not()
            .then_some(())
            .context(FilesystemSnafu {
                message: format!(
                    "Storage directory '{}' is not writable by the current user",
                    fs_config.directory.display()
                ),
            })?;
        Ok(())
    }

    fn add(&self, file: PathBuf) -> Result<Asset> {
        let mut asset = Asset::new(Some(file))?;
        let project_root = &self.config.project;
        let fs_config = self.config.filesystem.as_ref().unwrap();
        let mut directory = project_root.clone();
        directory.push(fs_config.directory.clone());
        let filename = format!("{}.toml", asset.id);
        let metadata_path = directory.join(filename);
        let toml_content = toml::to_string_pretty(&asset)?;
        std::fs::write(metadata_path, toml_content)?;
        Ok(asset)
    }

    fn list(&self) -> Result<Assets> {
        let mut directory = self.config.project.clone();
        let fs_config = self.config.filesystem.as_ref().unwrap();
        directory.push(fs_config.directory.clone());
        let glob_pattern = fs_config.glob.as_str();
        let matcher = directory.join(glob_pattern);
        let entries: Vec<PathBuf> = glob(matcher.to_str().unwrap())?.flatten().collect();
        let mut assets = Assets::new();
        for entry in entries {
            let content = read_to_string(&entry).map_err(|e| crate::types::Error::Filesystem {
                message: format!("Failed to read asset file: {}", e),
            })?;
            let mut asset: Asset =
                toml::from_str(&content).map_err(|e| crate::types::Error::Filesystem {
                    message: format!("Failed to parse asset file: {}", e),
                })?;
            asset.file = Some(entry);
            assets.add(asset);
        }
        Ok(assets)
    }
}
