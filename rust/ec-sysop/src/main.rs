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

    match cmd.as_str() {
        "--help" | "-h" => {
            usage::print_usage();
            Ok(())
        }
        "maint" => ec_cli::run_maintenance_cli("ec-sysop maint", args),
        _ => ec_cli::run_sysop_cli("ec-sysop", std::iter::once(cmd).chain(args)),
    }
}
