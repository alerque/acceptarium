// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt::Debug;
use std::path::{Path, PathBuf};

use crate::Blake3Sum;

pub mod local_file;

pub use local_file::LocalFile;

pub trait Ingestable: Send {
    fn blake3(&self) -> &Blake3Sum;
    fn filename(&self) -> Option<&Path>;
    fn path(&self) -> Option<&Path>;
}

impl TryFrom<PathBuf> for Box<dyn Ingestable> {
    type Error = crate::Error;

    fn try_from(path: PathBuf) -> crate::Result<Self> {
        Ok(Box::new(LocalFile::try_from(path)?))
    }
}

impl Debug for Box<dyn Ingestable> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ingestable")
            .field("blake3", &self.blake3())
            .field("filename", &self.filename())
            .field("path", &self.path())
            .finish()
    }
}
