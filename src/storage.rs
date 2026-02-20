// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
#[cfg(not(feature = "git-annex"))]
use crate::types::UnsupportedStorageSnafu;
use crate::types::{NoStorageConfiguredSnafu, Result, StorageDriver};

pub trait Storage {
    fn list(&self) -> Result<()>;
}

#[cfg(feature = "git-annex")]
pub mod git_annex;

pub mod filesystem;

pub fn list(config: &Config) -> Result<()> {
    let storage = instantiate_storage(config)?;
    storage.list()
}

fn instantiate_storage(config: &Config) -> Result<Box<dyn Storage>> {
    match config.storage {
        Some(StorageDriver::Filesystem) => {
            Ok(Box::new(filesystem::FilesystemStorage::new(config.clone())))
        }
        #[cfg(feature = "git-annex")]
        Some(StorageDriver::GitAnnex) => {
            Ok(Box::new(git_annex::GitAnnexStorage::new(config.clone())))
        }
        #[cfg(not(feature = "git-annex"))]
        Some(StorageDriver::GitAnnex) => UnsupportedStorageSnafu {
            driver: "git-annex",
        }
        .fail(),
        None => NoStorageConfiguredSnafu {}.fail(),
    }
}
