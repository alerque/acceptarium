// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::error::AssetHashExistsSnafu;
use crate::ingestable::Ingestable;
use crate::ingestable::local_file::LocalFile;
use crate::{Asset, AssetId, Assets};
use crate::{AssetSelectors, Config, DumpFormat, OperationMode};
use crate::{Error, Result};

use std::collections::HashSet;
use std::path::PathBuf;

use snafu::ensure;

pub mod filesystem;
#[cfg(feature = "git-annex")]
pub mod git_annex;
#[cfg(feature = "git")]
pub mod git_tracker;

pub trait Storage {
    fn ingest(&self, source: &dyn Ingestable, mode: OperationMode) -> Result<Asset>;
    fn list(&self) -> Result<Assets>;
    fn load(&self, id: AssetId) -> Result<Asset>;
    fn get(&self, format: DumpFormat, id: AssetId, key: &str) -> Result<String>;
    fn set(&self, format: DumpFormat, id: AssetId, key: &str, value: &str) -> Result<()>;
    fn save(&self, asset: &Asset) -> Result<()>;
    fn remove(&self, id: AssetId) -> Result<()>;
    fn is_clean(&self, diry: &bool) -> Result<()>;

    fn select(&self, selectors: &AssetSelectors) -> Result<Assets> {
        let assets = if selectors.all {
            self.list()?
        } else if selectors.processed {
            let mut assets = self.list()?;
            assets.retain(|_, asset| asset.transaction().is_some());
            assets
        } else if selectors.unprocessed {
            let mut assets = self.list()?;
            assets.retain(|_, asset| asset.transaction().is_none());
            assets
        } else {
            let mut assets = Assets::new();
            if let Some(ids) = &selectors.ids {
                for id in ids {
                    let asset_id: AssetId = id.try_into()?;
                    let asset = self.load(asset_id)?;
                    assets.insert(asset);
                }
            }
            assets
        };
        Ok(assets)
    }
}

pub fn add(config: &Config, storage: Box<dyn Storage>, sources: Vec<PathBuf>) -> Result<()> {
    storage.is_clean(&config.dirty)?;
    let ingestables: Vec<_> = sources
        .iter()
        .map(|source| LocalFile::from_path(source.as_path()))
        .collect::<Result<_>>()?;
    let mut seen_hashes = HashSet::new();
    for ingestable in &ingestables {
        log::debug!("Attempting dry run add for {:?}", ingestable);
        let _ = storage.ingest(ingestable, OperationMode::JustCheck)?;
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
            let asset = storage.ingest(ingestable, OperationMode::JustRun)?;
            println!("{}", asset);
        }
    }
    Ok(())
}

pub fn get<ID>(config: &Config, storage: Box<dyn Storage>, id: ID, key: &str) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let id: AssetId = id.try_into()?;
    let format = config.dump_format;
    let val = storage.get(format, id, key)?;
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
    let format = config.dump_format;
    storage.set(format, asset_id.clone(), key, value)?;
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
