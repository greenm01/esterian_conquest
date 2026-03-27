pub mod cp437;
pub mod door;
pub mod stdout;

use crossterm::event::KeyEvent;

use crate::screen::PlayfieldBuffer;

/// Wire encoding for terminal output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputEncoding {
    /// UTF-8 (default).  Modern terminals, SyncTERM in UTF-8 mode.
    #[default]
    Utf8,
    /// CP437 single-byte.  Classic BBS doors, SyncTERM in CP437 mode.
    Cp437,
}

/// Color depth supported by the target terminal.
///
/// Controls how [`GameColor::Indexed`] and [`GameColor::Rgb`] values are emitted.
/// Named 16-color variants are always emitted as-is.
///
/// [`GameColor::Indexed`]: crate::screen::GameColor::Indexed
/// [`GameColor::Rgb`]: crate::screen::GameColor::Rgb
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    /// Classic 16-color ANSI — safe for BBS doors, legacy telnet clients, and SyncTERM.
    /// Extended colors are downgraded to the nearest named 16-color equivalent.
    Ansi16,
    /// 256-color xterm palette — supported by most modern terminals over SSH.
    /// RGB values are downgraded to the nearest xterm-256 index.
    Color256,
    /// 24-bit RGB truecolor — supported by most local terminals and many SSH clients.
    /// All color tiers are emitted at full fidelity.
    #[default]
    TrueColor,
}

pub trait Terminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>>;
    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>>;
    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
