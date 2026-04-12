pub mod commands;
pub mod config;
pub mod dispatch;
pub mod game;
pub mod invite;
pub mod lobby;
pub mod support;
pub mod supervisor;

pub fn run_cli(args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    dispatch::run_args(args)
}
