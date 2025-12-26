# Do Something
A simple yet powerful command runner, with TUI and fuzzy search.

<br/>
<img width="700" alt="image" src="./demo.gif" />

<br/>

## Highlights
- **Quick access**: Two-letter binary `ds` conveniently located on the keyboard for blazingly fast access.
- **Fuzzy search**: TUI with fuzzy search to easily find available commands
- **Grouping**: Easy grouping and nesting of commands
- **Aliases**: Create shortcuts and aliases for commands and groups
- **Organization**: Define commands in multiple files, for specific folders, or globally available.
- **Environments**: Manage your environments, load dotenv files, run custom commands, and/or define custom environment vars.
- **JSON Config**: Simple configuration in JSON files.

<br/>

<img width="300" alt="image" src="https://github.com/user-attachments/assets/8843abe1-9083-4eeb-ab77-deb0c4c9d205" />

<br/><br/>

## Disclaimer & warning
This is a work in progress side project, unpublished, alpha version, use at your own risk.

The basics should work fairly well, but I would not be surprised if there are bugs and unhandled edge cases.

Only tested on Mac OS and Ubuntu, it will probably not work on Windows.

WARNING: Do not run commands from untrusted files. Always check what you are running.

<br/>

## Install
Clone the repo. Build and install:
```bash
git clone git@github.com:dsdeur/do-something.git
cd do-something
cargo build --release && cargo install --path .
```

<br/>

## How to use
Define your commands in a `ds.json` file:

```json
{
  "commands": {
    "hello-world": "echo 'Hello, world!'"
  }
}
```


Calling the command by its key:
```bash
ds hello-world
```

That's it!

<br/>

### TUI
You can run the TUI (built with Ratatui) that has fuzzy search (powered by Nucleo) to easily find available commands and run them:
```bash
ds
```
Just type to search, use up/down arrow keys to navigate the list, and press `Enter` to run the selected command. You can search for the command, the aliases, the file name, or the actual command that will be run.

You can exit with the `Escape` key or `Ctrl+C`.

<br/>

## Grouping and nesting
To organize commands you can group and nest them:
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

Then you can run them by their group and key:
```bash
ds app dev
ds app build
ds api dev
```

<br/>

### Default group command
If you add a command named `default` it will be called if the group is run without extra args.
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

<br/>

### Conflicts & nested options
There is no limit on how deep you can nest. If there are conflicts, by default the last command in the file is wins, the order of the files is:
- `~/config/do-something/ds.json`
- Paths defined in `~/config/do-something/config.json`
- `do.json` in current git root
- `do.json` in the current folder

This means you can define a global command in your config, and then overwrite it per project. See the Config section on how to error instead.

Options specified on nested groups and commands overwrite those higher up in the tree. Environments are merged by key.

<br/>

## Aliasing
To make convenient shortcuts for your commonly used commands you can add aliases. You can add aliases for groups and individual commands:

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
        }
      }
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

If you run `ds` it will show you the available aliases for each command.

Tip: You can create an alias in your zsh/bash/fish to remap for example `g` to `ds g`, which will allow skipping the `ds` making it even quicker to access, while still having the convenience of ds files.

<br/>

## Environments
It's not uncommon to want to run the same commands with different environment files and variables. This is made easy by configuring environments on groups, and commands. 
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

Now the `build` command will be turned into two options, and the environment variables will be loaded (using dotenvy) before running the command:
```bash
ds build dev
ds build prod
```

<br/>

### Default environment
You can also define a default environment:
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
Now running `ds build` will load the dev environment file, while you still need to run `ds build prod` to load the prod environment file.

<br/>

### Commands, and custom variables
Alternitvely to dotenv files you can also prefix your commands to load the environment, for example to use a secret manager or custom script, as well as define custom variables:

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
      "command_prefix": "load-env",
      "vars": {
        "ENVIRONMENT": "production"
      }
    }
  }
}
```
Running `ds build prod` will run `load-env pnpm run build`. This gives the flexibility to use a different cli to run the command, or use `&&` to first run the env, then the command.

WARNING: Do not put secrets in your do.json file. Use a secrets manager and/or environment files instead.

<br/>

## Flatten groups
Sometimes you want to group commands so you can add common settings and environments, but not have to type an extra word. You can flatten groups by setting the `mode` group setting to `flattened`:

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
      "mode": "flattened"
    },
    "other-command": "echo 'Hello'"
  }
}
```

