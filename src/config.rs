// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::{GlobPattern, Result, StorageDriver};

use crate::cli::Cli;

use config::Case;
use config::{Config as LayeredConfig, Environment, File};
use convert_case::Casing;
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Value};

use std::env;
use std::path::{Path, PathBuf};

fn default_directory() -> PathBuf {
    PathBuf::from("./acceptarium")
}

fn default_glob() -> GlobPattern {
    GlobPattern::new("*.toml").unwrap()
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[allow(unused)]
pub struct FilesystemConfig {
    #[serde(default = "default_directory")]
    pub directory: PathBuf,
    #[serde(default = "default_glob")]
    pub glob: GlobPattern,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[allow(unused)]
pub struct GitAnnexConfig {
    #[serde(default = "default_directory")]
    pub directory: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(unused)]
pub struct Config {
    pub debug: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub project: PathBuf,
    #[serde(rename(deserialize = "config-file"))]
    pub config_file: Option<PathBuf>,
    pub(crate) storage: Option<StorageDriver>,
    pub(crate) filesystem: Option<FilesystemConfig>,
    // swap rename for alias for env var parsing, but then the TOML breaks.
    // #[serde(alias = "GITANNEX")]
    #[serde(rename(deserialize = "git-annex"))]
    pub(crate) git_annex: Option<GitAnnexConfig>,
}

impl Config {
    pub fn new(args: &Cli) -> Result<Self> {
        // Keep track of potential project (and hence config file) paths before config is ready so we can load relative configs from there
        let discovered_project: PathBuf = args
            .project
            .clone()
            .or_else(|| {
                env::var("ACCEPTARIUM_PROJECT")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| {
                let current_dir = env::current_dir().unwrap_or(PathBuf::from("./"));
                discover_project_root(&current_dir)
            });
        // Setup default config values
        let mut builder = LayeredConfig::builder()
            .set_default("debug", false)?
            .set_default("quiet", false)?
            .set_default("verbose", false)?
            .set_default("project", discovered_project.to_str().unwrap())?;
        // Layer in project level or manually specified config file
        let project_config: Option<PathBuf> = args
            .config_file
            .clone()
            .or_else(|| {
                env::var("ACCEPTARIUM_CONFIG")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
            })
            .or_else(|| {
                // Check if discovered_project has acceptarium.toml
                let path = discovered_project.join("acceptarium.toml");
                path.exists().then_some(path)
            });
        if let Some(path) = project_config {
            builder = builder
                .set_default("config", path.to_str().unwrap())?
                .add_source(File::from(path.as_path()).required(true));
        }
        // Layer in environment variables
        builder = builder.add_source(
            Environment::with_prefix("acceptarium")
                .separator("_")
                .prefix_separator("_")
                .ignore_empty(true),
        );
        // Layer in command line flags
        if args.debug {
            builder = builder.set_override("debug", true)?;
        }
        if args.quiet {
            builder = builder.set_override("quiet", true)?;
        }
        if args.verbose {
            builder = builder.set_override("verbose", true)?;
        }
        let mut config_overrides = args.config.clone().into_iter();
        while let (Some(key), Some(value)) = (config_overrides.next(), config_overrides.next()) {
            builder = builder.set_override(&key, value)?;
        }
        // Put it all together and deserialize it to a config struct
        let sources = builder.build()?;
        let config = sources.try_deserialize()?;
        Ok(config)
    }

    pub fn try_to_env_vars(&self) -> Result<Vec<(String, String)>> {
        let json_value = to_value(self)?;
        let mut envs = Vec::new();
        flatten_json_value(&json_value, "ACCEPTARIUM", &mut envs);
        Ok(envs)
    }
}

#[cfg(feature = "git")]
fn discover_project_root(cwd: &Path) -> PathBuf {
    use git2::Repository;
    let git_repo = Repository::discover(cwd).ok();
    let git_root = git_repo
        .as_ref()
        .and_then(|repo| repo.workdir().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(&cwd));
    walk_to_root_or_config(cwd, &git_root)
}

#[cfg(not(feature = "git"))]
fn discover_project_root(cwd: &Path) -> PathBuf {
    walk_to_root_or_config(cwd, &PathBuf::from("/"))
}

fn walk_to_root_or_config(cwd: &Path, root: &PathBuf) -> PathBuf {
    let mut current = cwd.to_path_buf();
    loop {
        let config = current.join("acceptarium.toml");
        if config.exists() {
            return current;
        }
        if current == *root {
            break;
        }
        if !current.pop() {
            break;
        }
    }
    root.clone()
}

fn flatten_json_value(value: &Value, prefix: &str, envs: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                let key_upper = key.to_case(Case::UpperFlat);
                let new_prefix = format!("{}_{}", prefix, key_upper);
                flatten_json_value(val, &new_prefix, envs);
            }
        }
        Value::Null => {}
        _ => {
            let s = value
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| value.to_string());
            if !s.is_empty() && s != "null" {
                envs.push((prefix.to_string(), s));
            }
        }
    }
}
