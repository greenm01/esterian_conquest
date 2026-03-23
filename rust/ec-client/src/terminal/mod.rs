pub mod cp437;
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

pub trait Terminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>>;
    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>>;
    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