Now you can call the commands, as if they were not grouped:
```bash
ds dev
ds build prod
```

While still having the convenience of shared environments.

<br/>

## Multiple files
You can place your ds json files in multiple places:
- In a folder or git root, it will be discovered and available in the closest git root.
- In the config folder of do-something `~/.config/do-something/ds.json`

Or place your files wherever you want and include them in the config `ds_files` option (`~/.config/do-something/config.json`), for example I have it set up to read any json file in the commands folder in the config folder:
```json
{
  "ds_files": ["~/.config/do-something/commands/*.json"]
}
```

<br/>

## Root
If you want to have commands that need to be run in a specific location globally available, you can specify where commands should run by setting the root option:
```json
{
  "commands": {
    "hello": "echo 'Hello, world!'"
  },
  "root": {
    "path": "~/path/where/to/run"
  }
}
```

Now the commands in this file will be run from that path. You can define the root on files, groups and commands, just like any other settings.

<br/>

### Scoping
Sometimes you can't add a `ds.json` file in a project, for example if it's not your project, an open source project, or you just don't want to bother your team with yet another tool.
Definiing them outside of it would conflict with other projects (for example `ds dev` is used in multiple projects).

To solve this you can use the `scope` setting in the root config to limit where commands are available, based on the path.

You have three scoping options:
- `global` (default): The commands are always available, regardless of the current location
- `git_root`: The commands are available in the git folder
- `exact`: The commands only run in the folder that is set in `path`

```json
{
  "commands": {
    "hello": "echo 'Hello, world!'"
  },
  "root": {
    "path": "~/path/to/git/root",
    "scope": "git_root"
  }
}
```

Now the commands will be available anywhere in `~/path/to/git/root` but not outside of it.

<br/>

## Config
You can configure Do Something by creating a config file `~/.config/do-something/config.json`.

Settings:
- `ds_files`: Define where to look for command files, you can use glob patterns.
- `on_conflict`: What to do when there are two commands with the same key.
  - `override` (Default): The last command is used
  - `error`: Instead of running a command it will throw an error.

Note: Error is in theory a bit slower, as it will have to read all files to know if there is a conflict, instead of exiting when the first match is found. In practice this should make no difference unless you have many an enormous amount of files and commands. 

<br/>

## Why another command runner?
There are many great options for command runners. I made this one to fit my exact needs: Simple to configure, a limited set of powerful features, and super easy to use. It's optimized for convenience and efficiency at the cost of (some) flexibility and customization. Also, I have been wanting to build a project in Rust for a while, and this seemed like a great match.

<br/>

### Alternatives
To keep Do Something simple, it is limited in features and capabilities, and alpha version software. If you need something more powerful, customizable, and mature, I suggest using [Just](https://github.com/casey/just), which is incredibly powerful and full-featured, and an overall amazing application.

They also have a [convenient list of alternatives](https://github.com/casey/just?tab=readme-ov-file#alternatives-and-prior-art), there are plenty of choices.

<br/>

## Why JSON?
Preference. JSON is in my opinion the easiest format for configuration. It's simple to read, easy to learn, support is great, and makes it very easy to nest and group commands, which is one of the key features of Do Something. In other words: It's simple yet powerful, which makes it a great fit.

Of course JSON has its limitations. I'm considering adding JSONC or JSON5 support. I think TOML isn't bad either, it works great for defining dependencies (e.g. Cargo.toml, pyproject.toml), I just find nesting not as easy to work with TOML, that said, I might try adding support in the future. 

<br/>

## Was this vibe-coded?
Mostly no, except for 1 or 2 of the tests. I have used LLM's extensively to learn, review the code, explain errors, and workshop ideas and alternative approach. This has been incredibly helpful. My goal was learning the language, so I did write it myself (and rewrote it multiple times) to improve my skills and understanding.

<br/>

## Contributing
I have limited time to work on this, but PRs are very welcome. If you have plans to add features, please discuss it first in issues, as I do intend to keep it simple.

<br/>

## License
This project is licensed under the [MIT License](./LICENSE).
