use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Environment configuration, a dotenv file path
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DotenvConfig {
    /// The path to the dotenv file
    pub path: String,
    /// Optional list of specific variables to load from the command output
    pub vars: Option<BTreeMap<String, String>>,
}

/// Environment configuration, run a command to load environment variables
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct EnvCommand {
    /// The command to run to load environment variables
    pub command: String,
    /// Optional list of specific variables to load from the command output
    pub vars: Option<BTreeMap<String, String>>,
}

/// Environment configuration, a set of key-value pairs
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct EnvVars {
    /// The environment variables as key-value pairs
    pub vars: BTreeMap<String, String>,
}

/// An environment definition, either a dotenv file or a command to load envs
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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

/// The environment variables and/or command to actually run
pub struct RunnerEnv {
    pub command: Option<String>,
    pub vars: Option<BTreeMap<String, String>>,
}

impl Env {
    /// Get the environment variables and/or command to run from the config
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

/// Match an environment from the provided args and default
pub fn match_env<'a>(
    envs: BTreeMap<&'a String, &'a Env>,
    default_env: Option<&'a str>,
    args: &'a [&'a str],
) -> Result<Option<(&'a Env, &'a [&'a str])>> {
    if envs.is_empty() {
        return Ok(None);
    }

    // If there are environments defined, but no args and no default, return an error
    if args.is_empty() && !envs.is_empty() && default_env.is_none() {
        return Err(anyhow::anyhow!(
            "No environment specified, and no default environment is set"
        ));
    }

    if let Some(&env) = args.first().and_then(|&s| envs.get(&s.to_string())) {
        Ok(Some((env, &args[1..])))
    } else {
        if let Some(default_key) = default_env {
            if let Some(&env) = envs.get(&default_key.to_string()) {
                return Ok(Some((env, args)));
            } else {
                return Err(anyhow::anyhow!(
                    "Environment not found, and default environment '{}' is not found",
                    default_key
                ));
            }
        }

        Err(anyhow::anyhow!(
            "Environment not found, and no default environment is set",
        ))
    }
}

/// Get an environment by key or default
pub fn get_env_by_key<'a>(
    envs: BTreeMap<&'a String, &'a Env>,
    key: Option<String>,
    default_env: Option<&str>,
) -> Option<&'a Env> {
    let mut env = None;
    if let Some(env_key) = key {
        env = envs.get(&env_key.to_string());
    };

    if let Some(default) = default_env
        && env.is_none()
    {
        env = envs.get(&default.to_string());
    }

    env.copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_env() {
        let mut envs: BTreeMap<String, Env> = BTreeMap::new();
        envs.insert("dev".to_string(), Env::Dotenv(".env.dev".to_string()));
        envs.insert(
            "prod".to_string(),
            Env::Vars(EnvVars {
                vars: [("MODE".to_string(), "prod".to_string())].into(),
            }),
        );

        let envs_ref: BTreeMap<&String, &Env> = envs.iter().map(|(k, v)| (k, v)).collect();

        let result = match_env(BTreeMap::new(), None, &["dev"]).unwrap();
        assert!(result.is_none(), "no envs should return None");

        let result = match_env(envs_ref.clone(), None, &["dev", "extra"]).unwrap();
        let (env, remaining) = result.unwrap();
        assert_eq!(env, envs.get("dev").unwrap());
        assert_eq!(remaining, ["extra"]);

        let result = match_env(envs_ref.clone(), Some("prod"), &[]).unwrap();
        let (env, remaining) = result.unwrap();
        assert_eq!(env, envs.get("prod").unwrap());
        assert!(remaining.is_empty());

        let err = match_env(envs_ref.clone(), None, &[]).unwrap_err();
        assert!(err.to_string().contains("No environment specified"));

        let err = match_env(envs_ref.clone(), Some("missing"), &[]).unwrap_err();
        assert!(
            err.to_string()
                .contains("default environment 'missing' is not found")
        );

        let err = match_env(envs_ref, None, &["unknown"]).unwrap_err();
        assert!(err.to_string().contains("Environment not found"));
    }
}
