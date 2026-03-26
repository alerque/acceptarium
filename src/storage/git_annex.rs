// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::actions::{data_is_in_project, data_is_writable, is_in_project};
use crate::config::Config;
use crate::error::{
    AssetHashExistsSnafu, FilesystemSnafu, IoSnafu, MissingStorageConfigSnafu,
};
use crate::storage::git_tracker::GitTracker;
use crate::{Asset, AssetId, Assets, OperationMode, Result};
use crate::{Ingestable, Storage};

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use derive_more::Debug;
use git2::Repository;
use serde::{Deserialize, Serialize};
use snafu::ensure;
use snafu::{OptionExt, ResultExt};
use subprocess::{Exec, Redirection};

#[derive(Debug)]
pub struct GitAnnexStorage {
    project_dir: PathBuf,
    data_dir: PathBuf,
    copy: bool,
    rename: bool,
    commit: bool,
    #[debug(skip)]
    repo: Option<Repository>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum AnnexCommand {
    Add,
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

impl GitAnnexStorage {
    pub fn init(config: &Config) -> Result<Box<dyn Storage>> {
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
        let repo = Some(Repository::discover(&project_dir)?);
        let store = Box::new(Self {
            project_dir,
            data_dir,
            copy: storage_config.copy,
            rename: storage_config.rename,
            commit: storage_config.commit,
            repo,
        });
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

    fn set_asset_metadata(&self, asset: &Asset) -> Result<()> {
        let kvpairs = asset.to_annex_metadata();
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
}

impl GitTracker for GitAnnexStorage {
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

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<()> {
        self.exec_annex_cli(AnnexCommand::Add, Some(paths))?;
        Ok(())
    }
}

impl Storage for GitAnnexStorage {
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
            let mut to_stage = vec![];
            if self.copy || is_in_project(&asset_path_abs, &self.project_dir) {
                to_stage.push(asset_path_abs);
            } else {
                log::warn!(
                    "Not staging asset file {:?} outside of project directory.",
                    &asset_path_abs
                );
            }
            self.stage_paths(&to_stage)?;
            self.set_asset_metadata(&asset)?;
            if self.commit {
                self.commit_staged("Track new asset(s)")?;
            }
        }
        Ok(asset)
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
            assets.add(asset);
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
            log::error!(
                "Multiple asset files are tagged with id '{}' in git-annex metadata. Using first result, but manual correction of duplicated assets required.",
                &id,
            );
        }
        Asset::from_annex_metadata_json(line)
    }

    fn get(&self, config: &crate::Config, id: AssetId, key: &str) -> Result<String> {
        let asset = self.load(id)?;
        let value = asset.get_field(key)?;
        crate::output::dump(config, &value)
    }

    fn set(&self, id: AssetId, key: &str, value: &str) -> Result<()> {
        let mut asset = self.load(id.clone())?;
        asset.set_field(key, value)?;
        self.save(&asset)?;
        Ok(())
    }

    fn save(&self, asset: &Asset) -> Result<()> {
        self.set_asset_metadata(asset)?;
        Ok(())
    }

    fn remove(&self, id: AssetId) -> Result<()> {
        let asset = self.load(id.clone())?;
        if let Some(asset_path) = asset.asset_path(&self.project_dir)
            && asset_path.exists()
        {
            log::info!("Removing asset file {:?}", &asset_path);
            std::fs::remove_file(&asset_path)?;
            self.stage_paths(&[asset_path])?;
        }
        if self.commit {
            self.commit_staged("Remove asset(s)")?;
        }
        Ok(())
    }

    fn is_clean(&self, dirty: &bool) -> Result<()> {
        let cleanish = self.ensure_staging_empty();
        if *dirty && cleanish.is_err() {
            log::warn!("Operating on dirty repository");
            return Ok(());
        }
        cleanish
    }
}
