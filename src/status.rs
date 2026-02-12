// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::Result;

/// Dump what we know
pub fn run(config: &Config) -> Result<()> {
    print!("{config:#?}");
    Ok(())
}
