// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Storage;
use crate::error::AssetHashExistsSnafu;
use crate::ingestable::local_file::LocalFile;
use crate::{AssetId, Assets};
use crate::{Config, Error, OperationMode, Result};

use snafu::ensure;
use std::collections::HashSet;
use std::path::PathBuf;

pub mod filesystem;
#[cfg(feature = "git-annex")]
pub mod git_annex;
#[cfg(feature = "git")]
pub mod git_tracker;

pub fn add(config: &Config, storage: Box<dyn Storage>, sources: Vec<PathBuf>) -> Result<()> {
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

pub fn get<ID>(_config: &Config, storage: Box<dyn Storage>, id: ID, key: &str) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let id: AssetId = id.try_into()?;
    let val = storage.get(id, key)?;
    println!("{}", val);
    Ok(())
}

pub fn set<ID>(
    config: &Config,
    storage: Box<dyn Storage>,
    id: ID,
    key: &str,
    value: &str,
) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    storage.is_clean(&config.dirty)?;
    let asset_id: AssetId = id.try_into()?;
    storage.set(asset_id.clone(), key, value)?;
    println!("Set {} = {} for asset {}", key, value, asset_id);
    Ok(())
}

pub fn remove(config: &Config, storage: Box<dyn Storage>, assets: Assets) -> Result<()> {
    storage.is_clean(&config.dirty)?;
    for (_, asset) in &assets {
        let id = asset.id().clone();
        println!("Removed asset {}", &id);
        storage.remove(id)?;
    }
    Ok(())
}
