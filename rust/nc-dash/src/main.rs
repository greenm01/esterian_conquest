//! nc-dash — Full-screen dashboard TUI for Nostrian Conquest.
//!
//! A modern three-column terminal dashboard replacing the legacy 80×25
//! BBS-style interface. Built for SSH and local play on 1920×1200+
//! displays. See docs/dash/architecture.md for the full design spec.

mod app;
mod layout;
mod overlays;
mod panels;
mod popups;
mod startup;
mod theme;

use nc_session::args::detect_color_mode;

const MIN_COLS: u16 = 160;
const MIN_ROWS: u16 = 40;

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = args.into_iter().collect();

    if matches!(args.get(1).map(String::as_str), Some("--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let game_dir = args
        .get(1)
        .map(std::path::PathBuf::from)
        .ok_or("Usage: nc-dash <game-dir>")?;

    let color_mode = detect_color_mode();

    let (cols, rows) = crossterm::terminal::size()?;
    if cols < MIN_COLS || rows < MIN_ROWS {
        eprintln!(
            "nc-dash requires at least {}×{} terminal (yours is {}×{}).",
            MIN_COLS, MIN_ROWS, cols, rows
        );
        eprintln!("Resize your terminal or use nc-game for the classic 80×25 interface.");
        std::process::exit(1);
    }

    eprintln!("nc-dash: {}×{} terminal — OK", cols, rows);
    eprintln!("nc-dash: game_dir = {}", game_dir.display());
    eprintln!("nc-dash: color_mode = {:?}", color_mode);
    eprintln!("nc-dash: scaffold only — not yet playable.");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(std::env::args())
}

fn print_usage() {
    eprintln!("nc-dash — Nostrian Conquest full-screen dashboard");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    nc-dash <game-dir> [OPTIONS]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --help, -h    Show this help");
}
