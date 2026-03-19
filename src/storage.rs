// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(feature = "git-annex"))]
use crate::error::UnsupportedStorageSnafu;
use crate::error::{AssetHashExistsSnafu, FilesystemSnafu, NoStorageConfiguredSnafu};
use crate::ingestable::local_file::LocalFile;
use crate::{AssetId, Assets, Storage};
use crate::{Config, Error, OperationMode, Result, StorageDriver};

use snafu::ensure;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub mod filesystem;
#[cfg(feature = "git-annex")]
pub mod git_annex;
#[cfg(feature = "git")]
pub mod git_tracker;

pub fn add(config: &Config, sources: Vec<PathBuf>) -> Result<()> {
    let storage = instantiate_storage(config)?;
    storage.is_clean(&config.dirty)?;
    let ingestables: Vec<_> = sources
        .iter()
        .map(|source| LocalFile::from_path(source.as_path()))
        .collect::<Result<_>>()?;
    let mut seen_hashes = HashSet::new();
    for ingestable in &ingestables {
        log::debug!("Attempting dry run add for {:?}", ingestable);
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
            log::debug!("Adding {:?}", ingestable);
            let asset = storage.add(ingestable, OperationMode::JustRun)?;
            println!("{}", asset);
        }
    }
    Ok(())
}

pub fn list(config: &Config) -> Result<Assets> {
    let storage = instantiate_storage(config)?;
    storage.list()
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

pub fn set<ID>(config: &Config, id: ID, key: &str, value: &str) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let storage = instantiate_storage(config)?;
    storage.is_clean(&config.dirty)?;
    let asset_id: AssetId = id.try_into()?;
    storage.set(asset_id.clone(), key, value)?;
    println!("Set {} = {} for asset {}", key, value, asset_id);
    Ok(())
}

pub fn remove<ID>(config: &Config, id: ID) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let storage = instantiate_storage(config)?;
    storage.is_clean(&config.dirty)?;
    let asset_id: AssetId = id.try_into()?;
    storage.remove(asset_id.clone())?;
    println!("Removed asset {}", asset_id);
    Ok(())
}

pub(crate) fn instantiate_storage(config: &Config) -> Result<Box<dyn Storage>> {
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

pub(crate) fn data_is_in_project(data_dir: &Path, project_dir: &Path) -> Result<()> {
    ensure!(
        data_dir.starts_with(project_dir),
        FilesystemSnafu {
            message: format!(
                "Storage directory '{}' is not inside project root '{}'",
                data_dir.display(),
                project_dir.display()
            ),
        }
    );
    Ok(())
}

pub(crate) fn data_is_writable(data_dir: &Path) -> Result<()> {
    let data_meta = std::fs::metadata(data_dir)?;
    ensure!(
        !data_meta.permissions().readonly(),
        FilesystemSnafu {
            message: format!(
                "Storage directory '{}' is not writable by the current user",
                data_dir.display()
            ),
        }
    );
    Ok(())
}

#[cfg(feature = "git")]
pub(crate) fn is_in_project(path: &Path, project_dir: &Path) -> bool {
    path.starts_with(project_dir)
}
