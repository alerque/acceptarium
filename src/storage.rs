// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::Result;

pub mod dummy;
pub mod filesystem;
#[cfg(feature = "git-annex")]
pub mod git_annex;

pub trait Storage {
    fn list(&self) -> Result<()>;
}
