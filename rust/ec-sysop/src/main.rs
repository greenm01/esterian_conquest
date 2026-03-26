mod usage;

use std::env;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        usage::print_usage();
        return Ok(());
    };
    let rest = args.collect::<Vec<_>>();

    match cmd.as_str() {
        "--help" | "-h" | "help" => {
            usage::print_usage();
            Ok(())
        }
        "new-game" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_new_game_usage();
                return Ok(());
            }
            if rest.is_empty() {
                usage::print_new_game_usage();
                return Err("missing target_dir for new-game".into());
            }
            ec_cli::run_sysop_cli("ec-sysop", std::iter::once(cmd).chain(rest))
        }
        "maint" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_maint_usage();
                return Ok(());
            }
            if rest.is_empty() {
                usage::print_maint_usage();
                return Err("missing dir for maint".into());
            }
            ec_cli::run_maintenance_cli("ec-sysop maint", rest.into_iter())
        }
        _ => {
            usage::print_usage();
            Err(format!("unknown subcommand: {cmd}").into())
        }
    }
}
