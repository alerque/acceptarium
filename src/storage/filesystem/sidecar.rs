// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::any::Any;
use std::env::current_dir;
use std::fs::read_to_string;
use std::path::PathBuf;

use derive_more::Debug;
use glob::MatchOptions;
use glob::glob_with;
use snafu::OptionExt;
use sugar_path::SugarPath;

use crate::Config;
use crate::Result;
use crate::error::{MissingInfoConfigSnafu, UnknownAssetIdSnafu};
use crate::output::dump;
use crate::utils::{info_extension, path_relative_to_prefix};
use crate::{Asset, AssetId, Assets};
use crate::{InfoFormat, InfoStorage, StorageTracker};

#[derive(Debug)]
pub struct SidecarInfo {
    project_dir: PathBuf,
    data_dir: PathBuf,
    format: InfoFormat,
    hidden: bool,
}

impl SidecarInfo {
    pub fn new(config: &Config) -> Result<Self> {
        let project_dir = config.project.canonicalize()?;
        let sidecar_config = config
            .sidecar
            .as_ref()
            .context(MissingInfoConfigSnafu { driver: "sidecar" })?;
        let data_dir = project_dir.join(&sidecar_config.directory).canonicalize()?;
        Ok(Self {
            project_dir,
            data_dir,
            format: sidecar_config.format,
            hidden: sidecar_config.hidden,
        })
    }

    fn metadata_path(&self, asset: &Asset) -> Result<PathBuf> {
        let path = asset
            .asset_path(&self.project_dir)
            .expect("an asset without an asset path is a liability");
        let mut base_name: PathBuf = path
            .file_name()
            .expect("asset path has no file name")
            .into();
        if self.hidden {
            base_name = format!(".{}", base_name.display()).into();
        }
        let parent: PathBuf = path.parent().expect("you are lost Holmes").into();
        let extension = info_extension(self.format);
        let path = parent.join(base_name).with_extension(extension);
        Ok(path)
    }
}

impl InfoStorage for SidecarInfo {
    fn write(&self, asset: &Asset, tracker: &dyn StorageTracker) -> Result<()> {
        let content = dump(self.format, asset)?;
        let metadata_path = self.metadata_path(asset)?;
        std::fs::write(&metadata_path, content)?;
        tracker.stage_paths(&[metadata_path], None)?;
        tracker.commit_staged(Some(&move |msg| {
            msg.subject = Some("Update existing asset(s)".into());
        }))?;
        Ok(())
    }

    fn erase(&self, asset: &Asset, tracker: &dyn StorageTracker) -> Result<()> {
        let metadata_path = self.metadata_path(asset)?;
        if metadata_path.exists() {
            log::info!("Removing metadata file {:?}", metadata_path);
            std::fs::remove_file(&metadata_path)?;
            tracker.stage_paths(&[metadata_path], None)?;
        }
        Ok(())
    }

    fn list(&self) -> Result<Assets> {
        log::info!("Listing all assets");
        let pattern = {
            let ext = info_extension(self.format);
            let dir = self.data_dir.to_string_lossy();
            if self.hidden {
                format!("{}/.*.{}", dir, ext)
            } else {
                format!("{}/[!.]*.{}", dir, ext)
            }
        };
        log::debug!("Pattern used: {}", pattern);
        let entries: Vec<PathBuf> = glob_with(pattern.as_str(), MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        })?
        .flatten()
        .collect();
        let mut assets = Assets::new();
        for entry in entries {
            let content = read_to_string(&entry)?;
            let mut asset: Asset = match self.format {
                InfoFormat::JSON => serde_json::from_str(&content)?,
                InfoFormat::TOML => toml::from_str(&content)?,
                InfoFormat::YAML => serde_yaml::from_str(&content)?,
                InfoFormat::HJSON => serde_hjson::from_str(&content)?,
                InfoFormat::XML => serde_xml_rs::from_str(&content)?,
            };
            if let Some(asset_path) = asset.asset_path(&self.project_dir) {
                let cwd = current_dir()?.canonicalize()?;
                let asset_path = if asset_path.starts_with(&self.project_dir)
                    && cwd.starts_with(&self.project_dir)
                {
                    self.project_dir.join(&asset_path).relative(cwd)
                } else {
                    path_relative_to_prefix(&asset_path, &self.project_dir)
                };
                asset.set_asset_path(Some(&asset_path));
            }
            assets.insert(asset);
        }
        Ok(assets)
    }

    fn load(&self, id: AssetId) -> Result<Asset> {
        let assets = self.list()?;
        assets.get(&id).cloned().context(UnknownAssetIdSnafu { id })
    }

    fn get(&self, format: InfoFormat, id: AssetId, key: &str) -> Result<String> {
        let asset = self.load(id)?;
        asset.get_field(format, key)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
