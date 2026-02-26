// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(feature = "git-annex"))]
use crate::error::UnsupportedStorageSnafu;
use crate::error::{FilesystemSnafu, IoSnafu, NoStorageConfiguredSnafu};
use crate::Storage;
use crate::{Config, Result, StorageDriver};

use snafu::{OptionExt, ResultExt};
use std::path::PathBuf;

#[cfg(feature = "git-annex")]
pub mod git_annex;

pub mod filesystem;

pub fn add(config: &Config, sources: Vec<PathBuf>, _commit: bool) -> Result<()> {
    let storage = instantiate_storage(config)?;
    // Check that all sources are readable files first, fail early to avoid partial operations
    sources.iter().try_for_each(|source| {
        source
            .try_exists()
            .context(IoSnafu)?
            .then_some(())
            .context(FilesystemSnafu {
                message: format!("Source file '{}' does not exist", source.display()),
            })
    })?;
    sources.iter().try_for_each(|source| {
        let asset = storage.add(source)?;
        println!("{}", asset);
        Ok(())
    })
}

pub fn list(config: &Config, json: bool) -> Result<()> {
    let storage = instantiate_storage(config)?;
    let assets = storage.list()?;
    if json {
        println!("{}", assets.to_json()?);
    } else {
        print!("{}", assets);
    }
    Ok(())
}

fn instantiate_storage(config: &Config) -> Result<Box<dyn Storage>> {
    match config.storage {
        Some(StorageDriver::Filesystem) => filesystem::FilesystemStorage::init(config),
        #[cfg(feature = "git-annex")]
        Some(StorageDriver::GitAnnex) => git_annex::GitAnnexStorage::init(config),
        #[cfg(not(feature = "git-annex"))]
        Some(StorageDriver::GitAnnex) => {
            return UnsupportedStorageSnafu {
                driver: "git-annex",
            }
            .fail()
        }
        None => NoStorageConfiguredSnafu {}.fail(),
    }
}
