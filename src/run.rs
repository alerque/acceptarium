// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::CONFIGURE_DATADIR;
use crate::error::{BufferSnafu, CurrentExecutableSnafu};
use crate::{Config, Result};

use std::env::current_exe;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use snafu::{OptionExt, ResultExt};
use subprocess::{Exec, Redirection};
use which::which;

pub type RunArgs = Vec<OsString>;

/// Execute a script as a child process that inherits Acceptarium environment
pub fn run(config: &Config, name: OsString, arguments: RunArgs) -> Result<()> {
    let mut script = PathBuf::from(CONFIGURE_DATADIR);
    script.push("scripts");
    script.push(&name);
    let cmd = if script.is_file() {
        script
    } else {
        let mut external = OsString::from("acceptarium-");
        external.push(name);
        which(&external)?
    };
    let acceptarium_bin = current_exe().context(CurrentExecutableSnafu {})?;
    let exec = Exec::cmd(cmd)
        .env("ACCEPTARIUM", acceptarium_bin)
        .env("ACCEPTARIUMDIR", CONFIGURE_DATADIR)
        .args(&arguments)
        .env_extend(config.try_to_env_vars()?)
        .stderr(Redirection::Pipe)
        .stdout(Redirection::Pipe);
    let mut job = exec.start()?;
    let bufstdout = BufReader::new(
        job.stdout
            .take()
            .context(BufferSnafu { stream: "STDOUT" })?,
    );
    let bufstderr = BufReader::new(
        job.stderr
            .take()
            .context(BufferSnafu { stream: "STDERR" })?,
    );
    for line in bufstdout.lines() {
        println!("{}", line?);
    }
    for line in bufstderr.lines() {
        eprintln!("{}", line?);
    }
    job.wait()?;
    Ok(())
}
