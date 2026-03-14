pub mod stdout;

use crossterm::event::KeyEvent;

pub trait Terminal {
    fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn write_line(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>>;
}
