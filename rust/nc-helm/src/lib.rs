mod app;
mod input;
mod runtime;
mod startup;
mod storage;
mod transport;

pub use app::{App, Effect, GameRow, Msg, Route};
pub use input::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
pub use startup::{
    LaunchCommand, LaunchTarget, NativeBackendPreference, NativeLaunchOptions, NativeWindowMode,
    parse_launch_command,
};
pub use storage::{BootSnapshot, StoredSession};
pub use transport::LobbySnapshot;

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    match parse_launch_command(args)? {
        LaunchCommand::Help => {
            startup::print_usage();
            Ok(())
        }
        LaunchCommand::Launch(LaunchTarget::Lobby(options)) => runtime::run(options),
    }
}

pub fn main_entry() -> Result<(), Box<dyn std::error::Error>> {
    run(std::env::args())
}
