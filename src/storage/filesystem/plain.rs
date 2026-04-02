// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::error::{FilesystemSnafu, IoSnafu, MissingStorageConfigSnafu};
use crate::utils::path_relative_to_prefix;
#[cfg(feature = "git")]
use crate::utils::{data_is_in_project, data_is_writable};
use crate::{Asset, OperationMode, Result};
use crate::{BlobStorage, InfoStorage, Ingestable, StorageTracker};

use std::path::PathBuf;

use derive_more::Debug;
use snafu::ensure;
use snafu::{OptionExt, ResultExt};

#[derive(Debug)]
pub struct PlainBlob {
    project_dir: PathBuf,
    data_dir: PathBuf,
    copy: bool,
    rename: bool,
}

impl PlainBlob {
    pub fn init(config: &Config) -> Result<Self> {
        log::info!("Initializing plain blob storage");
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
        let store = Self {
            project_dir,
            data_dir,
            copy: storage_config.copy,
            rename: storage_config.rename,
        };
        log::debug!("Completed plain blob storage initialization: {:?}", store);
        Ok(store)
    }
}

impl BlobStorage for PlainBlob {
    fn ingest(
        &self,
        mode: OperationMode,
        source: &dyn Ingestable,
        info: &dyn InfoStorage,
        tracker: &dyn StorageTracker,
    ) -> Result<Option<Asset>> {
        log::info!("Ingesting new asset");
        let source_file = source.path().context(FilesystemSnafu {
            message: "Current implementation must have a valid filesystem path",
        })?;
        let blake3 = source.blake3().clone();
        if mode != OperationMode::JustRun {
            let assets = info.list()?;
            let existing_with_same_checksum = assets
                .iter()
                .find(|(_, asset)| asset.blake3().is_some_and(|hash| *hash == blake3));
            if existing_with_same_checksum.is_some() {
                return Ok(None);
            }
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
        let asset_path = path_relative_to_prefix(&asset_path_abs, &self.project_dir);
        asset.set_asset_path(Some(&asset_path));
        if mode != OperationMode::JustRun && !self.rename {
            ensure!(
                !&asset_path_abs.try_exists().context(IoSnafu)?,
                FilesystemSnafu {
                    message: format!("Data file '{}' already exists", &asset_path_abs.display()),
                }
            );
        }
        if mode != OperationMode::JustCheck {
            if self.copy {
                std::fs::copy(source_file, &asset_path_abs)?;
            }
            tracker.stage_paths(&[asset_path_abs])?;
            info.write(&asset, tracker)?;
        }
        Ok(Some(asset))
    }

    fn egest(
        &self,
        asset: &Asset,
        info: &dyn InfoStorage,
        tracker: &dyn StorageTracker,
    ) -> Result<()> {
        if let Some(asset_path) = asset.asset_path(&self.project_dir)
            && asset_path.exists()
        {
            if asset_path.starts_with(&self.project_dir) {
                log::info!("Removing asset file {:?}", &asset_path);
                std::fs::remove_file(&asset_path)?;
                tracker.stage_paths(&[asset_path])?;
            } else {
                log::warn!(
                    "Not removing asset file {:?} outside of project directory.",
                    &asset_path
                );
            }
        }
        info.erase(asset, tracker)?;
        Ok(())
    }

    fn as_info(&self) -> Option<Box<dyn InfoStorage>> {
        None
    }
}
