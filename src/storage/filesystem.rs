// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::Result;

use super::Storage;

use glob::glob;
use std::path::PathBuf;

pub struct FilesystemStorage {
    config: Config,
}

impl FilesystemStorage {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl Storage for FilesystemStorage {
    fn list(&self) -> Result<()> {
        let mut directory = self.config.project.clone();
        directory.push(self.config.filesystem.as_ref().unwrap().directory.clone());
        let matcher = directory.join(self.config.filesystem.as_ref().unwrap().glob.as_str());
        let entries: Vec<PathBuf> = glob(matcher.to_str().unwrap())?.flatten().collect();
        for entry in entries {
            if let Some(filename) = entry.file_name() {
                println!("{}", filename.to_string_lossy());
            }
        }
        Ok(())
    }
}
