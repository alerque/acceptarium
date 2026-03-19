// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::AssetId;
use crate::LedgerFormat;
use crate::storage::instantiate_storage;
use crate::{Config, Error, Result};

pub fn run<ID>(config: &Config, ids: &Vec<ID>) -> Result<()>
where
    for<'a> &'a ID: TryInto<AssetId>,
    for<'a> Error: From<<&'a ID as TryInto<AssetId>>::Error>,
{
    let storage = instantiate_storage(config)?;
    let mut assets = Vec::new();
    for id in ids {
        let asset_id: AssetId = id.try_into()?;
        let asset = storage.load(asset_id)?;
        assets.push(asset);
    }
    let format = config.format;
    for asset in assets {
        log::debug!("Attempting to format {} with as {:?}", &asset.id(), &format);
        match format {
            LedgerFormat::HLedger => {
                log::debug!(
                    "Using template {:?} for format {:?}",
                    &config.template,
                    &format
                );
                let transaction = config.template.render(config, &asset)?;
                println!("{transaction}");
            }
            LedgerFormat::LedgerCli => unimplemented!(),
            LedgerFormat::BeanCount => unimplemented!(),
            LedgerFormat::CSV => unimplemented!(),
            LedgerFormat::JSON => unimplemented!(),
        };
    }
    Ok(())
}
