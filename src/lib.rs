//! # do-something
//!
//! A simple yet powerful command runner with TUI and fuzzy search.
//!
//! ## Features
//! - Fuzzy search TUI for discovering commands
//! - Environment management (dotenv, custom vars)
//! - Command grouping and aliases
//! - Flexible scoping (global, git-root, exact)
//!

pub mod command;
pub mod config;
pub mod dir;
pub mod do_something;
pub mod ds_file;
pub mod env;
pub mod group;
pub mod help;
pub mod runner;
pub mod tui;
