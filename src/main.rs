mod cli;
mod commands;
mod config;
mod dir;
mod runner;
mod tui;

use crate::cli::run;

fn main() {
    run().unwrap();
}
