// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::{Asset, Assets, Result};

use super::Storage;

use std::path::PathBuf;

pub struct GitAnnexStorage {
    _config: Config,
}

impl GitAnnexStorage {
    pub fn new(_config: Config) -> Self {
        Self { _config }
    }
}

impl Storage for GitAnnexStorage {
    fn init(&self) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn add(&self, _file: PathBuf) -> Result<Asset> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }
    fn list(&self) -> Result<Assets> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }
}
