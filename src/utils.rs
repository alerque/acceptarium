// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::{Path, PathBuf};

use snafu::ensure;

use crate::InfoFormat;
use crate::PROJECT_CONFIG;
use crate::Result;
use crate::error::FilesystemSnafu;

#[cfg(feature = "git")]
pub(crate) fn discover_project_root(cwd: &Path) -> PathBuf {
    use git2::Repository;
    let git_repo = Repository::discover(cwd).ok();
    let git_root = git_repo
        .as_ref()
        .and_then(|repo| repo.workdir().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(&cwd));
    walk_to_root_or_config(cwd, &git_root)
}

#[cfg(not(feature = "git"))]
pub(crate) fn discover_project_root(cwd: &Path) -> PathBuf {
    walk_to_root_or_config(cwd, &PathBuf::from("/"))
}

fn walk_to_root_or_config(cwd: &Path, root: &PathBuf) -> PathBuf {
    let mut current = cwd.to_path_buf();
    loop {
        let config = current.join(PROJECT_CONFIG);
        if config.exists() {
            return current;
        }
        if current == *root {
            break;
        }
        if !current.pop() {
            break;
        }
    }
    root.clone()
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

pub(crate) fn info_extension(format: InfoFormat) -> String {
    match format {
        InfoFormat::JSON => "json",
        InfoFormat::TOML => "toml",
        InfoFormat::YAML => "yaml",
        InfoFormat::HJSON => "hjson",
        InfoFormat::XML => "xml",
    }
    .into()
}

pub(crate) fn path_relative_to_prefix(path: &Path, prefix: &Path) -> PathBuf {
    path.strip_prefix(prefix)
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.to_path_buf())
}
