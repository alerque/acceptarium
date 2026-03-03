// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::error::{
    AssetHashExistsSnafu, FilesystemSnafu, IoSnafu, MissingStorageConfigSnafu, NonUnicodePathSnafu,
    UnknownAssetIdSnafu, UnknownMetaKeySnafu,
};
use crate::storage::{data_is_in_project, data_is_writable};
use crate::{Asset, AssetId, Assets, OperationMode, Result};
use crate::{Ingestable, Storage};

use glob::glob;
use snafu::ensure;
use snafu::{OptionExt, ResultExt};
use std::env::current_dir;
use std::fs::read_to_string;
use std::path::PathBuf;
use sugar_path::SugarPath;

pub struct FilesystemStorage {
    project_dir: PathBuf,
    data_dir: PathBuf,
    glob_pattern: String,
    copy: bool,
    rename: bool,
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
        data_is_in_project(&data_dir, &project_dir)?;
        data_is_writable(&data_dir)?;
        Ok(Box::new(Self {
            project_dir,
            data_dir,
            glob_pattern: fs_config.glob.to_string(),
            copy: fs_config.copy,
            rename: fs_config.rename,
        }))
    }
}

impl Storage for FilesystemStorage {
    fn add(&self, source: &dyn Ingestable, mode: OperationMode) -> Result<Asset> {
        let source_file = source.path().context(FilesystemSnafu {
            message: "Current implementation must have a valid filesystem path",
        })?;
        let blake3 = source.blake3().clone();
        if mode != OperationMode::JustRun {
            let assets = self.list()?;
            let existing_with_same_checksum = assets
                .iter()
                .find(|(_, asset)| asset.blake3().is_some_and(|hash| *hash == blake3));
            ensure!(
                existing_with_same_checksum.is_none(),
                AssetHashExistsSnafu {
                    asset_path: existing_with_same_checksum
                        .map(|(asset_path, _)| asset_path.to_string())
                        .unwrap_or_default()
                }
            );
        }
        let mut asset = Asset::new(None, Some(source_file), Some(blake3))?;
        let source_ext = source_file.extension().unwrap_or_default();
        let dest_base: PathBuf = match self.rename {
            true => asset.id().to_string().into(),
            false => source_file.file_stem().unwrap_or_default().into(),
        };
        let asset_path_abs: PathBuf = match self.copy {
            true => {
                let mut dest = self.data_dir.join(&dest_base);
                dest.add_extension(source_ext);
                dest
            }
            false => source_file.to_path_buf(),
        };
        let asset_path = asset_path_abs
            .strip_prefix(&self.project_dir)
            .map(PathBuf::from)
            .unwrap_or(asset_path_abs.clone());
        asset.set_asset_path(Some(&asset_path));
        let toml_content = toml::to_string_pretty(&asset)?;
        let mut metadata_path = self.data_dir.join(&dest_base);
        metadata_path.add_extension("toml");
        if mode != OperationMode::JustRun && !self.rename {
            ensure!(
                !&asset_path_abs.try_exists().context(IoSnafu)?,
                FilesystemSnafu {
                    message: format!("Data file '{}' already exists", &asset_path_abs.display()),
                }
            );
            ensure!(
                !metadata_path.try_exists().context(IoSnafu)?,
                FilesystemSnafu {
                    message: format!(
                        "Metadata file '{}' already exists",
                        &metadata_path.display()
                    ),
                }
            );
        }
        if mode != OperationMode::JustCheck {
            if self.copy {
                std::fs::copy(source_file, &asset_path_abs)?;
            }
            std::fs::write(&metadata_path, toml_content)?;
        }
        Ok(asset)
    }

    fn list(&self) -> Result<Assets> {
        let matcher = self.data_dir.join(&self.glob_pattern);
        let entries: Vec<PathBuf> = glob(matcher.to_str().context(NonUnicodePathSnafu)?)?
            .flatten()
            .collect();
        let mut assets = Assets::new();
        for entry in entries {
            let content = read_to_string(&entry)?;
            let mut asset: Asset = toml::from_str(&content)?;
            if let Some(asset_path) = asset.asset_path(&self.project_dir) {
                let cwd = current_dir()?.canonicalize()?;
                let asset_path = if asset_path.starts_with(&self.project_dir)
                    && cwd.starts_with(&self.project_dir)
                {
                    self.project_dir.join(&asset_path).relative(cwd)
                } else {
                    asset_path.canonicalize()?
                };
                asset.set_asset_path(Some(&asset_path));
            }
            assets.add(asset);
        }
        Ok(assets)
    }

    fn get(&self, id: AssetId, key: &str) -> Result<String> {
        let assets = self.list()?;
        if let Some(asset) = assets.get(&id) {
            let value = match key {
                "id" => asset.id().to_string(),
                "asset_path" => asset
                    .asset_path(&self.project_dir)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "source_fname" => asset
                    .source_fname()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "blake3" => asset.blake3().map(|h| h.to_string()).unwrap_or_default(),
                _ => UnknownMetaKeySnafu { key }.fail()?,
            };
            Ok(value)
        } else {
            UnknownAssetIdSnafu { id }.fail()?
        }
    }
}
