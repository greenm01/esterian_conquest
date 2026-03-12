mod commands;
mod dispatch;
mod support;
mod usage;
mod workspace;

use std::env;
pub(crate) use workspace::INIT_FILES;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    dispatch::run_args(env::args().skip(1))
}
