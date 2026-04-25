// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use crate::Result;
use crate::{CommitMessage, StorageTracker};

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

    fn commit_staged(&self, composer: Option<&dyn Fn(&mut CommitMessage)>) -> Result<()> {
        let template = serde_json::from_str("[acceptarium] {{ subject }}")?;
        let mut msg = CommitMessage::new(template);
        let version =
            option_env!("VERGEN_GIT_DESCRIBE").unwrap_or_else(|| env!("CARGO_PKG_VERSION"));
        msg.trailers
            .push(format!("Assisted-by: acceptarium {}", version));
        if let Some(callback) = composer {
            callback(&mut msg);
        }
        let msg = msg.render()?;
        log::warn!("Suggest committing current changes as '{}'", msg);
        Ok(())
    }
}
