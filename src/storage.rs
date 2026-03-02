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
    // Run everything in dry run mode first, fails early to avoid partial operations
    let mut seen_hashes = HashSet::new();
    sources.iter().try_for_each(|source| {
        let source = LocalFile::from_path(source)?;
        // Dry run for preflight checks
        let asset = storage.add(source.into_boxed(), OperationMode::JustCheck)?;
        if let Some(hash) = asset.blake3() {
            ensure!(
                seen_hashes.insert(hash.clone()),
                AssetHashExistsSnafu {
                    asset_path: asset
                        .source_fname()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default(),
                }
            );
        }
        Ok::<(), Error>(())
    })?;
    if !config.dry_run {
        sources.iter().try_for_each(|source| {
            let source = LocalFile::from_path(source)?;
            let asset = storage.add(source.into_boxed(), OperationMode::JustRun)?;
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
