mod cli;
mod commands;
mod config;
mod dir;
mod runner;

use crate::cli::run;

fn main() {
    run().unwrap();
}
