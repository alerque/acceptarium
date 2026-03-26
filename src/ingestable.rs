// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Blake3Sum;

use std::path::Path;

pub mod local_file;

pub trait Ingestable: Send {
    fn blake3(&self) -> &Blake3Sum;
    fn filename(&self) -> Option<&Path>;
    fn path(&self) -> Option<&Path>;
}
