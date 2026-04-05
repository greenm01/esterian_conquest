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

const MIN_COLS: u16 = 160;
const MIN_ROWS: u16 = 40;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (cols, rows) = crossterm::terminal::size()?;
    if cols < MIN_COLS || rows < MIN_ROWS {
        eprintln!(
            "nc-dash requires at least {}×{} terminal (yours is {}×{}).",
            MIN_COLS, MIN_ROWS, cols, rows
        );
        eprintln!("Resize your terminal or use nc-game for the classic 80×25 interface.");
        std::process::exit(1);
    }

    eprintln!("nc-dash: terminal {}×{} — OK", cols, rows);
    eprintln!("nc-dash: scaffold only — not yet playable.");

    Ok(())
}
