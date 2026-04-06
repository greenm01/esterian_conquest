//! nc-dash — Full-screen dashboard TUI for Nostrian Conquest.

mod app;
mod layout;
mod overlays;
mod panels;
mod popups;
mod startup;
mod theme;

use nc_data::CampaignStore;
use nc_session::args::detect_color_mode;
use nc_ui::{OutputEncoding, StdoutTerminal};
use nc_ui::ScreenGeometry;

use app::state::DashApp;
use layout::geometry::{MIN_COLS, MIN_ROWS};

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

    // Check terminal size.
    let (cols, rows) = crossterm::terminal::size()?;
    if cols < MIN_COLS || rows < MIN_ROWS {
        eprintln!(
            "nc-dash requires at least {}×{} terminal (yours is {}×{}).",
            MIN_COLS, MIN_ROWS, cols, rows
        );
        eprintln!("Resize your terminal or use nc-game for the classic 80×25 interface.");
        std::process::exit(1);
    }

    let geometry = layout::geometry::capped_geometry(cols as usize, rows as usize);

    // Load game data.
    let campaign_store = CampaignStore::open_default_in_dir(&game_dir)?;
    let game_data = campaign_store.load_latest_runtime_game_data()?;

    // Default to player 1. Future: resolve from args/session.
    let player_record_index_1_based = 1;

    let color_mode = detect_color_mode();
    let mut terminal = StdoutTerminal::with_encoding_and_color_mode(OutputEncoding::Utf8, color_mode);

    // Enable alternate screen + raw mode.
    use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
    use crossterm::execute;
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen)?;

    let result = run_dashboard(game_dir, game_data, geometry, player_record_index_1_based, &mut terminal);

    // Restore terminal.
    use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);

    result
}

fn run_dashboard(
    game_dir: std::path::PathBuf,
    game_data: nc_data::CoreGameData,
    geometry: ScreenGeometry,
    player_record_index_1_based: usize,
    terminal: &mut dyn nc_ui::Terminal,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = DashApp::new(game_dir, game_data, geometry, player_record_index_1_based);
    app.run(terminal)
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
