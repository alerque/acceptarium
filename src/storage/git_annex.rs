// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::error::{FilesystemSnafu, MissingStorageConfigSnafu};
use crate::storage::git_tracker::GitTracker;
use crate::storage::{data_is_in_project, data_is_writable};
use crate::{Asset, AssetId, Assets, OperationMode, Result};
use crate::{Ingestable, Storage};

use derive_more::Debug;
use git2::Repository;
use snafu::OptionExt;
use std::path::PathBuf;

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
}

impl GitTracker for GitAnnexStorage {
    fn repo(&self) -> Result<&Repository> {
        self.repo.as_ref().context(FilesystemSnafu {
            message: "Git repository not initialized".to_string(),
        })
    }
}

impl Storage for GitAnnexStorage {
    fn add(&self, _source: &dyn Ingestable, _mode: OperationMode) -> Result<Asset> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn list(&self) -> Result<Assets> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn load(&self, _id: AssetId) -> Result<Asset> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn get(&self, _id: AssetId, _key: &str) -> Result<String> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn set(&self, _id: AssetId, _key: &str, _value: &str) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn save(&self, _asset: &Asset) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn remove(&self, _id: AssetId) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }
}
