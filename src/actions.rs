// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Storage;
use crate::error::{FilesystemSnafu, NoStorageConfiguredSnafu};
use crate::storage::filesystem::FilesystemStorage;
use crate::storage::git_annex::GitAnnexStorage;
use crate::{Config, Result, StorageDriver};

use std::path::Path;

use snafu::ensure;

pub fn instantiate_storage(config: &Config) -> Result<Box<dyn Storage>> {
    log::debug!("Selecting and initializing storage backend");
    match config.storage {
        Some(StorageDriver::Filesystem) => FilesystemStorage::init(config),
        #[cfg(feature = "git-annex")]
        Some(StorageDriver::GitAnnex) => GitAnnexStorage::init(config),
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
