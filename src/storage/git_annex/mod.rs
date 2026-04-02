// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::error::{FilesystemSnafu, IoSnafu, MissingStorageConfigSnafu};
use crate::utils::{data_is_in_project, data_is_writable, path_relative_to_prefix};
use crate::{Asset, AssetId, Assets, InfoFormat, OperationMode, Result};
use crate::{BlobStorage, InfoStorage, Ingestable, StorageTracker};

use std::any::Any;
use std::ffi::OsString;
use std::path::PathBuf;

use derive_more::Debug;
use serde::{Deserialize, Serialize};
use snafu::ensure;
use snafu::{OptionExt, ResultExt};
use subprocess::{Exec, Redirection};

#[derive(Debug, Clone)]
pub struct AnnexedBlob {
    project_dir: PathBuf,
    data_dir: PathBuf,
    copy: bool,
    rename: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum AnnexCommand {
    Metadata,
}

impl From<AnnexCommand> for OsString {
    fn from(value: AnnexCommand) -> OsString {
        serde_json::to_value(value)
            .ok()
            .and_then(|v| v.as_str().map(Into::into))
            .unwrap()
    }
}

impl AnnexedBlob {
    pub fn init(config: &Config) -> Result<Self> {
        log::info!("Initializing storage");
        let storage_config = config
            .git_annex
            .as_ref()
            .context(MissingStorageConfigSnafu {
                driver: "git-annex",
            })?;
        let project_dir = config.project.canonicalize()?;
        let data_dir = project_dir.join(&storage_config.directory).canonicalize()?;
        data_is_in_project(&data_dir, &project_dir)?;
        data_is_writable(&data_dir)?;
        let store = Self {
            project_dir,
            data_dir,
            copy: storage_config.copy,
            rename: storage_config.rename,
        };
        log::debug!("Completed initialization: {:?}", store);
        Ok(store)
    }

    fn exec_annex_cli<A>(&self, command: AnnexCommand, args: Option<A>) -> Result<String>
    where
        A: IntoIterator,
        A::Item: Into<OsString>,
    {
        let args: Vec<OsString> = args.into_iter().flatten().map(Into::into).collect();
        log::info!(
            "Executing git-annex CLI command {:?} with args {:?}",
            &command,
            &args
        );
        let output = Exec::cmd("git-annex")
            .arg(command)
            .args(args)
            .cwd(&self.project_dir)
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Pipe)
            .capture()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.exit_status.success() {
            return Err(crate::error::FilesystemSnafu {
                message: format!("git-annex failed: {} {}", stdout, stderr),
            }
            .build());
        }
        Ok(stdout)
    }

    fn write_min(&self, asset: &Asset, _tracker: &dyn StorageTracker) -> Result<()> {
        let kvpairs_min = asset.to_annex_metadata(true);
        let mut args: Vec<OsString> = kvpairs_min
            .iter()
            .flat_map(|kv| [OsString::from("-s"), OsString::from(kv)])
            .collect();
        let asset_path = asset
            .asset_path(&self.project_dir)
            .ok_or("Asset has no asset path")?;
        args.insert(0, asset_path.into());
        self.exec_annex_cli(AnnexCommand::Metadata, Some(args))?;
        Ok(())
    }
}

impl BlobStorage for AnnexedBlob {
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
            let assets = self.list()?;
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
            if info.as_any().downcast_ref::<AnnexedBlob>().is_none() {
                // Keep an ID field in annex meta data  even when using sidecar info storage
                self.write_min(&asset, tracker)?;
            } else {
                info.write(&asset, tracker)?;
            }
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
            log::info!("Removing asset file {:?}", &asset_path);
            std::fs::remove_file(&asset_path)?;
            tracker.stage_paths(&[asset_path])?;
            info.erase(asset, tracker)?;
        }
        Ok(())
    }

    fn as_info(&self) -> Option<Box<dyn InfoStorage>> {
        Some(Box::new(self.clone()))
    }
}

impl InfoStorage for AnnexedBlob {
    fn write(&self, asset: &Asset, _tracker: &dyn StorageTracker) -> Result<()> {
        let kvpairs = asset.to_annex_metadata(false);
        let mut args: Vec<OsString> = kvpairs
            .iter()
            .flat_map(|kv| [OsString::from("-s"), OsString::from(kv)])
            .collect();
        let asset_path = asset
            .asset_path(&self.project_dir)
            .ok_or("Asset has no asset path")?;
        args.insert(0, "--remove-all".into());
        args.insert(0, asset_path.into());
        self.exec_annex_cli(AnnexCommand::Metadata, Some(args))?;
        Ok(())
    }

    fn erase(&self, _asset: &Asset, _tracker: &dyn StorageTracker) -> Result<()> {
        Ok(())
    }

    fn list(&self) -> Result<Assets> {
        log::info!("Listing annex files with acceptarium metadata");
        let output = self.exec_annex_cli(
            AnnexCommand::Metadata,
            Some(&["--json", "--metadata", "acceptarium.id=*"]),
        )?;
        let mut assets = Assets::new();
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let asset = Asset::from_annex_metadata_json(line)?;
            assets.insert(asset);
        }
        Ok(assets)
    }

    fn load(&self, id: AssetId) -> Result<Asset> {
        let args: Vec<String> = vec![
            "--json".to_string(),
            "--metadata".to_string(),
            format!("acceptarium.id={}", id),
        ];
        let output = self.exec_annex_cli(AnnexCommand::Metadata, Some(args))?;
        let mut lines = output.lines();
        let line = lines.next().unwrap_or_default();
        log::debug!("Raw git-annex metadata output: {}", &line);
        if lines.next().is_some() {
            log::warn!(
                "Multiple asset files are tagged with id '{}' in git-annex metadata. Using first result, but manual correction of duplicated assets required.",
                &id,
            );
        }
        Asset::from_annex_metadata_json(line)
    }

    fn get(&self, format: InfoFormat, id: AssetId, key: &str) -> Result<String> {
        let asset = self.load(id)?;
        asset.get_field(format, key)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
