// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::error::NoStorageConfiguredSnafu;
#[cfg(not(feature = "git-annex"))]
use crate::error::UnsupportedStorageSnafu;
use crate::{AssetId, Storage};
use crate::{Config, Error, Result, StorageDriver};

use std::path::PathBuf;

#[cfg(feature = "git-annex")]
pub mod git_annex;

pub mod filesystem;

pub fn add(config: &Config, sources: Vec<PathBuf>) -> Result<()> {
    let storage = instantiate_storage(config)?;
    // Run everything in dry run mode first, fails early to avoid partial operations
    sources.iter().try_for_each(|source| {
        // Dry run for preflight checks
        storage.add(source, true).map(drop)
    })?;
    if !config.dry_run {
        sources.iter().try_for_each(|source| {
            let asset = storage.add(source, false)?;
            println!("{}", asset);
            Ok(())
        })
    } else {
        Ok(())
    }
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

pub fn get<ID>(config: &Config, id: ID, key: &str) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let storage = instantiate_storage(config)?;
    let id: AssetId = id.try_into()?;
    let val = storage.get(id, key)?;
    println!("{}", val);
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
            .fail();
        }
        None => NoStorageConfiguredSnafu {}.fail(),
    }
}
