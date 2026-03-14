pub mod stdout;

use crossterm::event::KeyEvent;

pub trait Terminal {
    fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn write_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn set_cursor(&mut self, column: u16, row: u16) -> Result<(), Box<dyn std::error::Error>>;
    fn clear_cursor(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>>;
}
