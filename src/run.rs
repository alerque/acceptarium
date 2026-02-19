// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::config::Config;
use crate::types::Result;
use crate::CONFIGURE_DATADIR;

use std::ffi::OsString;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;
use subprocess::{Exec, Redirection};
use which::which;

/// Execute a script as a child process that inherits Acceptarium environment
pub fn run(config: &Config, name: OsString, arguments: Vec<OsString>) -> Result<()> {
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
    let mut process = Exec::cmd(cmd)
        .args(&arguments)
        .env("ACCEPTARIUM", "true")
        .env_extend(&config.try_to_env_vars()?);
    process = process.stderr(Redirection::Pipe).stdout(Redirection::Pipe);
    let mut proc = process.popen()?;
    let bufstdout = BufReader::new(proc.stdout.as_mut().unwrap());
    let bufstderr = BufReader::new(proc.stderr.as_mut().unwrap());
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
    proc.wait()?;
    Ok(())
}
