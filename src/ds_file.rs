use crate::{
    command::Command,
    dir::collapse_to_tilde,
    group::{Group, Walk},
    help::{HelpGroup, HelpRow},
};
use anyhow::{Result, anyhow};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Represents a command file, which contains a group of commands
/// Mainly to have a common interface for loading and matching commands.
#[derive(Clone)]
pub struct DsFile {
    pub group: Group,
    pub file_name: String,
    pub path: PathBuf,
    pub path_string: String,
}

/// Represents a match for a command, containing the score and keys
#[derive(Debug)]
pub struct Match {
    pub file_path: PathBuf,
    /// The score of the match, which is the number of levels that match
    pub score: usize,
    /// The path in the command file that matched
    pub keys: Vec<String>,
}

impl Match {
    /// Calculate the match score for a command based on the provided matches
    /// - The score is the number of levels that match
    pub fn from_command(
        file_path: PathBuf,
        keys: &[&str],
        alias_keys: &[Vec<&str>],
        target: &[&str],
    ) -> Option<Self> {
        let mut score = 0;

        for (i, key) in target.iter().enumerate() {
            match alias_keys.get(i) {
                Some(keys) if keys.contains(key) => {
                    score += 1;
                }
                _ => {
                    // If the key is not found, we stop scoring
                    break;
                }
            }
        }

        // If we are not including nested commands, we need to match the score to the alias keys length
        let is_nested = score < alias_keys.len();

        // If the score is 0, or if we are in nested mode and the alias keys are longer than the matches, we return None
        if is_nested || score == 0 {
            return None;
        }

        // If the score is greater than 0, we return a match
        // Only do all the copying if we have an actual match
        Some(Match {
            file_path,
            score,
            keys: keys.iter().map(|s| s.to_string()).collect(),
        })
    }
}

