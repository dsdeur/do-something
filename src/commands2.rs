use crate::commands::{CommandDefinition, Group};

fn get_commmand_keys<'a>(
    keys: &[&'a str],
    command: &CommandDefinition,
    parents: &[&'a Group],
) -> Vec<Vec<&'a str>> {
    // let mut all_keys = Vec::new();
    let mut parent_keys = Vec::new();

    // Collect all parent keys
    for (i, group) in parents.iter().enumerate() {
        let key = keys[i];

        match group.mode {
            // Only collect group aliases if the group is namespaced (default)
            Some(crate::commands::GroupMode::Namespaced) | None => {
                // Add the main key of the group
                let mut keys = vec![key];

                // Add the aliases if they exist
                if let Some(aliases) = &group.aliases {
                    for alias in aliases {
                        keys.push(alias);
                    }
                }

                parent_keys.push(keys);
            }
            Some(crate::commands::GroupMode::Flattened) => {
                continue;
            }
        }
    }

    // Add the command key
    let last_key = keys.last().unwrap_or(&"");
    let mut command_keys = vec![*last_key];

    // Add the command aliases if they exist
    match command {
        CommandDefinition::Command(_) => (),
        CommandDefinition::CommandConfig(command) => {
            if let Some(aliases) = &command.aliases {
                for alias in aliases {
                    command_keys.push(alias);
                }
            }
        }
        CommandDefinition::Group(group) => {
            if let Some(aliases) = &group.aliases {
                for alias in aliases {
                    command_keys.push(alias);
                }
            }
        }
    }

    parent_keys
}

fn get_match_score(command_keys: &Vec<Vec<&str>>, matches: &[&str], include_nested: bool) -> usize {
    let mut score = 0;

    for (i, key) in matches.iter().enumerate() {
        // Rest params, we are not interested in them
        if i >= command_keys.len() {
            break;
        }

        // Check if the key matches any of the command keys
        if command_keys[i].contains(key) {
            score += 1;
        } else {
            // If it doesn't match, we stop scoring
            break;
        }
    }

    if !include_nested && command_keys.len() > matches.len() {
        // If we are not including nested commands, the key can only be smaller (rest args) or equal to the matches
        return 0;
    }

    score
}

impl Group {
    pub fn walk_tree<'a>(
        &'a self,
        keys: &mut Vec<&'a str>,
        parents: &mut Vec<&'a Group>,
        on_command: &mut dyn FnMut(&[&str], &CommandDefinition, &[&'a Group]),
    ) {
        parents.push(self);

        for (key, command) in self.commands.iter() {
            keys.push(key);
            on_command(&keys, command, parents);

            if let CommandDefinition::Group(group) = command {
                group.walk_tree(keys, parents, on_command);
            }

            keys.pop();
        }

        parents.pop();
    }

    pub fn walk_commands<'a>(
        &'a self,
        on_command: &mut dyn FnMut(&[&str], &CommandDefinition, &[&'a Group]),
    ) {
        let mut keys = Vec::new();
        let mut parents = Vec::new();
        self.walk_tree(&mut keys, &mut parents, on_command);
    }

    pub fn get_matches(
        &self,
        matches: Vec<&str>,
        include_nested: bool,
    ) -> Vec<(usize, Vec<String>, CommandDefinition, Vec<&Group>)> {
        let mut commands = Vec::new();

        self.walk_commands(&mut |key, cmd, parents| {
            let command_keys = get_commmand_keys(key, cmd, parents);
            let score = get_match_score(&command_keys, &matches, include_nested);

            if score > 0 {
                commands.push((
                    score,
                    key.to_vec()
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>(),
                    cmd.clone(),
                    parents.iter().copied().collect(),
                ));
            }
        });

        // Determine the maximum depth of the matching commands
        let max_depth = commands
            .iter()
            .map(|(score, _, _, _)| *score)
            .max()
            .unwrap_or(0);

        // Filter the most deeply matching commands
        commands
            .into_iter()
            .filter(|(score, _, _, _)| *score == max_depth)
            .collect()
    }
}
