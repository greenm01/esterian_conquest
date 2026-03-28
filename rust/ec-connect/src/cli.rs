use std::env;
use std::path::PathBuf;

use nostr_sdk::Keys;

use crate::config::{load_config, ConnectConfig};
use crate::connect::game_discovery::discover_game_for_invite;
use crate::connect::public_join::run_public_join;
use crate::connect::resolve::{resolve_invite, resolve_server};
use crate::connect::session::{resolve_gate_npub, run_session, DisambigMode, SessionOutcome};
#[cfg(debug_assertions)]
use crate::dev_seed::{seed_ui, SeedUiOptions};
use crate::identity::{
    cmd_id_import, cmd_id_list, cmd_id_new, cmd_id_reset, cmd_id_secret, cmd_id_show, cmd_id_switch,
};
use crate::launcher::{run_password_gate, run_password_gate_in_session};
use crate::map_store::resolve_maps_root;

use crate::picker::{load_picker_session, run_picker_in_session};
use crate::wallet::io::{load_wallet_from, now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{push_new_identity, Wallet};
use ec_ui::session::TerminalSession;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all args upfront so that flags like --gate can appear in any
    // position relative to the positional subcommand.
    let all_args: Vec<String> = env::args().skip(1).collect();

    // Determine the subcommand by scanning for the first non-flag token,
    // while treating `--join` as a named subcommand (not a plain flag).
    // Flags --gate and --relay are extracted into a separate list so they can
    // appear before or after the positional argument.
    //
    // Special cases handled:
    //   ec-connect                       → picker (no positional)
    //   ec-connect --gate <npub>         → picker with explicit gate
    //   ec-connect --join <code> ...     → join flow
    //   ec-connect id [sub] ...          → identity management
    //   ec-connect <server> ...          → direct mode
    //   ec-connect --help / --version    → meta

    // First check for the meta flags that take no value.
    if all_args
        .iter()
        .any(|a| matches!(a.as_str(), "--help" | "-h" | "help"))
    {
        print_usage();
        return Ok(());
    }
    if all_args.iter().any(|a| a == "--version") {
        println!("ec-connect {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Check for `--join` subcommand.
    if let Some(join_pos) = all_args.iter().position(|a| a == "--join") {
        let code = all_args
            .get(join_pos + 1)
            .cloned()
            .ok_or("--join requires an invite code")?;
        // Remaining flags: everything except `--join` and the code.
        let rest: Vec<String> = all_args
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != join_pos && i != join_pos + 1)
            .map(|(_, v)| v.clone())
            .collect();
        let opts = parse_connect_opts(&mut rest.into_iter())?;
        return cmd_join(&code, opts);
    }

    // Check for `id` subcommand.
    if all_args.first().map(|s| s.as_str()) == Some("id") {
        let mut id_args = all_args.into_iter().skip(1);
        let sub = id_args.next();
        return match sub.as_deref() {
            None => cmd_id_show(),
            Some("--secret") => cmd_id_secret(),
            Some("list") => cmd_id_list(),
            Some("new") => cmd_id_new(),
            Some("import") => cmd_id_import(),
            Some("switch") => {
                let n = id_args.next().ok_or("usage: ec-connect id switch <N>")?;
                cmd_id_switch(&n)
            }
            Some("reset") => cmd_id_reset(),
            Some(other) => Err(format!("unknown id subcommand: {other}").into()),
        };
    }

    #[cfg(debug_assertions)]
    if all_args.first().map(|s| s.as_str()) == Some("dev") {
        let mut dev_args = all_args.into_iter().skip(1);
        let sub = dev_args.next();
        return match sub.as_deref() {
            Some("seed-ui") => cmd_dev_seed_ui(dev_args),
            Some(other) => Err(format!("unknown dev subcommand: {other}").into()),
            None => Err(
                "usage: ec-connect dev seed-ui [--games N] [--identities N] [--password PASS] [--force] [--launch]"
                    .into(),
            ),
        };
    }
    #[cfg(not(debug_assertions))]
    if all_args.first().map(|s| s.as_str()) == Some("dev") {
        return Err("ec-connect dev is only available in debug builds".into());
    }

    // Extract --gate and --relay from all_args, leaving the first non-flag
    // token as the optional positional (server or nothing).
    let mut positional: Option<String> = None;
    let mut flag_args: Vec<String> = Vec::new();
    let mut i = 0;
    while i < all_args.len() {
        match all_args[i].as_str() {
            "--gate" | "--relay" => {
                flag_args.push(all_args[i].clone());
                i += 1;
                if i < all_args.len() {
                    flag_args.push(all_args[i].clone());
                }
            }
            arg if arg.starts_with("--") => {
                return Err(format!("unknown option: {arg}").into());
            }
            arg => {
                if positional.is_none() {
                    positional = Some(arg.to_string());
                }
                // Any extra positional tokens are silently ignored;
                // parse_connect_opts will catch unexpected flags.
            }
        }
        i += 1;
    }

    let opts = parse_connect_opts(&mut flag_args.into_iter())?;

    match positional {
        None => cmd_picker(opts),
        Some(server) => cmd_direct(&server, opts),
    }
}

// ── Connect option parsing ────────────────────────────────────────────────────

struct ConnectOpts {
    gate_npub: Option<String>,
    relay_override: Option<String>,
    maps_dir: Option<PathBuf>,
}

fn parse_connect_opts(
    args: &mut impl Iterator<Item = String>,
) -> Result<ConnectOpts, Box<dyn std::error::Error>> {
    let mut gate_npub = None;
    let mut relay_override = None;
    let mut maps_dir = None;

    let remaining: Vec<String> = args.collect();
    let mut i = 0;
    while i < remaining.len() {
        match remaining[i].as_str() {
            "--gate" => {
                i += 1;
                gate_npub = Some(
                    remaining
                        .get(i)
                        .cloned()
                        .ok_or("--gate requires a npub argument")?,
                );
            }
            "--relay" => {
                i += 1;
                relay_override = Some(
                    remaining
                        .get(i)
                        .cloned()
                        .ok_or("--relay requires a URL argument")?,
                );
            }
            "--maps-dir" => {
                i += 1;
                maps_dir = Some(PathBuf::from(
                    remaining
                        .get(i)
                        .cloned()
                        .ok_or("--maps-dir requires a path argument")?,
                ));
            }
            other => return Err(format!("unexpected argument: {other}").into()),
        }
        i += 1;
    }
    drop(remaining);

    Ok(ConnectOpts {
        gate_npub,
        relay_override,
        maps_dir,
    })
}

// ── --join ────────────────────────────────────────────────────────────────────

fn cmd_join(code: &str, opts: ConnectOpts) -> Result<(), Box<dyn std::error::Error>> {
    // Load config (optional).
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let maps_root = resolve_maps_root(config.maps_dir.as_deref(), opts.maps_dir.as_deref());

    // Load wallet — auto-create identity if wallet is missing.
    let Some((wallet, keys, npub)) = prompt_and_load_identity()? else {
        return Ok(());
    };
    drop(wallet);

    // Resolve invite code.
    let mut target =
        resolve_invite(code, &config).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    // Apply relay override.
    if let Some(relay) = opts.relay_override {
        target.relay_url = relay;
    }

    eprintln!("Joining game...");
    let outcome = if let Some(gate_npub) = opts.gate_npub {
        let discovered = run_tokio(discover_game_for_invite(&keys, &target, code))
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        if let Ok(discovered) = discovered {
            target.game_id = Some(discovered.game_id);
        }
        run_tokio(run_session(
            &keys,
            target,
            &npub,
            &gate_npub,
            DisambigMode::Prompt,
            &maps_root,
        ))?
    } else {
        run_tokio(run_public_join(
            &keys,
            target,
            &npub,
            DisambigMode::Prompt,
            &maps_root,
        ))?
        .map_err(|err| -> Box<dyn std::error::Error> { err })?
    };

    report_outcome(outcome)
}

// ── Direct mode ───────────────────────────────────────────────────────────────

fn cmd_direct(server: &str, opts: ConnectOpts) -> Result<(), Box<dyn std::error::Error>> {
    // Load config (optional).
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let maps_root = resolve_maps_root(config.maps_dir.as_deref(), opts.maps_dir.as_deref());

    // Load wallet — auto-create identity if the wallet is missing.
    let Some((_wallet, keys, npub)) = prompt_and_load_identity()? else {
        return Ok(());
    };

    // Resolve server.
    let mut target =
        resolve_server(server, &config).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // Apply relay override.
    if let Some(relay) = opts.relay_override {
        target.relay_url = relay;
    }

    // Inject cached game_id hint if we have exactly one game for this server.
    inject_cached_game_id(&mut target, &npub);

    // Resolve gate npub: explicit --gate takes priority, then cache lookup.
    let cache = crate::cache::load_cache().unwrap_or_else(|_| crate::cache::GameCache::empty());
    let gate_npub = resolve_gate_npub(&target.server_host, &cache, opts.gate_npub.as_deref())
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    eprintln!("Connecting...");

    let outcome = run_tokio(run_session(
        &keys,
        target,
        &npub,
        &gate_npub,
        DisambigMode::Prompt,
        &maps_root,
    ))?;

    report_outcome(outcome)
}

// ── Picker mode ───────────────────────────────────────────────────────────────

fn cmd_picker(opts: ConnectOpts) -> Result<(), Box<dyn std::error::Error>> {
    // --gate is optional for the picker; the user will be prompted if they try
    // to connect and no gate is configured.  For now we pass it through; a
    // missing gate will surface as an error inside the session handshake.
    let gate_npub = opts.gate_npub.unwrap_or_default();

    let mut session = TerminalSession::enter_picker()?;

    // Load wallet — auto-create identity if wallet is missing.
    let Some(password) = prompt_for_picker_password(&mut session)? else {
        let _ = session.restore();
        return Ok(());
    };
    let picker_session = load_picker_session(password)?;
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let maps_root = resolve_maps_root(config.maps_dir.as_deref(), opts.maps_dir.as_deref());

    let result = run_picker_in_session(
        picker_session,
        gate_npub,
        maps_root,
        config.effective_lock_timeout_minutes(),
        session,
    );
    if result.is_ok() {
        emit_picker_exit_lines();
    }
    result
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn prompt_and_load_identity() -> Result<Option<(Wallet, Keys, String)>, Box<dyn std::error::Error>>
{
    let mut error_msg = None;
    loop {
        let Some(password) = run_password_gate(error_msg.take())? else {
            return Ok(None);
        };
        match load_or_create_identity(&password) {
            Ok(identity) => return Ok(Some(identity)),
            Err(err) => {
                error_msg = Some(format!("Error: {err}"));
            }
        }
    }
}

fn prompt_for_picker_password(
    session: &mut TerminalSession,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut error_msg = None;
    loop {
        let Some(password) = run_password_gate_in_session(session, error_msg.take())? else {
            return Ok(None);
        };
        match load_wallet_from(&password, &wallet_path()) {
            Ok(Some(wallet)) if wallet.identities.is_empty() => {
                error_msg = Some("Error: wallet has no active identity".to_string())
            }
            Ok(_) => return Ok(Some(password)),
            Err(err) => error_msg = Some(format!("Error: {err}")),
        }
    }
}

/// Load wallet + active identity, or create a fresh one if no wallet exists.
/// Returns `(wallet, keys, npub_string)`.
fn load_or_create_identity(
    password: &str,
) -> Result<(Wallet, Keys, String), Box<dyn std::error::Error>> {
    use nostr_sdk::ToBech32;

    let path = wallet_path();

    let mut wallet = load_wallet_from(password, &path)?.unwrap_or_else(Wallet::empty);

    if wallet.identities.is_empty() {
        let npub = push_new_identity(&mut wallet, now_iso8601())?;
        save_wallet_to(&wallet, password, &path)?;
        eprintln!("Nostr keypair created: {npub}");
    }

    let id = wallet
        .active_identity()
        .ok_or("wallet has no active identity")?;
    let keys = Keys::parse(&id.nsec)?;
    let npub = keys.public_key().to_bech32()?;
    Ok((wallet, keys, npub))
}

/// If the player has exactly one cached game for the resolved server, inject
/// its game_id as a hint so the handshake can skip disambiguation.
fn inject_cached_game_id(target: &mut crate::connect::resolve::ResolvedTarget, npub: &str) {
    use crate::cache::load_cache;

    let Ok(cache) = load_cache() else { return };
    let matches: Vec<_> = cache
        .games
        .iter()
        .filter(|g| g.server == target.server_host && g.npub == npub)
        .collect();

    if matches.len() == 1 {
        target.game_id = Some(matches[0].id.clone());
    }
}

/// Run an async future on a new single-threaded tokio runtime.
fn run_tokio<F, T>(fut: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: std::future::Future<Output = T>,
{
    let rt = tokio::runtime::Runtime::new()?;
    Ok(rt.block_on(fut))
}

#[doc(hidden)]
pub fn successful_session_handoff_lines(outcome: &SessionOutcome) -> Option<Vec<String>> {
    match outcome {
        SessionOutcome::Done {
            exit_code: 0,
            notice,
        } => {
            let mut lines = Vec::new();
            if let Some(msg) = notice.as_deref().filter(|msg| !msg.trim().is_empty()) {
                lines.push(msg.to_string());
            }
            lines.push("For Griffith and glory.".to_string());
            Some(lines)
        }
        _ => None,
    }
}

#[doc(hidden)]
pub fn picker_exit_lines() -> Vec<String> {
    vec!["For Griffith and glory.".to_string()]
}

fn emit_picker_exit_lines() {
    for line in picker_exit_lines() {
        eprintln!("{line}");
    }
}

/// Print the `SessionOutcome` and convert to a Result.
fn report_outcome(outcome: SessionOutcome) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(lines) = successful_session_handoff_lines(&outcome) {
        for line in lines {
            eprintln!("{line}");
        }
        return Ok(());
    }

    match outcome {
        SessionOutcome::Done { exit_code, notice } => {
            if let Some(msg) = notice {
                eprintln!("{msg}");
            }
            Err(format!("session exited with code {exit_code}").into())
        }
        SessionOutcome::Error(msg) => Err(msg.into()),
        SessionOutcome::Timeout => Err("handshake timed out (no response from server)".into()),
        SessionOutcome::NeedsDisambiguation { .. } => {
            // Should never happen in CLI mode (DisambigMode::Prompt handles this inline).
            Err("unexpected disambiguation required (use picker mode)".into())
        }
    }
}

#[cfg(debug_assertions)]
#[derive(Debug, Clone)]
struct DevSeedCommand {
    options: SeedUiOptions,
    launch: bool,
}

#[cfg(debug_assertions)]
fn cmd_dev_seed_ui(args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let command = parse_seed_ui_opts(args)?;
    let summary = seed_ui(&command.options)?;
    if command.launch {
        let picker_session = load_picker_session(summary.password.clone())?;
        let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
        let maps_root = resolve_maps_root(config.maps_dir.as_deref(), None);
        let session = TerminalSession::enter_picker()?;
        let result = run_picker_in_session(
            picker_session,
            String::new(),
            maps_root,
            config.effective_lock_timeout_minutes(),
            session,
        );
        if result.is_ok() {
            emit_picker_exit_lines();
        }
        return result;
    }
    println!("Seeded ec-connect UI test data.");
    println!("wallet: {}", summary.wallet_path.display());
    println!("cache: {}", summary.cache_path.display());
    println!("identities: {}", summary.identities);
    println!("games: {}", summary.games);
    println!("password: {}", summary.password);
    println!("Run ec-connect normally to open the seeded picker.");
    Ok(())
}

#[cfg(debug_assertions)]
fn parse_seed_ui_opts(
    args: impl Iterator<Item = String>,
) -> Result<DevSeedCommand, Box<dyn std::error::Error>> {
    let mut options = SeedUiOptions::default();
    let mut launch = false;
    let values: Vec<String> = args.collect();
    let mut i = 0;
    while i < values.len() {
        match values[i].as_str() {
            "--games" => {
                i += 1;
                options.games = values
                    .get(i)
                    .ok_or("--games requires a value")?
                    .parse::<usize>()
                    .map_err(|_| "invalid --games value")?;
            }
            "--identities" => {
                i += 1;
                options.identities = values
                    .get(i)
                    .ok_or("--identities requires a value")?
                    .parse::<usize>()
                    .map_err(|_| "invalid --identities value")?;
            }
            "--password" => {
                i += 1;
                options.password = values
                    .get(i)
                    .cloned()
                    .ok_or("--password requires a value")?;
            }
            "--force" => {
                options.force = true;
            }
            "--launch" => {
                launch = true;
            }
            "--help" | "-h" | "help" => {
                return Err(
                    "usage: ec-connect dev seed-ui [--games N] [--identities N] [--password PASS] [--force] [--launch]"
                        .into(),
                );
            }
            other => return Err(format!("unexpected argument: {other}").into()),
        }
        i += 1;
    }
    Ok(DevSeedCommand { options, launch })
}

// ── Usage ─────────────────────────────────────────────────────────────────────

fn print_usage() {
    #[cfg(debug_assertions)]
    let developer = "\nDeveloper:\n  ec-connect dev seed-ui               Seed fake wallet/cache data for UI testing\n  ec-connect dev seed-ui --launch      Seed fake data and open the picker immediately\n";
    #[cfg(not(debug_assertions))]
    let developer = "";
    println!(
        "{}\
ec-connect — Esterian Conquest multiplayer client

Usage:
  ec-connect                                       Picker mode (game list)
  ec-connect <SERVER> --gate <NPUB>                Direct mode (connect to server)
  ec-connect --join <INVITE-CODE>                  Join a new game

Identity:
  ec-connect id                        Show active identity (npub)
  ec-connect id --secret               Show npub + nsec (for backup)
  ec-connect id list                   List all wallet identities
  ec-connect id new                    Generate a new Nostr keypair
  ec-connect id import                 Import an existing Nostr nsec
  ec-connect id switch <N>             Switch active identity
  ec-connect id reset                  Wipe wallet and cache (triple confirmation)

Options:
  --gate <NPUB>    Gate server Nostr public key (optional override / fallback)
  --relay <URL>    Override Nostr relay URL
  --maps-dir <PATH> Override where downloaded starmap bundles are stored
  --version        Print version
  --help           Print this help",
        developer
    );
}
