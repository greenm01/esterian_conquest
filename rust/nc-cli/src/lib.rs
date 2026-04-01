pub mod commands;
pub mod dispatch;
mod setup_preset;
pub mod support;
pub mod usage;
pub mod workspace;

pub fn run_dev_cli(args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    dispatch::run_args(args)
}

pub fn run_sysop_cli(
    program: &str,
    args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    commands::sysop::run_sysop_args(program, args)
}

pub fn run_maintenance_cli(
    program: &str,
    args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    commands::maint::run_rust_maintenance_from_args(program, args)
}
