// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;

use std::path::PathBuf;

mod git;
mod manual;

pub use git::GitTracker;
pub use manual::ManualTracker;

pub trait StorageTracker {
    fn is_clean(&self, dirty: bool) -> Result<()>;
    fn stage_paths(&self, paths: &[PathBuf]) -> Result<()>;
    fn commit_staged(&self, msg: &str) -> Result<()>;
}
