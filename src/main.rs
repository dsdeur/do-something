mod cli;
mod commands;
mod commands2;
mod config;
mod dir;

use crate::cli::run;

fn main() {
    run().unwrap();
}
