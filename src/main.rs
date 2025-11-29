mod cli;
mod commands;
mod config;
mod dir;

use crate::cli::run;

fn main() {
    run().unwrap();
}
