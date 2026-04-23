mod app;
mod chrome_box;
mod chrome_tags;
mod dashboard;
mod fonts;
mod geometry;
mod grid;
mod input;
mod runtime;
mod startup;
mod storage;
mod theme;
mod transport;

pub use app::{App, Effect, Msg, MyGameRow, OpenGameRow, Route};
pub use grid::{
    AnsiColor, BackgroundMode, Cell, CellStyle, Column, GameColor, PlayfieldBuffer, Point, Row,
    ScreenGeometry, StyledSpan,
};
pub use input::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
pub use startup::{
    LaunchCommand, LaunchTarget, LocalLaunchOptions, NativeBackendPreference, NativeLaunchOptions,
    NativeWindowMode, parse_launch_command,
};
pub use storage::{BootSnapshot, StoredSession};
pub use transport::{LobbySnapshot, SandboxReleaseSuccess};

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    match parse_launch_command(args)? {
        LaunchCommand::Help => {
            startup::print_usage();
            Ok(())
        }
        LaunchCommand::Launch(LaunchTarget::Lobby(options)) => runtime::run(options),
        LaunchCommand::Launch(LaunchTarget::Local(options)) => runtime::run_local(options),
    }
}

pub fn main_entry() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    run(std::env::args())
}
