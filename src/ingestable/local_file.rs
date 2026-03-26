// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Ingestable;
use crate::assets::Blake3Sum;
use crate::error::{FilesystemSnafu, IoSnafu};
use crate::types::Result;

use snafu::ResultExt;
use snafu::ensure;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct LocalFile {
    pub path: PathBuf,
    pub filename: PathBuf,
    pub blake3: Blake3Sum,
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
}

impl Ingestable for LocalFile {
    fn blake3(&self) -> &Blake3Sum {
        &self.blake3
    }

    fn filename(&self) -> Option<&Path> {
        Some(&self.filename)
    }

    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}
