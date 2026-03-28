mod nostr;
mod usage;

use std::env;
use std::path::PathBuf;

#[derive(Clone)]
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
    let mut args = parsed.args.clone().into_iter();
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
            init_logging(&parsed, false)?;
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_new_game_usage();
                return Ok(());
            }
            if rest.is_empty() {
                usage::print_new_game_usage();
                return Err("missing target_dir for new-game".into());
            }
            tracing::info!(target_dir = %rest[0], "running ec-sysop new-game");
            let target_dir = resolve_repo_path(&rest[0]);
            ec_cli::run_sysop_cli("ec-sysop", std::iter::once(cmd).chain(rest))?;
            nostr::initialize_hosted_seats_for_new_game(&target_dir)
        }
        "maint" => {
            init_logging(&parsed, false)?;
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
        "nostr" => run_nostr(&parsed, rest),
        _ => {
            usage::print_usage();
            Err(format!("unknown subcommand: {cmd}").into())
        }
    }
}

fn resolve_repo_path(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else if path.exists() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }
}

fn init_logging(
    parsed: &ParsedArgs,
    default_to_stderr: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(log_file) = &parsed.log_file {
        ec_log::init_file_logging(log_file, parsed.log_level)?;
        tracing::info!(
            log_file = %log_file.display(),
            level = ?parsed.log_level,
            "ec-sysop logging initialized"
        );
    } else if default_to_stderr {
        ec_log::init_stderr_logging(parsed.log_level)?;
        tracing::info!(level = ?parsed.log_level, "ec-sysop stderr logging initialized");
    }
    Ok(())
}

fn run_nostr(parsed: &ParsedArgs, rest: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut args = rest.into_iter();
    let Some(cmd) = args.next() else {
        usage::print_nostr_usage();
        return Ok(());
    };
    let rest = args.collect::<Vec<_>>();

    match cmd.as_str() {
        "--help" | "-h" | "help" => {
            usage::print_nostr_usage();
            Ok(())
        }
        "init" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_init_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let identity_path = parse_single_path_flag(&rest, "--identity")?;
            tracing::info!(
                identity_path = ?identity_path.as_ref().map(|path| path.display().to_string()),
                "running ec-sysop nostr init"
            );
            let initialized = ec_gate::init_identity_at(identity_path)?;
            if initialized.already_exists {
                println!(
                    "Daemon identity already exists at: {}",
                    initialized.path.display()
                );
                println!("Public key (npub): {}", initialized.npub);
                println!("Created: {}", initialized.created);
            } else {
                println!("Daemon identity created at: {}", initialized.path.display());
                println!("Public key (npub): {}", initialized.npub);
            }
            Ok(())
        }
        "serve" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_serve_usage();
                return Ok(());
            }
            init_logging(parsed, true)?;
            let parsed_flags = parse_path_flags(&rest, &["--config", "--identity"])?;
            let config_path = parsed_flags.get("--config").cloned().flatten();
            let identity_path = parsed_flags.get("--identity").cloned().flatten();
            tracing::info!(
                config_path = ?config_path.as_ref().map(|path| path.display().to_string()),
                identity_path = ?identity_path.as_ref().map(|path| path.display().to_string()),
                "running ec-sysop nostr serve"
            );
            ec_gate::serve_from_paths(config_path, identity_path)
        }
        "migrate-roster" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_migrate_roster_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let dir = nostr::parse_required_dir_flag(&rest)?;
            tracing::info!(dir = %dir.display(), "running ec-sysop nostr migrate-roster");
            println!("{}", nostr::migrate_roster(&dir)?);
            Ok(())
        }
        "seats" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_seats_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let dir = nostr::parse_required_dir_flag(&rest)?;
            tracing::info!(dir = %dir.display(), "running ec-sysop nostr seats");
            print!("{}", nostr::render_hosted_seats(&dir)?);
            Ok(())
        }
        "reissue" => {
            if rest.iter().any(|arg| arg == "--help" || arg == "-h") {
                usage::print_nostr_reissue_usage();
                return Ok(());
            }
            init_logging(parsed, false)?;
            let (dir, player) = nostr::parse_dir_and_player_flags(&rest)?;
            tracing::info!(dir = %dir.display(), player, "running ec-sysop nostr reissue");
            println!("{}", nostr::reissue_hosted_seat(&dir, player)?);
            Ok(())
        }
        _ => {
            usage::print_nostr_usage();
            Err(format!("unknown nostr subcommand: {cmd}").into())
        }
    }
}

fn parse_single_path_flag(
    args: &[String],
    flag: &str,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let mut value = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == flag {
            i += 1;
            let Some(next) = args.get(i) else {
                return Err(format!("missing value for {flag}").into());
            };
            value = Some(PathBuf::from(next));
        } else if let Some(next) = arg.strip_prefix(&format!("{flag}=")) {
            value = Some(PathBuf::from(next));
        } else {
            return Err(format!("unexpected argument: {arg}").into());
        }
        i += 1;
    }
    Ok(value)
}

fn parse_path_flags(
    args: &[String],
    allowed_flags: &[&str],
) -> Result<std::collections::BTreeMap<String, Option<PathBuf>>, Box<dyn std::error::Error>> {
    let allowed = allowed_flags
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut values = allowed_flags
        .iter()
        .map(|flag| ((*flag).to_string(), None))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some((flag, value)) = allowed.iter().find_map(|flag| {
            arg.strip_prefix(&format!("{flag}="))
                .map(|value| (*flag, value))
        }) {
            values.insert(flag.to_string(), Some(PathBuf::from(value)));
            i += 1;
            continue;
        }
        if allowed.contains(arg.as_str()) {
            i += 1;
            let Some(next) = args.get(i) else {
                return Err(format!("missing value for {arg}").into());
            };
            values.insert(arg.clone(), Some(PathBuf::from(next)));
            i += 1;
            continue;
        }
        return Err(format!("unexpected argument: {arg}").into());
    }
    Ok(values)
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