impl DsFile {
    pub fn from_json(json: String, path: impl AsRef<Path>) -> Result<Self> {
        let mut group: Group = serde_json::from_str(&json)?;
        let file_name = path
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Failed to get file name"))?;

        if group.name.is_none() {
            group.name = Some(file_name.clone());
        }

        let path_string = collapse_to_tilde(path.as_ref());
        if group.description.is_none() {
            group.description = Some(path_string.clone());
        }

        Ok(Self {
            group,
            file_name,
            path_string,
            path: path.as_ref().to_path_buf(),
        })
    }
    /// Load a group configuration from a file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(anyhow::anyhow!(
                "File not found: {}",
                path.as_ref().display()
            ));
        }

        let content = fs::read_to_string(&path)?;
        Self::from_json(content, path)
    }

    /// Get a command (and its parents) from the tree, based on the provided keys
    pub fn command_from_keys(&self, keys: &[String]) -> Result<(&Command, Vec<&Group>)> {
        let mut parents: Vec<&Group> = Vec::new();
        let mut command = None;

        for key in keys {
            let group = parents.last().copied().unwrap_or(&self.group);
            if let Some(cmd) = group.commands.get(key) {
                command = Some(cmd);

                if let Command::Group(group) = cmd {
                    parents.push(group);
                }
            } else {
                return Err(anyhow::anyhow!(
                    "No command found for keys: {}",
                    keys.join(" ")
                ));
            }
        }

        if let Some(command) = command {
            // Resolve the default command from groups
            // Make sure we also collect the parents correctly, if we are further nesting into defaults
            let command = command.resolve_default(&mut Some(&mut parents));

            // If the command is a group, we need to remove it from the parents
            // Otherwise we would have the group as command and as parent
            if let Command::Group(_) = command {
                parents.pop();
            }

            Ok((command, parents))
        } else {
            Err(anyhow::anyhow!(
                "No command found for keys: {}",
                keys.join(" ")
            ))
        }
    }

    /// Get the commands that match the provided matches
    pub fn matches(
        &self,
        target: &[&str],
        current_dir: impl AsRef<Path>,
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<Vec<Match>> {
        let mut matches = Vec::new();
        let mut err = None;

        self.group.walk_commands(&mut |keys, cmd, parents| {
            // If the command/group is not in scope, we skip it early to avoid unnecessary processing
            match cmd.is_in_scope(current_dir.as_ref(), git_root.as_ref()) {
                Err(_) => {
                    // Store the error and stop processing
                    err = Some(anyhow::anyhow!(
                        "Error determining scope for command: {}",
                        keys.join(" ")
                    ));
                    return Walk::Stop;
                }
                Ok(false) => return Walk::Skip,
                Ok(true) => {}
            }

            // Calculate the match score
            let command_keys = cmd.resolve_aliases(keys, parents);
            let m = Match::from_command(self.path.clone(), keys, &command_keys, target);

            if let Some(m) = m {
                matches.push(m);
            }

            Walk::Continue
        });

        // If there was an error, return it
        if let Some(err) = err {
            return Err(err);
        }

        // Determine the maximum depth of the matching commands
        let max_depth = matches
            .iter()
            .map(|Match { score, .. }| *score)
            .max()
            .unwrap_or(0);

        // Filter the most deeply matching commands
        let res = matches
            .into_iter()
            .filter(|Match { score, .. }| *score == max_depth)
            .collect();

        Ok(res)
    }

    /// Get the help rows for a match in the command file
    pub fn help_rows_for_match(
        &self,
        match_: &Match,
        current_dir: impl AsRef<Path>,
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<Vec<HelpRow>> {
        let (command, mut parents) = self.command_from_keys(&match_.keys)?;
        let mut keys: Vec<&str> = match_.keys.iter().map(|s| s.as_str()).collect();

        if let Command::Group(group) = command {
            group.help_rows(&self.path, &mut keys, &mut parents, current_dir, git_root)
        } else {
            Err(anyhow!("Only groups have help rows"))
        }
    }

    /// Get the help rows for the full command file
    pub fn help_rows(
        &self,
        current_dir: impl AsRef<Path>,
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<Vec<HelpRow>> {
        let mut keys = Vec::new();
        let mut parents = Vec::new();
        self.group.help_rows(
            &self.path,
            &mut keys,
            &mut parents,
            current_dir,
            git_root.as_ref(),
        )
    }

    /// Create the base group, mainly constructing the name and description
    /// - Uses the file name as the default name if not provided
    /// - Uses the path as the default description if not provided
    pub fn help_group(&self, rows: Vec<HelpRow>) -> HelpGroup {
        let group = &self.group;
        let file_name = &self.file_name;
        let name = group.name.as_ref().unwrap_or(file_name);
        let path = &self.path_string;
        let description = group.description.as_deref().unwrap_or(path);

        HelpGroup {
            name: name.to_string(),
            description: description.to_string(),
            search: path.to_string(),
            rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_score_cases() {
        struct Case {
            name: &'static str,
            keys: Vec<&'static str>,
            alias_keys: Vec<Vec<&'static str>>,
            target: Vec<&'static str>,
            expected_score: Option<usize>,
        }

        let cases = vec![
            Case {
                name: "Exact match",
                keys: vec!["group", "cmd"],
                alias_keys: vec![vec!["group"], vec!["cmd"]],
                target: vec!["group", "cmd"],
                expected_score: Some(2),
            },
            Case {
                name: "Match by alias",
                keys: vec!["group", "cmd"],
                alias_keys: vec![vec!["g", "group"], vec!["c", "cmd"]],
                target: vec!["g", "c"],
                expected_score: Some(2),
            },
            Case {
                name: "Partial match is nested and rejected",
                keys: vec!["group", "cmd"],
                alias_keys: vec![vec!["group"], vec!["cmd"]],
                target: vec!["group"],
                expected_score: None,
            },
            Case {
                name: "Mismatch at first segment",
                keys: vec!["group", "cmd"],
                alias_keys: vec![vec!["group"], vec!["cmd"]],
                target: vec!["other"],
                expected_score: None,
            },
            Case {
                name: "Mismatch after first segment",
                keys: vec!["group", "cmd"],
                alias_keys: vec![vec!["group"], vec!["cmd"]],
                target: vec!["group", "nope"],
                expected_score: None,
            },
            Case {
                name: "Extra target segments still match",
                keys: vec!["group"],
                alias_keys: vec![vec!["group"]],
                target: vec!["group", "extra"],
                expected_score: Some(1),
            },
        ];

        for case in cases {
            let result = Match::from_command(
                PathBuf::from("ds.json"),
                &case.keys,
                &case.alias_keys,
                &case.target,
            );

            let score = result.as_ref().map(|m| m.score);
            assert_eq!(score, case.expected_score, "{}", case.name);
        }
    }
}
