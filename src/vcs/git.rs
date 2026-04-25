// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::env::current_dir;
use std::path::PathBuf;

use derive_more::Debug;
use git2::Repository;
use snafu::OptionExt;
use snafu::ensure;

use crate::Result;
use crate::config::Config;
use crate::error::{FilesystemSnafu, MissingVcsConfigSnafu};
use crate::utils::{is_in_project, path_relative_to_prefix};
use crate::{CommitMessage, StorageTracker};

#[derive(Debug)]
pub struct GitTracker {
    stage: bool,
    commit: bool,
    #[debug(skip)]
    repo: Repository,
    message: CommitMessage,
}

impl GitTracker {
    pub fn new(config: &Config) -> Result<Self> {
        log::info!("Discovering Git repo");
        let repo = Repository::discover(&config.project)?;
        let git_config = config.git.as_ref().context(MissingVcsConfigSnafu {
            driver: "gittracker",
        })?;
        let message = CommitMessage::new(git_config.message.clone());
        Ok(Self {
            stage: git_config.stage,
            commit: git_config.commit,
            repo,
            message,
        })
    }
}

impl StorageTracker for GitTracker {
    fn is_clean(&self, dirty: bool) -> Result<()> {
        log::info!("Initializing git storage tracker");
        let statuses = self.repo.statuses(None).map_err(|_| {
            FilesystemSnafu {
                message: "Failed to get git status".to_string(),
            }
            .build()
        })?;
        let has_staged = statuses.iter().any(|s| {
            let status = s.status();
            status.is_index_new() || status.is_index_modified() || status.is_index_deleted()
        });
        if dirty && has_staged {
            log::warn!("Operating on dirty repository");
            return Ok(());
        }
        ensure!(
            !has_staged,
            FilesystemSnafu {
                message: "Git repository has staged changes. Please commit or unstage them first."
                    .to_string(),
            }
        );
        Ok(())
    }

    fn stage_paths(
        &self,
        paths: &[PathBuf],
        pre_stage_hook: Option<&dyn Fn(&PathBuf) -> Result<()>>,
    ) -> Result<()> {
        if self.stage {
            log::info!("Adding files to VCS staging: {:?}", paths);
            let mut index = self.repo.index()?;
            let project_dir = self.repo.workdir().ok_or_else(|| {
                FilesystemSnafu {
                    message: "Git repository has no working directory".to_string(),
                }
                .build()
            })?;
            for path in paths {
                if is_in_project(path, project_dir) {
                    let path = path_relative_to_prefix(path, &current_dir()?);
                    if let Some(hook) = &pre_stage_hook {
                        hook(&path)?;
                    }
                    index.add_path(&path)?;
                } else {
                    log::warn!(
                        "Not staging asset file {:?} outside of project directory.",
                        path
                    );
                }
            }
            index.write()?;
        } else {
            log::warn!(
                "Automatic staging disabled, suggest manually staging: {:?}",
                paths
            );
        }
        Ok(())
    }

    fn commit_staged(&self, composer: Option<&dyn Fn(&mut CommitMessage)>) -> Result<()> {
        let mut index = self.repo.index()?;
        let oid = index.write_tree()?;
        let tree = self.repo.find_tree(oid)?;
        let signature = self.repo.signature()?;
        let parent = self
            .repo
            .head()
            .ok()
            .map(|h| h.peel_to_commit())
            .transpose()?;
        let mut msg = self.message.clone();
        let version =
            option_env!("VERGEN_GIT_DESCRIBE").unwrap_or_else(|| env!("CARGO_PKG_VERSION"));
        msg.trailers
            .push(format!("Assisted-by: acceptarium {}", version));
        if let Some(callback) = composer {
            callback(&mut msg);
        }
        let msg = msg.render()?;
        if self.commit {
            self.repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                msg.as_ref(),
                &tree,
                &parent.iter().collect::<Vec<_>>(),
            )?;
            log::info!("Committing changes");
        } else {
            log::warn!("Automatic committing disabled, suggest committing: {}", msg);
        }
        Ok(())
    }
}
