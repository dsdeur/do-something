use do_something::cli::run;

fn main() {
    do_something::tui::run_tui(vec![]).unwrap();
    // if let Err(e) = run() {
    //     eprintln!("Error: {:#}", e);
    //     std::process::exit(1);
    // }
}
