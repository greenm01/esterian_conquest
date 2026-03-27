mod usage;

use std::env;
use std::path::PathBuf;

struct ParsedArgs {
    log_file: Option<PathBuf>,
    log_level: ec_log::LogLevel,
    args: Vec<String>,
}

fn main() {
    if let Err(err) = run() {
        tracing::error!(error = %err, "ec-sysop command failed");
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = parse_args(env::args().skip(1))?;
    if let Some(log_file) = &parsed.log_file {
        ec_log::init_file_logging(log_file, parsed.log_level)?;
        tracing::info!(
            log_file = %log_file.display(),
            level = ?parsed.log_level,
            "ec-sysop logging initialized"
        );
    }
    let mut args = parsed.args.into_iter();
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
            tracing::info!(target_dir = %rest[0], "running ec-sysop new-game");
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
            tracing::info!(dir = %rest[0], "running ec-sysop maint");
            ec_cli::run_maintenance_cli("ec-sysop maint", rest.into_iter())
        }
        _ => {
            usage::print_usage();
            Err(format!("unknown subcommand: {cmd}").into())
        }
    }
}

fn parse_args(
    args: impl Iterator<Item = String>,
) -> Result<ParsedArgs, Box<dyn std::error::Error>> {
    let mut rest = Vec::new();
    let mut log_file = None;
    let mut log_level = ec_log::LogLevel::Info;
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--log-file" => {
                let Some(value) = args.next() else {
                    return Err("missing value for --log-file".into());
                };
                log_file = Some(PathBuf::from(value));
            }
            "--log-level" => {
                let Some(value) = args.next() else {
                    return Err(
                        "missing value for --log-level (error, warn, info, debug, or trace)".into(),
                    );
                };
                log_level = ec_log::LogLevel::parse(&value)?;
            }
            other => {
                rest.push(other.to_string());
                rest.extend(args);
                break;
            }
        }
    }
    Ok(ParsedArgs {
        log_file,
        log_level,
        args: rest,
    })
}
