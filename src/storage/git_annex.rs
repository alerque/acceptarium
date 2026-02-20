// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::{Asset, AssetId, Result};

use super::Storage;

use std::collections::HashMap;

pub struct GitAnnexStorage {
    _config: Config,
}

impl GitAnnexStorage {
    pub fn new(_config: Config) -> Self {
        Self { _config }
    }
}

impl Storage for GitAnnexStorage {
    fn list(&self) -> Result<HashMap<AssetId, Asset>> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }
}
