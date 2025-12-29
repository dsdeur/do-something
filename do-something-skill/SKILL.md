---
name: do-something-skill
description: Discover and run repo/system CLI commands via the ds (do-something) runner. Use when asked to build, test, lint, or otherwise find and execute available commands in this repo or environment.
---

# Do-Something Command Runner

## Quick Start

- List available commands: `ds`
- Run a command: `ds <command>`

## Workflow

1. If unsure which command to run, call `ds` first and read the available commands.
2. Prefer the most specific command (e.g., `test`, `build`, `lint`) that matches the user request.
3. Run `ds <command>` and report results clearly.

## Notes

- `ds` without arguments prints the command catalog.
- Use `ds` for test/build/lint discovery instead of guessing tool-specific commands.
