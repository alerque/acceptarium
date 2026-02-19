// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::Result;

use super::Storage;

pub struct DummyStorage {
    _config: Config,
}

impl DummyStorage {
    pub fn new(_config: Config) -> Self {
        Self { _config }
    }
}

impl Storage for DummyStorage {
    fn list(&self) -> Result<()> {
        unimplemented!("No storage driver configured.")
    }
}
