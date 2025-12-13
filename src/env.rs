use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Environment configuration, a dotenv file path
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DotenvConfig {
    /// The path to the dotenv file
    pub path: String,
    /// Optional list of specific variables to load from the command output
    pub vars: Option<BTreeMap<String, String>>,
}

/// Environment configuration, run a command to load environment variables
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvCommand {
    /// The command to run to load environment variables
    pub command: String,
    /// Optional list of specific variables to load from the command output
    pub vars: Option<BTreeMap<String, String>>,
}

/// Environment configuration, a set of key-value pairs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvVars {
    /// The environment variables as key-value pairs
    pub vars: BTreeMap<String, String>,
}

/// An environment definition, either a dotenv file or a command to load envs
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Env {
    /// A dotenv file path
    Dotenv(String),
    /// A dotenv file with specific configuration
    Config(DotenvConfig),
    /// A command to load environment variables
    Command(EnvCommand),
    /// A set of environment variables defined directly
    Vars(EnvVars),
}

pub struct RunnerEnv {
    pub command: Option<String>,
    pub vars: Option<BTreeMap<String, String>>,
}

impl Env {
    pub fn get_env_vars(&self) -> RunnerEnv {
        match self {
            Env::Dotenv(path) => {
                // Load from dotenv file
                let env_vars = dotenvy::from_path_iter(path)
                    .unwrap()
                    .filter_map(|item| item.ok())
                    .collect();

                RunnerEnv {
                    command: None,
                    vars: Some(env_vars),
                }
            }
            Env::Config(config) => {
                // Load from dotenv file with specific vars
                let mut env_vars: BTreeMap<String, String> = dotenvy::from_path_iter(&config.path)
                    .unwrap()
                    .filter_map(|item| item.ok())
                    .collect();

                // Add extra vars if specificied
                if let Some(vars) = &config.vars {
                    for (key, value) in vars {
                        env_vars.insert(key.clone(), value.clone());
                    }
                }

                RunnerEnv {
                    command: None,
                    vars: Some(env_vars),
                }
            }
            Env::Command(cmd) => RunnerEnv {
                command: Some(cmd.command.clone()),
                vars: cmd.vars.clone(),
            },
            Env::Vars(vars) => RunnerEnv {
                command: None,
                vars: Some(vars.vars.clone()),
            },
        }
    }
}

/// Configuration for environments
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvConfig {
    /// A map of environment names to their definitions
    pub envs: BTreeMap<String, Env>,
    /// An optional default environment name
    pub default: Option<String>,
}

/// An environment definition, either a full config or a list of supported envs
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Envs {
    /// A full environment configuration
    Config(EnvConfig),
    /// A list of supported environment names, defined in parent group(s)
    Supported(Vec<String>),
}

pub fn match_env<'a>(
    envs: BTreeMap<&'a String, &'a Env>,
    default: Option<&'a String>,
    args: &'a [&'a str],
) -> Result<Option<(&'a Env, &'a [&'a str])>> {
    if envs.is_empty() {
        return Ok(None);
    }

    // If there are environments defined, but no args and no default, return an error
    if args.is_empty() && !envs.is_empty() && default.is_none() {
        return Err(anyhow::anyhow!(
            "No environment specified, and no default environment is set"
        ));
    }

    // If no args provided, use default if available
    if args.is_empty() {
        if let Some(default_name) = default {
            let env = envs.get(default_name);
            if let Some(env) = env {
                return Ok(Some((env, args)));
            } else {
                return Err(anyhow::anyhow!(
                    "Default environment '{}' not found in available environments",
                    default_name
                ));
            }
        }
    }

    let first_arg = args.get(0).ok_or(anyhow::anyhow!(
        "No environment specified, and no default environment is set"
    ))?;

    if let Some(env) = envs.get(&first_arg.to_string()) {
        return Ok(Some((env, &args[1..])));
    } else {
        return Err(anyhow::anyhow!(
            "Environment '{}' not found in available environments",
            first_arg
        ));
    }
}
