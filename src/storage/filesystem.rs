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
    data_dir: PathBuf,
    glob_pattern: String,
}

impl FilesystemStorage {
    pub fn init(config: &Config) -> Result<Box<dyn Storage>> {
        let fs_config = config
            .filesystem
            .as_ref()
            .context(MissingStorageConfigSnafu {
                driver: "filesystem",
            })?;
        let project_dir = config.project.canonicalize()?;
        let data_dir = project_dir.join(&fs_config.directory).canonicalize()?;
        data_dir
            .starts_with(&project_dir)
            .then_some(())
            .context(FilesystemSnafu {
                message: format!(
                    "Storage directory '{}' is not inside project root '{}'",
                    fs_config.directory.display(),
                    project_dir.display()
                ),
            })?;
        let data_meta = std::fs::metadata(&data_dir)?;
        data_meta
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
        Ok(Box::new(Self {
            data_dir,
            glob_pattern: fs_config.glob.to_string(),
        }))
    }
}

impl Storage for FilesystemStorage {
    fn add(&self, file: PathBuf) -> Result<Asset> {
        let asset = Asset::new(Some(file))?;
        let mut metadata_path = self.data_dir.join(asset.id.to_string());
        metadata_path.add_extension("toml");
        let toml_content = toml::to_string_pretty(&asset)?;
        std::fs::write(metadata_path, toml_content)?;
        Ok(asset)
    }

    fn list(&self) -> Result<Assets> {
        let matcher = self.data_dir.join(&self.glob_pattern);
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
