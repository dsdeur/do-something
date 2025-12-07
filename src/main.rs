mod cli;
mod commands;
mod config;
mod dir;
mod ds_file;
mod help;
mod runner;

use crate::cli::run;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
