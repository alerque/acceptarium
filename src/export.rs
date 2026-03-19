// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::storage;
use crate::{AssetId, Config, Error, LedgerFormat, Result};

pub fn run<ID>(config: &Config, all: bool, unprocessed: bool, ids: Option<&[ID]>) -> Result<()>
where
    for<'a> &'a ID: TryInto<AssetId>,
    for<'a> Error: From<<&'a ID as TryInto<AssetId>>::Error>,
{
    let assets = storage::list(config, all, unprocessed, ids)?;
    let format = config.format;
    for (_, asset) in &assets {
        log::debug!("Attempting to format {} with as {:?}", &asset.id(), &format);
        match format {
            LedgerFormat::HLedger => {
                log::debug!(
                    "Using template {:?} for format {:?}",
                    &config.template,
                    &format
                );
                let transaction = config.template.render(config, asset)?;
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
