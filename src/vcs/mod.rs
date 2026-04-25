// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use crate::CommitMessage;
use crate::Result;

mod git;
mod manual;

pub use git::GitTracker;
pub use manual::ManualTracker;

pub type PreStageCallback = dyn Fn(&PathBuf) -> Result<()>;

pub trait StorageTracker {
    fn is_clean(&self, dirty: bool) -> Result<()>;
    fn stage_paths(
        &self,
        paths: &[PathBuf],
        pre_stage_hook: Option<&PreStageCallback>,
    ) -> Result<()>;
    fn commit_staged(&self, composer: Option<&dyn Fn(&mut CommitMessage)>) -> Result<()>;
}
