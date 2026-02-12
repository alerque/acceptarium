// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use clap::builder::styling::{AnsiColor, Styles};
use clap::{Parser, Subcommand};
use std::path;

/// Ingest, process, store, analyze, and export receipts from raster scans to plain text accounting
/// tools.
#[derive(Parser, Debug)]
#[clap(author, bin_name = "acceptarium")]
pub struct Cli {
    /// Enable extra debug output from tooling
    #[clap(short, long)]
    pub debug: bool,

    /// Set project root path
    #[clap(short = 'P', long, default_value = "./", value_hint = clap::ValueHint::DirPath)]
    pub project: path::PathBuf,

    /// Discard all non-error output messages
    #[clap(short, long)]
    pub quiet: bool,

    /// Enable extra verbose output from tooling
    #[clap(short, long)]
    pub verbose: bool,

    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Show status information about configuration, and state
    Status {},
}

pub const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Magenta.on_default().bold())
    .usage(AnsiColor::Yellow.on_default().bold())
    .literal(AnsiColor::BrightCyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default())
    .error(AnsiColor::BrightRed.on_default().bold())
    .valid(AnsiColor::BrightGreen.on_default().bold())
    .invalid(AnsiColor::BrightYellow.on_default().bold());
