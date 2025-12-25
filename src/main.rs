use anyhow::Result;
use do_something::do_something::DoSomething;
use std::env;

/// Run the CLI application
pub fn run() -> Result<()> {
    let mut ds = DoSomething::new()?;

    // Get the command line arguments, skipping the first one (the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    if args_str.is_empty() {
        // If no arguments are provided, we show the fuzzy finder TUI
        ds.render_tui()
    } else {
        // Otherwise, we match the command
        ds.run_match(&args_str)
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{:#}", e);
        std::process::exit(1);
    }
}
