// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
#[cfg(not(feature = "git-annex"))]
use crate::types::UnsupportedStorageSnafu;
use crate::types::{MissingStorageConfigSnafu, NoStorageConfiguredSnafu};
use crate::types::{Result, StorageDriver};
use snafu::prelude::*;

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
    let config = config.clone();
    let storage: Box<dyn Storage> = match config.storage {
        Some(StorageDriver::Filesystem) => {
            let config = config.filesystem.context(MissingStorageConfigSnafu {
                driver: "filesystem",
            })?;
            Box::new(filesystem::FilesystemStorage::new(config))
        }
        #[cfg(feature = "git-annex")]
        Some(StorageDriver::GitAnnex) => {
            let config = config.git_annex.context(MissingStorageConfigSnafu {
                driver: "git-annex",
            })?;
            Box::new(git_annex::GitAnnexStorage::new(config))
        }
        #[cfg(not(feature = "git-annex"))]
        Some(StorageDriver::GitAnnex) => {
            return UnsupportedStorageSnafu {
                driver: "git-annex",
            }
            .fail()
        }
        None => return NoStorageConfiguredSnafu {}.fail(),
    };
    Ok(storage)
}
