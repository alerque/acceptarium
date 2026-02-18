// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::Result;
use crate::CONFIGURE_DATADIR;

use std::io::prelude::*;
use std::path::Path;
use std::{ffi::OsString, io};
use subprocess::{Exec, Redirection};

/// Execute a script as a child process that inherits Acceptarium environment
pub fn run(config: &Config, name: OsString, arguments: Vec<OsString>) -> Result<()> {
    let datadir = Path::new(CONFIGURE_DATADIR);
    let mut cmd = datadir.to_path_buf();
    cmd.push("scripts");
    cmd.push(name);
    let mut process = Exec::cmd(cmd)
        .args(&arguments)
        .env_extend(&config.try_to_env_vars()?);
    process = process.stderr(Redirection::Pipe).stdout(Redirection::Pipe);
    let mut popen = process.popen()?;
    let bufstdout = io::BufReader::new(popen.stdout.as_mut().unwrap());
    let bufstderr = io::BufReader::new(popen.stderr.as_mut().unwrap());
    for line in bufstdout.lines() {
        let text: &str =
            &line.unwrap_or_else(|_| String::from("INVALID UTF-8 FROM CHILD PROCESS STREAM"));
        println!("{text}");
    }
    for line in bufstderr.lines() {
        let text: &str =
            &line.unwrap_or_else(|_| String::from("INVALID UTF-8 FROM CHILD PROCESS STREAM"));
        eprintln!("{text}");
    }
    popen.wait()?;
    Ok(())
}
