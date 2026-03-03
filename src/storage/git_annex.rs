// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::{Asset, AssetId, Assets, OperationMode, Result};
use crate::{Ingestable, Storage};

pub struct GitAnnexStorage;

impl GitAnnexStorage {
    pub fn init(config: &Config) -> Result<Box<dyn Storage>> {
        let _ = config;
        unimplemented!("git-annex storage driver is not implemented yet")
    }
}

impl Storage for GitAnnexStorage {
    fn add(&self, _source: &dyn Ingestable, _mode: OperationMode) -> Result<Asset> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn list(&self) -> Result<Assets> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn load(&self, _id: AssetId) -> Result<Asset> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn get(&self, _id: AssetId, _key: &str) -> Result<String> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn set(&self, _id: AssetId, _key: &str, _value: &str) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn save(&self, _asset: &Asset) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }

    fn remove(&self, _id: AssetId) -> Result<()> {
        unimplemented!("git-annex storage driver is not implemented yet")
    }
}
