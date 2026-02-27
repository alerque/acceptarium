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
    #[clap(short, long, action = clap::ArgAction::SetTrue, overrides_with("no_debug"))]
    pub debug: Option<bool>,

    #[clap(long, action = clap::ArgAction::SetFalse, hide = true, overrides_with("debug"))]
    pub no_debug: Option<bool>,

    /// Discard all non-error output messages
    #[clap(short, long, action = clap::ArgAction::SetTrue, overrides_with("no_quiet"))]
    pub quiet: Option<bool>,

    #[clap(long = "no-quiet", action = clap::ArgAction::SetFalse, hide = true)]
    pub no_quiet: Option<bool>,

    /// Enable extra verbose output from tooling
    #[clap(short, long, action = clap::ArgAction::SetTrue, overrides_with("no_verbose"))]
    pub verbose: Option<bool>,

    #[clap(long = "no-verbose", action = clap::ArgAction::SetFalse, hide = true)]
    pub no_verbose: Option<bool>,

    /// Run actions in dry run mode that checks everything but makes no changes
    #[clap(short = 'n', long, action = clap::ArgAction::SetTrue, overrides_with("no_dry_run"))]
    pub dry_run: Option<bool>,

    #[clap(long = "no-dry-run", action = clap::ArgAction::SetFalse, hide = true)]
    pub no_dry_run: Option<bool>,

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
