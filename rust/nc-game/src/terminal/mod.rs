pub mod cp437;
pub mod door;
pub(crate) mod door_transport;
pub mod stdout;

// Terminal trait and enums now live in nc-ui. Re-export for backward compatibility
// within nc-game and for external consumers that import via nc-game's lib.rs.
pub use nc_ui::terminal::{ColorMode, OutputEncoding, Terminal};
pub use nc_ui::terminal::stdout::StdoutTerminal;
