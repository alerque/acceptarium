// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::GitAnnexConfig;
use crate::types::Result;

use super::Storage;

pub struct GitAnnexStorage {
    _config: GitAnnexConfig,
}

impl GitAnnexStorage {
    pub fn new(_config: GitAnnexConfig) -> Self {
        Self { _config }
    }
}

impl Storage for GitAnnexStorage {
    fn list(&self) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }
}
