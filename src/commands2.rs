use crate::commands::{CommandDefinition, Group};

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
    ) -> Vec<(Vec<String>, CommandDefinition, Vec<&Group>)> {
        let mut commands = Vec::new();

        self.walk_commands(&mut |key, cmd, parents| {
            if matches.starts_with(key) {
                commands.push((
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
            .map(|(key, _, _)| key.len())
            .max()
            .unwrap_or(0);

        // Filter the most deeply matching commands
        commands
            .into_iter()
            .filter(|(key, _, _)| key.len() == max_depth)
            .collect()
    }
}
