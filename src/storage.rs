// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
#[cfg(not(feature = "git-annex"))]
use crate::types::UnsupportedStorageSnafu;
use crate::types::{
    Assets, MissingStorageConfigSnafu, NoStorageConfiguredSnafu, Result, StorageDriver,
};

pub trait Storage {
    fn list(&self) -> Result<Assets>;
}

#[cfg(feature = "git-annex")]
pub mod git_annex;

pub mod filesystem;

pub fn list(config: &Config) -> Result<()> {
    let storage = instantiate_storage(config)?;
    let assets = storage.list()?;
    print!("{}", assets);
    Ok(())
}

fn instantiate_storage(config: &Config) -> Result<Box<dyn Storage>> {
    let config = config.clone();
    let storage: Box<dyn Storage> = match config.storage {
        Some(StorageDriver::Filesystem) => match config.filesystem {
            Some(_) => Box::new(filesystem::FilesystemStorage::new(config)),
            None => {
                return MissingStorageConfigSnafu {
                    driver: "filesystem",
                }
                .fail()
            }
        },
        #[cfg(feature = "git-annex")]
        Some(StorageDriver::GitAnnex) => match config.git_annex {
            Some(_) => Box::new(git_annex::GitAnnexStorage::new(config)),
            None => {
                return MissingStorageConfigSnafu {
                    driver: "git-annex",
                }
                .fail()
            }
        },
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
