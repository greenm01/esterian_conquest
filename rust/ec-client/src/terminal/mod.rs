pub mod stdout;

use crossterm::event::KeyEvent;

use crate::screen::PlayfieldBuffer;

pub trait Terminal {
    fn render(&mut self, playfield: &PlayfieldBuffer) -> Result<(), Box<dyn std::error::Error>>;
    fn dump_text_capture(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>>;
    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
