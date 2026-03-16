// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::AssetId;
use crate::LedgerFormat;
use crate::storage::instantiate_storage;
use crate::{Config, Error, Result};

pub fn run<ID>(config: &Config, id: ID) -> Result<()>
where
    ID: TryInto<AssetId>,
    Error: From<ID::Error>,
{
    let id: AssetId = id.try_into()?;
    let storage = instantiate_storage(config)?;
    let asset = storage.load(id)?;
    let format = config.format;
    log::debug!("Attempting to format {} with as {:?}", &asset.id(), &format);
    match format {
        LedgerFormat::HLedger => {
            log::debug!("Using template {:?}", &config.template);
            let transaction = config.template.render(config, &asset)?;
            println!("{transaction}");
        }
        LedgerFormat::LedgerCli => unimplemented!(),
        LedgerFormat::BeanCount => unimplemented!(),
        LedgerFormat::CSV => unimplemented!(),
        LedgerFormat::JSON => unimplemented!(),
    };
    Ok(())
}
