// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::error::{
    AssetHashExistsSnafu, FilesystemSnafu, IoSnafu, MissingStorageConfigSnafu, NonUnicodePathSnafu,
    UnknownAssetIdSnafu, UnknownMetaKeySnafu,
};
#[cfg(feature = "git")]
use crate::storage::git_tracker::GitTracker;
#[cfg(feature = "git")]
use crate::storage::is_in_project;
use crate::storage::{data_is_in_project, data_is_writable};
use crate::{Asset, AssetId, Assets, OperationMode, Result};
use crate::{Ingestable, Storage};

use blake3::Hash as Blake3;
use derive_more::Debug;
#[cfg(feature = "git")]
use git2::Repository;
use glob::glob;
use snafu::ensure;
use snafu::{OptionExt, ResultExt};
use std::env::current_dir;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use sugar_path::SugarPath;

#[derive(Debug)]
pub struct FilesystemStorage {
    project_dir: PathBuf,
    data_dir: PathBuf,
    glob_pattern: String,
    copy: bool,
    rename: bool,
    track: bool,
    commit: bool,
    #[cfg(feature = "git")]
    #[debug(skip)]
    repo: Option<Repository>,
}

impl FilesystemStorage {
    pub fn init(config: &Config) -> Result<Box<dyn Storage>> {
        log::info!("Initializing storage");
        let storage_config = config
            .filesystem
            .as_ref()
            .context(MissingStorageConfigSnafu {
                driver: "filesystem",
            })?;
        let project_dir = config.project.canonicalize()?;
        let data_dir = project_dir.join(&storage_config.directory).canonicalize()?;
        data_is_in_project(&data_dir, &project_dir)?;
        data_is_writable(&data_dir)?;
        #[cfg(feature = "git")]
        let repo = if storage_config.track {
            log::info!("Tracking is enabled, discovering VCS repo");
            Some(Repository::discover(&project_dir)?)
        } else {
            None
        };
        let store = Box::new(Self {
            project_dir,
            data_dir,
            glob_pattern: storage_config.glob.to_string(),
            copy: storage_config.copy,
            rename: storage_config.rename,
            track: storage_config.track,
            commit: storage_config.commit,
            #[cfg(feature = "git")]
            repo,
        });
        #[cfg(not(feature = "git"))]
        ensure!(
            !store.track && !store.commit,
            FilesystemSnafu {
                message: "This project is configured to track assets in Git, but the 'git' feature to be enabled",
            }
        );
        log::debug!("Completed initialization: {:?}", store);
        Ok(store)
    }

    fn metadata_path(&self, asset: &Asset) -> Result<PathBuf> {
        let base_name: PathBuf = if self.copy {
            asset.id().to_string().into()
        } else {
            let path = asset
                .asset_path(&self.project_dir)
                .expect("an asset without an asset path is a liability");
            path.file_name()
                .expect("asset path has no file name")
                .into()
        };
        let path = self.data_dir.join(base_name).with_extension("toml");
        Ok(path)
    }
}

#[cfg(feature = "git")]
impl GitTracker for FilesystemStorage {
    fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    fn repo(&self) -> Result<&Repository> {
        self.repo.as_ref().context(FilesystemSnafu {
            message: "Git repository not initialized".to_string(),
        })
    }

    fn commit(&self) -> bool {
        self.commit
    }
}

impl Storage for FilesystemStorage {
    fn add(&self, source: &dyn Ingestable, mode: OperationMode) -> Result<Asset> {
        log::info!("Ingesting new asset");
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
        let metadata_path = self.metadata_path(&asset)?;
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
            #[cfg(feature = "git")]
            if self.track {
                let mut to_stage = vec![metadata_path];
                if self.copy || is_in_project(&asset_path_abs, &self.project_dir) {
                    to_stage.push(asset_path_abs);
                } else {
                    log::warn!(
                        "Not staging asset file {:?} outside of project directory.",
                        &asset_path_abs
                    );
                }
                self.stage_paths(&to_stage)?;
                if self.commit {
                    self.commit_staged("Track new asset(s)")?;
                }
            }
        }
        Ok(asset)
    }

    fn list(&self) -> Result<Assets> {
        log::info!("Listing known assets");
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

    fn load(&self, id: AssetId) -> Result<Asset> {
        let assets = self.list()?;
        assets.get(&id).cloned().context(UnknownAssetIdSnafu { id })
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

    fn set(&self, id: AssetId, key: &str, value: &str) -> Result<()> {
        let mut asset = self.load(id.clone())?;
        match key {
            "asset_path" => {
                asset.set_asset_path(Some(Path::new(value)));
            }
            "source_fname" => {
                asset.set_source_fname(Some(Path::new(value)));
            }
            "blake3" => {
                asset.set_blake3(Some(Blake3::from_hex(value).expect("bad hash").into()));
            }
            _ => return UnknownMetaKeySnafu { key }.fail(),
        }
        self.save(&asset)?;
        Ok(())
    }

    fn save(&self, asset: &Asset) -> Result<()> {
        let toml_content = toml::to_string_pretty(asset)?;
        let metadata_path = self.metadata_path(asset)?;
        std::fs::write(&metadata_path, toml_content)?;
        #[cfg(feature = "git")]
        if self.track {
            self.stage_paths(&[metadata_path])?;
            if self.commit {
                self.commit_staged("Update existing asset(s)")?;
            }
        }
        Ok(())
    }

    fn remove(&self, id: AssetId) -> Result<()> {
        let asset = self.load(id.clone())?;
        if let Some(asset_path) = asset.asset_path(&self.project_dir)
            && asset_path.exists()
        {
            if asset_path.starts_with(&self.project_dir) {
                log::info!("Removing asset file {:?}", &asset_path);
                std::fs::remove_file(&asset_path)?;
                #[cfg(feature = "git")]
                if self.track {
                    self.stage_paths(&[asset_path])?;
                }
            } else {
                log::warn!(
                    "Not removing asset file {:?} outside of project directory.",
                    &asset_path
                );
            }
        }
        let metadata_path = self.metadata_path(&asset)?;
        if metadata_path.exists() {
            log::info!("Removing metadata file {:?}", &metadata_path);
            std::fs::remove_file(&metadata_path)?;
            #[cfg(feature = "git")]
            if self.track {
                self.stage_paths(&[metadata_path])?;
            }
        }
        #[cfg(feature = "git")]
        if self.track && self.commit {
            self.commit_staged("Remove asset(s)")?;
        }
        Ok(())
    }

    fn is_clean(&self, dirty: &bool) -> Result<()> {
        #[cfg(feature = "git")]
        if self.track {
            let cleanish = self.ensure_staging_empty();
            if *dirty && cleanish.is_err() {
                log::warn!("Operating on dirty repository");
                return Ok(());
            }
            return cleanish;
        }
        Ok(())
    }
}
