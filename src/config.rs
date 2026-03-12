// SPDX-FileCopyrightText: © 2026 Caleb Maclennan <caleb@alerque.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::cli::{Cli, Commands};
use crate::deboolify;
use crate::error::NonUnicodePathSnafu;
use crate::types::GlobPattern;
use crate::{Extractor, Processor, Result, StorageDriver};

use clap::ValueEnum;
use config::Case;
use config::{Config as LayeredConfig, Environment, File};
use convert_case::Casing;
use log::LevelFilter;
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, to_value};
use snafu::OptionExt;

use std::env;
use std::path::{Path, PathBuf};

struct LevelFilterVisitor;

impl<'de> Visitor<'de> for LevelFilterVisitor {
    type Value = LevelFilter;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a log level string (error, warn, info, debug, trace, or off)")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let lower = value.to_lowercase();
        match lower.as_str() {
            "error" => Ok(LevelFilter::Error),
            "warn" => Ok(LevelFilter::Warn),
            "info" => Ok(LevelFilter::Info),
            "debug" => Ok(LevelFilter::Debug),
            "trace" => Ok(LevelFilter::Trace),
            "off" => Ok(LevelFilter::Off),
            _ => Err(de::Error::unknown_variant(
                value,
                &["error", "warn", "info", "debug", "trace", "off"],
            )),
        }
    }
}

fn deserialize_level_filter<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(LevelFilterVisitor)
}

fn default_directory() -> PathBuf {
    PathBuf::from("./acceptarium")
}

fn default_glob() -> GlobPattern {
    GlobPattern::new("*.toml").unwrap()
}

fn default_verbosity() -> LevelFilter {
    LevelFilter::Warn
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[allow(unused)]
pub struct FilesystemConfig {
    #[serde(default = "default_directory")]
    pub directory: PathBuf,
    #[serde(default = "default_glob")]
    pub glob: GlobPattern,
    #[serde(default)]
    pub commit: bool,
    #[serde(default)]
    pub copy: bool,
    #[serde(default)]
    pub rename: bool,
    #[serde(default)]
    pub track: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[allow(unused)]
pub struct GitAnnexConfig {
    #[serde(default = "default_directory")]
    pub directory: PathBuf,
}

/* VISION MODEL NOTES
    String::from("gemma3:27b") // some data, but many mistakes
    String::from("granite3.2-vision:latest") // many fields wrong
    String::from("llama3.2-vision") // good extraction, bogus json
    String::from("qwen3.5:35b") // slow and best
    String::from("qwen3.5:9b") // fast and pretty good

    String::from("bakllava:7b") // summary of example with no json
    String::from("deepseek-ocr:3b") // no results
    String::from("gemma3:4b") // made up everything
    String::from("gemma3n:e4b") // made up some, used example for some
    String::from("glm-ocr:bf16") / used example
*/
fn default_vision_model() -> String {
    String::from("qwen3.5:9b") // fast and pretty good
}

fn default_llm_model() -> String {
    String::from("qwen3.5:9b")
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[allow(unused)]
pub struct VisionConfig {
    #[serde(default = "default_vision_model")]
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[allow(unused)]
pub struct LLMConfig {
    #[serde(default = "default_llm_model")]
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(unused)]
pub struct Config {
    pub project: PathBuf,
    #[serde(
        default = "default_verbosity",
        deserialize_with = "deserialize_level_filter"
    )]
    pub verbosity: LevelFilter,
    #[serde(rename(deserialize = "dry-run"))]
    pub dry_run: bool,
    pub overwrite: bool,
    #[serde(rename(deserialize = "config-file"))]
    pub config_file: Option<PathBuf>,
    #[serde(default)]
    pub processor: Processor,
    #[serde(default)]
    pub extractor: Extractor,
    pub(crate) storage: Option<StorageDriver>,
    pub(crate) filesystem: Option<FilesystemConfig>,
    // swap rename for alias for env var parsing, but then the TOML breaks.
    // #[serde(alias = "GITANNEX")]
    #[serde(rename(deserialize = "git-annex"))]
    pub(crate) git_annex: Option<GitAnnexConfig>,
    pub(crate) vision: Option<VisionConfig>,
    pub(crate) llm: Option<LLMConfig>,
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
            })
            .canonicalize()?;
        // Setup default config values
        let mut builder = LayeredConfig::builder()
            .set_default("project", discovered_project.to_str().unwrap())?
            .set_default("verbosity", "warn")?
            .set_default("dry-run", false)?
            .set_default("overwrite", false)?
            .set_default("processor", "manual")?
            .set_default("extractor", "manual")?;
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
                .set_default("config", path.to_str().context(NonUnicodePathSnafu)?)?
                .add_source(File::from(path.as_path()).required(true));
        }
        // Layer in environment variables
        builder = builder.add_source(
            Environment::with_prefix("acceptarium")
                .separator("_")
                .prefix_separator("_")
                .ignore_empty(true),
        );
        // Layer in config overrides
        let mut config_overrides = args.config.clone().into_iter();
        while let (Some(key), Some(value)) = (config_overrides.next(), config_overrides.next()) {
            builder = builder.set_override(&key, value)?;
        }
        // Layer in command line flags
        if args.verbosity.is_present() {
            let val = args.verbosity.log_level_filter().to_string().to_lowercase();
            builder = builder.set_override("verbosity", val)?;
        }
        if let Some(val) = deboolify(args.dry_run, args.no_dry_run) {
            builder = builder.set_override("dry-run", val)?;
        }
        if let Some(val) = deboolify(args.overwrite, args.no_overwrite) {
            builder = builder.set_override("overwrite", val)?;
        }
        match args.subcommand {
            Commands::Add {
                commit,
                no_commit,
                copy,
                no_copy,
                rename,
                no_rename,
                ..
            } => {
                if let Some(val) = deboolify(commit, no_commit) {
                    builder = builder.set_override("filesystem.commit", val)?;
                }
                if let Some(val) = deboolify(copy, no_copy) {
                    builder = builder.set_override("filesystem.copy", val)?;
                }
                if let Some(val) = deboolify(rename, no_rename) {
                    builder = builder.set_override("filesystem.rename", val)?;
                }
            }
            Commands::List {
                // tracked,
                // no_tracked,
                ..
            } => {
                // if let Some(val) = deboolify(tracked, no_tracked) {
                //     builder = builder.set_override("filesystem.tracked", val)?;
                // }
            }
            Commands::Process {
                processor,
                extractor,
                ..
            } => {
                if let Some(val) = processor {
                    let val: String = val.to_possible_value().unwrap().get_name().into();
                    builder = builder.set_override("processor", val)?;
                }
                if let Some(val) = extractor {
                    let val: String = val.to_possible_value().unwrap().get_name().into();
                    builder = builder.set_override("extractor", val)?;
                }
            }
            Commands::Get { .. } => {}
            Commands::Set { .. } => {}
            Commands::Remove { .. } => {}
            Commands::Run { .. } => {}
            Commands::Status {} => {}
            Commands::External(_) => {}
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
