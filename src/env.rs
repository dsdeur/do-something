use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::command::RootConfig;

/// Environment configuration, a dotenv file path
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DotenvConfig {
    /// The path to the dotenv file
    pub path: String,
    /// Optional list of specific variables to load from the command output
    pub vars: BTreeMap<String, String>,
}

/// Environment configuration, run a command to load environment variables
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvCommand {
    /// The command to run to load environment variables
    pub command: String,
    /// Optional root configuration, to define where the command is run from.
    pub root: Option<RootConfig>,
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
pub enum Env {
    /// A dotenv file path
    Dotenv(String),
    /// A command to load environment variables
    Command(EnvCommand),
    /// A set of environment variables defined directly
    Vars(EnvVars),
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
pub enum Envs {
    /// A full environment configuration
    Config(EnvConfig),
    /// A list of supported environment names, defined in parent group(s)
    Supported(Vec<String>),
}
