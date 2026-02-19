// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum StorageDriver {
    Filesystem,
    GitAnnex,
}

/// Ingest, process, store, analyze, and export receipts from raster scans to plain text accounting
/// tools.
#[derive(Parser, Debug)]
#[clap(author, bin_name = "acceptarium")]
pub struct Cli {
    /// Enable extra debug output from tooling
    #[clap(short, long)]
    pub debug: bool,

    /// Set project root path
    #[clap(short, long, value_hint = clap::ValueHint::DirPath)]
    pub project: Option<PathBuf>,

    /// Discard all non-error output messages
    #[clap(short, long)]
    pub quiet: bool,

    /// Enable extra verbose output from tooling
    #[clap(short, long)]
    pub verbose: bool,

    /// Storage backend to use
    #[clap(short, long)]
    pub storage: Option<StorageDriver>,

    /// Path to a TOML config file (overrides default acceptarium.toml)
    #[clap(short, long, value_hint = clap::ValueHint::FilePath)]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Debug)]
#[clap(subcommand_negates_reqs = true)]
pub enum Commands {
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

    /// Run a custom command script
    #[clap(external_subcommand)]
    External(Vec<OsString>),
}

pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Magenta.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::BrightCyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default())
    .error(AnsiColor::BrightRed.on_default().bold())
    .valid(AnsiColor::BrightGreen.on_default().bold())
    .invalid(AnsiColor::BrightYellow.on_default().bold());
