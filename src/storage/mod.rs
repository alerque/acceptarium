// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;
use crate::ingestable::Ingestable;
use crate::{Asset, AssetId, Assets};
use crate::{InfoFormat, OperationMode, StorageTracker};

pub mod filesystem;
#[cfg(feature = "git-annex")]
pub mod git_annex;

use std::any::Any;

pub trait BlobStorage {
    fn ingest(
        &self,
        mode: OperationMode,
        source: &dyn Ingestable,
        info: &dyn InfoStorage,
        tracker: &dyn StorageTracker,
    ) -> Result<Option<Asset>>;
    fn egest(
        &self,
        asset: &Asset,
        info: &dyn InfoStorage,
        tracker: &dyn StorageTracker,
    ) -> Result<()>;
    fn as_info(&self) -> Option<Box<dyn InfoStorage>>;
}

pub trait InfoStorage {
    fn write(&self, asset: &Asset, tracker: &dyn StorageTracker) -> Result<()>;
    fn erase(&self, asset: &Asset, tracker: &dyn StorageTracker) -> Result<()>;
    fn list(&self) -> Result<Assets>;
    fn load(&self, id: AssetId) -> Result<Asset>;
    fn get(&self, format: InfoFormat, id: AssetId, key: &str) -> Result<String>;
    fn as_any(&self) -> &dyn Any;
}
