// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(feature = "git-annex"))]
use crate::error::UnsupportedStorageSnafu;
use crate::error::{AssetHashExistsSnafu, NoStorageConfiguredSnafu};
use crate::ingestable::local_file::LocalFile;
use crate::{AssetId, Storage};
use crate::{Config, Error, OperationMode, Result, StorageDriver};

use snafu::ensure;
use std::collections::HashSet;
use std::path::PathBuf;

pub mod filesystem;
#[cfg(feature = "git-annex")]
pub mod git_annex;

pub fn add(config: &Config, sources: Vec<PathBuf>) -> Result<()> {
    let storage = instantiate_storage(config)?;
    let ingestables: Vec<_> = sources
        .iter()
        .map(|source| LocalFile::from_path(source.as_path()))
        .collect::<Result<_>>()?;
    let mut seen_hashes = HashSet::new();
    for ingestable in &ingestables {
        let _ = storage.add(ingestable, OperationMode::JustCheck)?;
        ensure!(
            seen_hashes.insert(&ingestable.blake3),
            AssetHashExistsSnafu {
                asset_path: &ingestable.filename,
            }
        );
    }
    if !config.dry_run {
        for ingestable in &ingestables {
            let asset = storage.add(ingestable, OperationMode::JustRun)?;
            println!("{}", asset);
        }
    }
    Ok(())
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
