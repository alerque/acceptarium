// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::error::{FilesystemSnafu, IoSnafu};
use crate::types::{Blake3Sum, Result};
use crate::Ingestable;

use snafu::ensure;
use snafu::ResultExt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalFile {
    path: PathBuf,
    filename: PathBuf,
    blake3: Blake3Sum,
}

impl LocalFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let path = path.canonicalize()?;
        ensure!(
            path.try_exists().context(IoSnafu)?,
            FilesystemSnafu {
                message: format!("Source file '{}' does not exist", path.display()),
            }
        );
        let filename = PathBuf::from(path.file_name().unwrap_or_default());
        let blake3 = Self::compute_blake3(&path)?;
        Ok(Self {
            path,
            filename,
            blake3,
        })
    }

    fn compute_blake3(path: &Path) -> Result<Blake3Sum> {
        let mut file = File::open(path)?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0u8; 8192];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(Blake3Sum::new(hasher.finalize()))
    }

    pub fn into_boxed(self) -> Box<dyn Ingestable> {
        Box::new(self)
    }
}

impl Ingestable for LocalFile {
    fn blake3(&self) -> &Blake3Sum {
        &self.blake3
    }

    fn filename(&self) -> Option<&Path> {
        Some(&self.filename)
    }

    fn source_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}
