// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
#[cfg(not(feature = "git-annex"))]
use crate::types::UnsupportedStorageSnafu;
use crate::types::{Asset, Assets, Result, StorageDriver};
use crate::types::{FileIoSnafu, FilesystemSnafu, NoStorageConfiguredSnafu};

use snafu::prelude::*;
use std::path::PathBuf;

pub trait Storage {
    fn add(&self, file: PathBuf) -> Result<Asset>;
    fn list(&self) -> Result<Assets>;
}

#[cfg(feature = "git-annex")]
pub mod git_annex;

pub mod filesystem;

pub fn add(config: &Config, files: Vec<PathBuf>, _commit: bool) -> Result<()> {
    let storage = instantiate_storage(config)?;
    // Check for extant readable files first to fail early and avoid a partial operation
    files.iter().try_for_each(|file| {
        file.try_exists()
            .context(FileIoSnafu)?
            .then_some(())
            .context(FilesystemSnafu {
                message: format!("File does not exist: {}", file.display()),
            })
    })?;
    files.iter().try_for_each(|file| {
        let asset = storage.add(file.to_owned())?;
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
