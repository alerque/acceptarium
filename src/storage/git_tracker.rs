// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;
use crate::error::FilesystemSnafu;
use git2::Repository;
use snafu::ensure;
use std::path::{Path, PathBuf};

pub trait GitTracker {
    fn project_dir(&self) -> &Path;
    fn repo(&self) -> Result<&Repository>;
    fn commit(&self) -> bool;

    fn ensure_staging_empty(&self) -> Result<()> {
        let statuses = self.repo()?.statuses(None).map_err(|_| {
            FilesystemSnafu {
                message: "Failed to get git status".to_string(),
            }
            .build()
        })?;
        let has_staged = statuses.iter().any(|s| {
            let status = s.status();
            status.is_index_new() || status.is_index_modified() || status.is_index_deleted()
        });
        ensure!(
            !has_staged,
            FilesystemSnafu {
                message: "Git repository has staged changes. Please commit or unstage them first."
                    .to_string(),
            }
        );
        Ok(())
    }

    fn stage_paths(&self, paths: &[PathBuf]) -> Result<()> {
        let mut index = self.repo()?.index()?;
        index.add_all(paths, git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    fn commit_staged(&self, msg: &str) -> Result<()> {
        let repo = self.repo()?;
        let mut index = repo.index()?;
        let oid = index.write_tree()?;
        let tree = repo.find_tree(oid)?;
        let signature = repo.signature()?;
        let parent = repo.head().ok().map(|h| h.peel_to_commit()).transpose()?;
        let msg = format!("{}\n\nAssisted-by: Acceptarium", msg);
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            msg.as_ref(),
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )?;
        Ok(())
    }
}
