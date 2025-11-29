mod cli;
mod commands;
mod config;

use crate::cli::run;

fn main() {
    run().unwrap();
}
