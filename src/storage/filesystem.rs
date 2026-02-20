// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::FilesystemConfig;
use crate::types::Result;

use super::Storage;

pub struct FilesystemStorage {
    _config: FilesystemConfig,
}

impl FilesystemStorage {
    pub fn new(_config: FilesystemConfig) -> Self {
        Self { _config }
    }
}

impl Storage for FilesystemStorage {
    fn list(&self) -> Result<()> {
        unimplemented!("filesystem storage driver is not implemented yet")
    }
}
