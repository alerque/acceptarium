// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(feature = "git-annex"))]
use crate::error::UnsupportedStorageSnafu;
use crate::error::{AssetHashExistsSnafu, FilesystemSnafu, NoStorageConfiguredSnafu};
use crate::ingestable::local_file::LocalFile;
use crate::{AssetId, Storage};
use crate::{Config, Error, OperationMode, Result, StorageDriver};

use snafu::ensure;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

pub fn set<ID>(config: &Config, id: ID, key: &str, value: &str) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let storage = instantiate_storage(config)?;
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
pub(crate) fn project_is_workdir(project_dir: &Path) -> Result<()> {
    use git2::Repository;
    let git_repo = Repository::discover(&project_dir)?;
    let git_root = git_repo.workdir().and_then(|p| p.canonicalize().ok());
    ensure!(
        git_root == Some(project_dir.to_path_buf()),
        FilesystemSnafu {
            message: format!(
                "Project directory '{}' is not the root of a git repository",
                project_dir.display()
            ),
        }
    );
    Ok(())
}

#[cfg(feature = "git")]
pub(crate) fn staging_is_empty(project_dir: &Path) -> Result<()> {
    use git2::Repository;
    let repo = Repository::discover(project_dir)?;
    let statuses = repo.statuses(None).map_err(|_| {
        FilesystemSnafu {
            message: "Failed to get git status".to_string(),
        }
        .build()
    })?;
    let has_staged = statuses.iter().any(|s| {
        s.status().is_index_new() || s.status().is_index_modified() || s.status().is_index_deleted()
    });
    ensure!(
        !has_staged,
        FilesystemSnafu {
            message: "Git repository has staged changes. Please commit or unstage them first.",
        }
    );
    Ok(())
}
