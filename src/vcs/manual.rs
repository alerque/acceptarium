// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;
use crate::StorageTracker;

use std::path::PathBuf;

pub struct ManualTracker {}

impl StorageTracker for ManualTracker {
    fn is_clean(&self, _dirty: bool) -> Result<()> {
        log::warn!("Configured for manual tracking, ignoring safety check for clean working state");
        Ok(())
    }

    fn stage_paths(
        &self,
        paths: &[PathBuf],
        _pre_stage_hook: Option<&dyn Fn(&PathBuf) -> Result<()>>,
    ) -> Result<()> {
        log::warn!(
            "Suggest manually adding {:?} to your version tracking.",
            paths
        );
        Ok(())
    }

    fn commit_staged(&self, msg: &str) -> Result<()> {
        log::warn!("Suggest committing current changes as '{}'", msg);
        Ok(())
    }
}
