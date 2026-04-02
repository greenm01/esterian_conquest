#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    if let Err(err) = nc_game::cli::run(std::env::args()) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
