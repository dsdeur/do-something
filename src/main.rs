mod cli;
mod commands;
mod config;
mod dir;
mod ds_file;
mod runner;
mod tui;

use crate::cli::run;

fn main() {
    run().unwrap();
}
