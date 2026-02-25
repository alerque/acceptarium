// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::error::{FilesystemSnafu, MissingStorageConfigSnafu};
use crate::types::{Asset, Assets, Result};

use super::Storage;

use glob::glob;
use snafu::OptionExt;
use std::env::current_dir;
use std::fs::read_to_string;
use std::ops::Not;
use std::path::{Path, PathBuf};
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
            project_dir,
            data_dir,
            glob_pattern: fs_config.glob.to_string(),
            copy: fs_config.copy,
            rename: fs_config.rename,
        }))
    }
}

impl Storage for FilesystemStorage {
    fn add(&self, source: &Path) -> Result<Asset> {
        let source = source.canonicalize()?;
        let source_file = PathBuf::from(source.file_name().unwrap_or_default());
        let mut asset = Asset::new(None, Some(&source_file))?;
        let source_ext = source_file.extension().unwrap_or_default();
        let dest_base: PathBuf = match self.rename {
            true => asset.id().to_string().into(),
            false => source_file.file_stem().unwrap_or_default().into(),
        };
        let asset_path: PathBuf = match self.copy {
            true => {
                let mut dest = self.data_dir.join(&dest_base);
                dest.add_extension(source_ext);
                std::fs::copy(&source, &dest)?;
                dest
            }
            false => source,
        };
        let asset_path = asset_path
            .strip_prefix(&self.project_dir)
            .map(PathBuf::from)
            .unwrap_or(asset_path);
        asset.set_asset_path(Some(&asset_path));
        let toml_content = toml::to_string_pretty(&asset)?;
        let mut metadata_path = self.data_dir.join(&dest_base);
        metadata_path.add_extension("toml");
        std::fs::write(&metadata_path, toml_content)?;
        Ok(asset)
    }

    fn list(&self) -> Result<Assets> {
        let matcher = self.data_dir.join(&self.glob_pattern);
        let entries: Vec<PathBuf> = glob(matcher.to_str().unwrap())?.flatten().collect();
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
}
