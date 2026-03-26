// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Private modules
mod assets;
mod config;
mod error;
mod types;
mod utils;

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
pub use ingestable::Ingestable;
pub use storage::Storage;
pub use types::OperationMode;
pub use types::Result;
pub use types::Transaction;

// Import stuff set by autoconf/automake at build time
pub static CONFIGURE_PREFIX: &str = env!["CONFIGURE_PREFIX"];
pub static CONFIGURE_BINDIR: &str = env!["CONFIGURE_BINDIR"];
pub static CONFIGURE_DATADIR: &str = env!["CONFIGURE_DATADIR"];

const ASSET_ID_LEN: usize = 7;

const ASSET_ID_CHARS: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

const TRANSFORMED_PACKAGE_NAME: &str = env!["TRANSFORMED_PACKAGE_NAME"];
const BINARY_PREFIX: &str = basename(TRANSFORMED_PACKAGE_NAME);
#[cfg(feature = "git-annex")]
const ANNEX_META_PREFIX: &str = "acceptarium";

const DEFAULTS_TOML: &str = include_str!("defaults.toml");
const PROJECT_CONFIG: &str = "acceptarium.toml";

const fn basename(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'/' {
            let (_, rest) = s.split_at(i + 1);
            return rest;
        }
    }
    s
}
