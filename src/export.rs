// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::AssetId;
use crate::{Config, Error, Result};

pub fn run<ID>(config: &Config, id: ID) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let id: AssetId = id.try_into()?;
    let format = config.format;
    dbg!(id, format);
    Ok(())
}
