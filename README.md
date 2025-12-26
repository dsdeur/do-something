# Do Something
A simple yet powerful command runner.

## Highlights
- **Quick access**: Two letter binary `ds` conveniently located on the keyboard for blazingly fast access.
- **Fuzzy search**: TUI with fuzzy search to easily find available commands
- **Grouping**: Easy grouping and nesting of commands
- **Aliases**: Create shortcuts and aliases for commands and groups
- **Organization**: Define commands in multiple files, for specific folders, or globally available.
- **Environments**: Manage your environments, load dotenv files, run custom commands, and/or define custom environment vars.
- **JSON Config**: Simple configuration in JSON files.

## Disclaimer
This is a work in progress side project, alpha version, use at your own risk.
The basics should work fairly well, but I would not be surprised if there are bugs and unhandled edge cases.

Only tested on Mac OS and Ubuntu, it will probably not work on Windows.

## Install
```bash
cargo install do-something
```

## How to use
Define your commands in a `do.json` file:

```json
{
  "commands": {
    "hello-world": "echo 'Hello, world!'"
  }
}
```

Calling the command by it's key:
```bash
ds hello-world
```

That's it!

### TUI
You can run the TUI (build with Ratatui) that has fuzzy search (powered by Nucleo) to easily find available commands and run them:
```bash
ds
```
Just type to search, use up/down arrow keys to navigate the list, and press `Enter` to run the selected command. You can search for the command, the aliases, the file name, or the actual command that will be run.

You can exit with the `Escape` key or `ctrl + c`


## Grouping and nesting
Commands can be nested in groups:
```json
{
  "commands": {
    "app": {
      "commands": {
        "dev": "pnpm run dev",
        "build": "pnpm run build"
      }
    },
    "api": {
      "commands": {
        "dev": "fastapi dev main.py"
      }
    }
  }
}
```

Then you can run these:
```bash
ds app dev
ds app build
ds api dev
```

There is no limit on how deep you can nest.

### Default group command
If you add a command named `default` it will be called if the group is ran without extra args.
You can also change which command to run by default by setting the `default` setting:

```json
{
  "commands": {
    "app": {
      "default": "dev",
      "commands": {
        "dev": "pnpm run dev",
        "build": "pnpm run build"
      }
    },
    "api": {
      "default": "dev",
      "commands": {
        "dev": "fastapi dev main.py"
      }
    }
  }
}
```

So now, you can call the group without extra arguments:
```bash
ds app
```
And that will run `ds app dev`.

## Aliasing
You can add aliases for groups and individual commands:

```json
{
  "commands": {
    "git": {
      "aliases": ["g"]
      "commands": {
        "pull": {
          "aliases": ["pl"],
          "command": "git pull"
        },
        "push": {
          "aliases": ["pu"],
          "command": "git push"
        },
      },
    }
  }
}
```

Then you can run these commands by any alias for the groups and commands, for example:
```bash
ds g pl
ds git pl
ds g pull
ds git pull
```
This makes it really easy to create shortcuts for frequently used commands.

If you run `ds` it will show you the available aliases for each command.

Tip: You can create an alias in your zsh/bash/fish to remap for example `g` to `ds g`, which would allow skipping the `ds`.


## Environments
You can define (multiple) environments on groups and commands.
Environments are merged/overwritten if they are defined on multiple levels.

```json
{
  "commands": {
    "build": "pnpm run build"
  },
  "envs": {
    "dev": ".env.dev",
    "prod": ".env.prod"
  }
}
```

Now build will be turned into two options, and the environment variables will be loaded before running the command:
```bash
ds build dev
ds build prod
```

### Default environment
You can also define a default env:
```json
{
  "commands": {
    "build": "pnpm run build"
  },
  "default_env": "dev",
  "envs": {
    "dev": ".env.dev",
    "prod": ".env.prod"
  }
}
```
Now running `ds build` will load the dev environment file.

### Commands, and custom variables
You can also run a command to load the environment, for example to use a secret manager, as well as define custom variables

