// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod config;
pub mod types;

// Data handling
pub mod storage;

// Subcommands
pub mod run;
pub mod status;

// Import stuff set by autoconf/automake at build time
pub static CONFIGURE_PREFIX: &str = env!["CONFIGURE_PREFIX"];
pub static CONFIGURE_BINDIR: &str = env!["CONFIGURE_BINDIR"];
pub static CONFIGURE_DATADIR: &str = env!["CONFIGURE_DATADIR"];

#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod cli;

use nanoid::nanoid;

pub(crate) const ASSET_ID_LEN: usize = 7;

pub(crate) const ASSET_ID_CHARS: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

pub(crate) fn new_id() -> String {
    let id = nanoid!(ASSET_ID_LEN, &ASSET_ID_CHARS);
    id.to_string()
}
