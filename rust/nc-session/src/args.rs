//! Shared CLI argument parsing infrastructure for nc-game and nc-dash.

use std::path::PathBuf;

use nc_ui::terminal::{ColorMode, OutputEncoding};

/// Common launch arguments shared between nc-game and nc-dash.
///
/// Frontends add their own additional fields (e.g. ScreenGeometry for
/// nc-game, dashboard-specific options for nc-dash) on top of these.
#[derive(Debug, Clone)]
pub struct CommonLaunchArgs {
    pub game_dir: PathBuf,
    pub explicit_player_record_index_1_based: Option<usize>,
    pub export_root: Option<PathBuf>,
    pub queue_dir: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub log_level: nc_log::LogLevel,
    pub encoding: OutputEncoding,
    pub color_mode: ColorMode,
    pub session_timeout_secs: Option<u32>,
    pub session_token: Option<String>,
    pub hosted_invite_code: Option<String>,
}

/// Detect the terminal's color depth from environment variables.
///
/// Checks COLORTERM and TERM in order. Defaults to Ansi16 for safety.
pub fn detect_color_mode() -> ColorMode {
    if let Ok(colorterm) = std::env::var("COLORTERM") {
        if colorterm == "truecolor" || colorterm == "24bit" {
            return ColorMode::TrueColor;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("256color") {
            return ColorMode::Color256;
        }
        if term.contains("color") || term == "xterm" || term == "screen" {
            return ColorMode::Ansi16;
        }
    }
    ColorMode::Ansi16
}
