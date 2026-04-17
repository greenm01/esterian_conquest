//! Shared CLI argument parsing infrastructure for nc-game and nc-dash.

use std::path::PathBuf;

/// Wire encoding for terminal output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputEncoding {
    /// UTF-8 (default). Modern terminals, SyncTERM in UTF-8 mode.
    #[default]
    Utf8,
    /// CP437 single-byte. Classic BBS doors, SyncTERM in CP437 mode.
    Cp437,
}

/// Color depth supported by the target terminal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    /// Classic 16-color ANSI.
    Ansi16,
    /// 256-color xterm palette.
    Color256,
    /// 24-bit RGB truecolor.
    #[default]
    TrueColor,
}

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