```json
{
  "commands": {
    "build": "pnpm run build"
  },
  "default_env": "dev",
  "envs": {
    "dev": {
      "path": ".env.dev",
      "vars": {
        "ENVIRONMENT": "development"
      }
    },
    "prod": {
      "command": "op run --",
      "vars": {
        "ENVIRONMENT": "production"
      }
    },
  }
}
```

## Flatten groups
Sometimes you want to group commands, so you can add settings and environments, but not have an extra word to type. For this set the `mode` group setting to `Flattened`:

```json
{
  "commands": {
    "env-commands": {
      "commands": {
        "build": "npm build",
        "dev": "npm dev"
      },
      "default_env": "dev",
      "envs": {
        "dev": ".env.dev",
        "prod": ".env.prod"
      },
      "mode": "Flattened"
    },
    "other-command": "echo 'Hello'"
  },
}
```

Now you can call the commands, as if they were not grouped:
```bash
ds dev
ds build prod
```

While still benefitting from the group configuration like environments in the example.


## Multiple files
You can define your files in multiple places:
- In a folder or git root, will be discovered in the closest git root.
- `~/config/do-something/ds.json`

Or place your files wherever you want and include them in the config (`~/config/do-something/config.json`), for example:
```json
{
  "ds_files": ["~/.config/do-something/commands/*.json"],
}
```

## Root
You can specific where commands should run and be available by setting the root option:
```json
{
  "commands": {
    "hello": "echo 'Hello, world!'"
  },
  "root": {
    "path": "~/path/where/to/run",
  }
}
```

Now the commands in this file will be run from that path. You can define the root on files, groups and commands, just like any other settings.

### Scoping
Sometimes you can't add a `ds.json` in a project, for example if it's not your project, open source, you don't want to bother your team with yet another tool.

To "inject" commands in a folder, you can use the `scope` setting of the root config.

You have three scoping options:
- `Global` (default): The commands are always available
- `GitRoot`: The commands are available in the git folder
- `Exact`: The commands only run in the folder that is set in `path`

```json
{
  "commands": {
    "hello": "echo 'Hello, world!'"
  },
  "root": {
    "path": "~/path/to/git/root",
    "scope": "GitRoot"
  }
}
```

Now the commands will be available anywhere in `~/path/to/git/root` but not outside of it.

## Config
You can confige Do Something by creating a config file `~/config/do-something/config.json`.

Settings:
- `ds_files`: Define where to look for command files, you can use glob patterns.
- `on_conflict`: What to do when there are two commands with the same key.
  - `Override` (Default): The last command is used
  - `Error`: Instead of running a command it will throw and error


## Why another command runner?
There are many great options for command runners. I made this one to fit my exact needs: Simple to configure, a limited set of powerful features, and easy to use. Secondarily, I wanted to improve my Rust skills.

### Alternatives
To keep Do Something simple, it is limited in features and capabilities, and alpha version software. If you need something more power, customizability, and mature, I suggest using [Just](https://github.com/casey/just), which is incredibly powerful and full featured, and overall amazing application.

They also have a [convenient list of alternatives](https://github.com/casey/just?tab=readme-ov-file#alternatives-and-prior-art), there is plenty of choices.


## Why JSON?
Preference. JSON is in my opinion the easiest format for configuration. It's simple to read, easy to learn, support is great, and makes it very easy to nest and group commands, which is one of the key features of Do Something. In other words: It's simple yet powerful, which makes it a great fit.

Of course it has it's limitations. I'm considering adding JSONC or JSON5 support. I think TOML isn't bad either, it works great for defining dependencies (Cargo.toml, pyproject.toml), I just find the nesting not as easy to work with, that said, I might try adding support as it seems not too difficult to add.

## Contributing
I have limited time to work on this, but PR's are very welcome. If you have plans to add features, please discuss it first in issues, as I do intent to keep it simple.

## License
This project is licensed under the [MIT License](./LICENSE).
