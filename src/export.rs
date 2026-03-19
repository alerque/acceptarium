// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;
use crate::{Assets, Config, LedgerFormat};

pub fn run(config: &Config, assets: Assets) -> Result<()> {
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
