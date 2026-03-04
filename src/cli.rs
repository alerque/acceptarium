// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum StorageDriver {
    #[default]
    Filesystem,
    GitAnnex,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Processor {
    #[default]
    Manual,
    OCR,
    Vision,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Extractor {
    #[default]
    Manual,
    LLM,
    Regex,
    Vision,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LedgerFormat {
    #[default]
    HLedger,
    #[serde(rename = "ledger-cli")]
    LedgerCli,
    BeanCount,
    CSV,
    JSON,
}

/// Ingest, process, store, analyze, and export receipts from raster scans to plain text accounting
/// tools.
#[derive(Parser, Debug)]
#[clap(author, bin_name = "acceptarium")]
pub struct Cli {
    #[command(flatten)]
    pub verbosity: Verbosity<WarnLevel>,

    /// Run actions in dry run mode that checks everything but makes no changes
    #[clap(short = 'n', long, action = clap::ArgAction::SetTrue, overrides_with("no_dry_run"))]
    pub dry_run: Option<bool>,

    #[clap(long = "no-dry-run", action = clap::ArgAction::SetFalse, hide = true)]
    pub no_dry_run: Option<bool>,

    /// Allow changing Git state even when repository is dirty
    #[clap(short = 'd', long, action = clap::ArgAction::SetTrue, overrides_with("no_dirty"))]
    pub dirty: Option<bool>,

    #[clap(long, action = clap::ArgAction::SetFalse, hide = true)]
    pub no_dirty: Option<bool>,

    /// Overwrite existing extracted transaction data
    #[clap(long, action = clap::ArgAction::SetTrue, overrides_with("no_overwrite"))]
    pub overwrite: Option<bool>,

    #[clap(long = "no-overwrite", action = clap::ArgAction::SetFalse, hide = true)]
    pub no_overwrite: Option<bool>,

    /// Set project root path
    #[clap(short, long, value_hint = clap::ValueHint::DirPath)]
    pub project: Option<PathBuf>,

    /// Override a configuration value (can be passed multiple times)
    #[clap(short, long, num_args = 2, value_names = ["KEY", "VALUE"])]
    pub config: Vec<String>,

    /// Path to a TOML config file, relative to project root
    #[clap(long, value_hint = clap::ValueHint::FilePath)]
    pub config_file: Option<PathBuf>,

    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Debug)]
#[clap(subcommand_negates_reqs = true)]
pub enum Commands {
    /// Import an asset file and begin tracking via the configured storage
    Add {
        /// Automatically commit imported asset to VCS tracker (if configured)
        #[clap(short = 't', long, action = clap::ArgAction::SetTrue, overrides_with("no_commit"))]
        commit: Option<bool>,

        #[clap(long = "no-commit", action = clap::ArgAction::SetFalse, hide = true)]
        no_commit: Option<bool>,

        /// Copy the source file from its current location into the configured data directory
        #[clap(short, long, action = clap::ArgAction::SetTrue, overrides_with("no_copy"))]
        copy: Option<bool>,

        #[clap(long = "no-copy", action = clap::ArgAction::SetFalse, hide = true)]
        no_copy: Option<bool>,

        /// Rename source files using the internal asset ID when copying to data folder
        #[clap(short, long, action = clap::ArgAction::SetTrue, overrides_with("no_rename"))]
        rename: Option<bool>,

        #[clap(long = "no-rename", action = clap::ArgAction::SetFalse, hide = true)]
        no_rename: Option<bool>,

        /// Files to add as assets (at least one required)
        #[clap(value_hint = clap::ValueHint::FilePath, required = true, num_args(1..))]
        files: Vec<PathBuf>,
    },

    /// List known assets
    List {
        /// View only assets currently tracked in VCS (if configured)
        #[clap(long, action = clap::ArgAction::SetTrue, overrides_with("no_tracked"))]
        tracked: Option<bool>,

        #[clap(long = "no-tracked", action = clap::ArgAction::SetFalse, hide = true)]
        no_tracked: Option<bool>,

        /// Output assets as JSON
        #[clap(short, long)]
        json: bool,

        #[command(flatten)]
        selectors: AssetSelectors,
    },

    /// Process an asset to extract data
    Process {
        /// Choose a specific image processor
        #[clap(short, long)]
        processor: Option<Processor>,

        /// Choose a specific data extractor
        #[clap(short, long)]
        extractor: Option<Extractor>,

        #[command(flatten)]
        selectors: AssetSelectors,
    },

    /// Output an asset to a PTA format
    Export {
        /// Ledger format to target
        #[clap(short, long)]
        format: Option<LedgerFormat>,

        #[command(flatten)]
        selectors: AssetSelectors,
    },

    /// Get metadata for a specific asset by ID
    Get {
        /// Asset ID to look up
        id: String,

        /// Metadata key for which to return the value
        key: String,
    },

    /// Set metadata for a specific asset by ID
    Set {
        /// Asset ID to modify
        id: String,

        /// Metadata key to set
        key: String,

        /// Value to set
        value: String,
    },

    /// Remove an asset and its metadata
    Remove {
        #[command(flatten)]
        selectors: AssetSelectors,
    },

    /// Execute a script as a child process that inherits Acceptarium environment
    Run {
        /// Name of script supplied either by Acceptarium or a local project
        #[clap(value_hint = clap::ValueHint::CommandName)]
        name: OsString,

        /// Arguments to pass to script being run
        #[clap(value_hint = clap::ValueHint::Unknown)]
        arguments: Vec<OsString>,
    },

    /// Show status information about configuration, and state
    Status {},

    /// TUI interface for interactively managing assets
    #[cfg(feature = "tui")]
    Tui {},

    /// Run a custom command script
    #[clap(external_subcommand)]
    External(Vec<OsString>),
}

/// Asset selector arguments
#[derive(Args, Debug)]
#[group()]
pub struct AssetSelectors {
    /// Operate on all known assets
    #[clap(short, long, action = clap::ArgAction::SetTrue)]
    pub all: bool,

    /// Operate on assets that have not been marked as processed
    #[clap(short, long, action = clap::ArgAction::SetTrue)]
    pub unprocessed: bool,

    /// Operate on a list of asset ID(s)
    #[clap(value_hint = clap::ValueHint::Unknown, required_unless_present_any = ["all", "unprocessed"], num_args(1..))]
    pub ids: Option<Vec<String>>,
}

pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Magenta.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::BrightCyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default())
    .error(AnsiColor::BrightRed.on_default().bold())
    .valid(AnsiColor::BrightGreen.on_default().bold())
    .invalid(AnsiColor::BrightYellow.on_default().bold());
