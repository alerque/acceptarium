// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Private modules
mod assets;
mod config;
mod error;
mod types;

// Public modules
pub mod actions;
pub mod ingestable;
pub mod output;
pub mod process;
pub mod run;
pub mod status;
pub mod storage;

#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod cli;

#[cfg(feature = "tui")]
#[doc(hidden)]
pub mod tui;

// Public structs
pub use assets::Asset;
pub use assets::AssetId;
pub use assets::Assets;
pub use assets::Blake3Sum;
pub use cli::AssetSelectors;
pub use cli::DumpFormat;
pub use cli::ExportFormat;
pub use cli::Extractor;
pub use cli::Processor;
pub use cli::StorageDriver;
pub use config::Config;
pub use error::Error;
pub use types::OperationMode;
pub use types::Result;
pub use types::Transaction;

// Import stuff set by autoconf/automake at build time
pub static CONFIGURE_PREFIX: &str = env!["CONFIGURE_PREFIX"];
pub static CONFIGURE_BINDIR: &str = env!["CONFIGURE_BINDIR"];
pub static CONFIGURE_DATADIR: &str = env!["CONFIGURE_DATADIR"];

use std::path::Path;

pub trait Storage {
    fn add(&self, source: &dyn Ingestable, mode: OperationMode) -> Result<Asset>;
    fn list(&self) -> Result<Assets>;
    fn load(&self, id: AssetId) -> Result<Asset>;
    fn get(&self, id: AssetId, key: &str) -> Result<String>;
    fn set(&self, id: AssetId, key: &str, value: &str) -> Result<()>;
    fn save(&self, asset: &Asset) -> Result<()>;
    fn remove(&self, id: AssetId) -> Result<()>;
    fn is_clean(&self, diry: &bool) -> Result<()>;

    fn select(&self, selectors: &AssetSelectors) -> Result<Assets> {
        let assets = if selectors.all {
            self.list()?
        } else if selectors.processed {
            let mut assets = self.list()?;
            assets.retain(|_, asset| asset.transaction().is_some());
            assets
        } else if selectors.unprocessed {
            let mut assets = self.list()?;
            assets.retain(|_, asset| asset.transaction().is_none());
            assets
        } else {
            let mut assets = Assets::new();
            if let Some(ids) = &selectors.ids {
                for id in ids {
                    let asset_id: AssetId = id.try_into()?;
                    let asset = self.load(asset_id)?;
                    assets.add(asset);
                }
            }
            assets
        };
        Ok(assets)
    }
}

pub trait Ingestable: Send {
    fn blake3(&self) -> &Blake3Sum;
    fn filename(&self) -> Option<&Path>;
    fn path(&self) -> Option<&Path>;
}

const ASSET_ID_LEN: usize = 7;

const ASSET_ID_CHARS: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

#[cfg(feature = "git-annex")]
const ANNEX_META_PREFIX: &str = "acceptarium";

// Make up for clap not having a way to negate flags with None being a possible state
// c.f. https://github.com/clap-rs/clap/issues/815
pub(crate) fn deboolify(yes: Option<bool>, no: Option<bool>) -> Option<bool> {
    match (yes, no) {
        (Some(true), _) => yes,
        (_, Some(false)) => no,
        _ => None,
    }
}
