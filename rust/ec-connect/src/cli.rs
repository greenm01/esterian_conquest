use std::env;
use std::path::PathBuf;

use nostr_sdk::Keys;

use crate::config::{ConnectConfig, load_config};
use crate::connect::resolve::{resolve_invite, resolve_server};
use crate::connect::session::{DisambigMode, SessionOutcome, resolve_gate_npub, run_session};
use crate::identity::{
    cmd_id_import, cmd_id_list, cmd_id_new, cmd_id_secret, cmd_id_show, cmd_id_switch,
};
use crate::map_store::resolve_maps_root;
use crate::picker::run_picker;
use crate::wallet::io::{load_wallet_from, now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{Identity, IdentityType, Wallet};

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
            Some(other) => Err(format!("unknown id subcommand: {other}").into()),
        };
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
    let gate_npub = opts
        .gate_npub
        .ok_or("--join requires --gate <npub> (the server's Nostr public key)")?;

    // Load config (optional).
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let maps_root = resolve_maps_root(config.maps_dir.as_deref(), opts.maps_dir.as_deref());

    // Load wallet — auto-create identity if wallet is missing.
    let password = prompt_password("Password: ")?;
    let (wallet, keys, npub) = load_or_create_identity(&password)?;
    drop(wallet);

    // Resolve invite code.
    let mut target =
        resolve_invite(code, &config).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    // Apply relay override.
    if let Some(relay) = opts.relay_override {
        target.relay_url = relay;
    }

    eprintln!("Joining game...");

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

// ── Direct mode ───────────────────────────────────────────────────────────────

fn cmd_direct(server: &str, opts: ConnectOpts) -> Result<(), Box<dyn std::error::Error>> {
    // Load config (optional).
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let maps_root = resolve_maps_root(config.maps_dir.as_deref(), opts.maps_dir.as_deref());

    // Load wallet — must exist for direct mode.
    let password = prompt_password("Password: ")?;
    let (_wallet, keys, npub) = load_existing_identity(&password)?;

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

    // Load wallet — auto-create identity if wallet is missing.
    let password = prompt_password("Password: ")?;
    let (wallet, keys, npub) = load_or_create_identity(&password)?;

    let identity_count = wallet.identities.len();
    let identity_type = wallet
        .active_identity()
        .map(|id| id.identity_type.as_str().to_string())
        .unwrap_or_else(|| "local".to_string());
    let config = load_config().unwrap_or_else(|_| ConnectConfig::empty());
    let maps_root = resolve_maps_root(config.maps_dir.as_deref(), opts.maps_dir.as_deref());

    run_picker(
        keys,
        npub,
        gate_npub,
        identity_count,
        identity_type,
        maps_root,
    )
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Prompt for a password without echoing.
fn prompt_password(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    rpassword::prompt_password(prompt).map_err(|e| e.into())
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
        // Auto-create an identity.
        let confirm = prompt_password("Confirm password: ")?;
        if confirm != password {
            return Err("passwords do not match".into());
        }
        let new_keys = Keys::generate();
        let nsec = new_keys.secret_key().to_bech32()?;
        let npub = new_keys.public_key().to_bech32()?;
        wallet.identities.push(Identity {
            nsec,
            identity_type: IdentityType::Local,
            created: now_iso8601(),
        });
        save_wallet_to(&wallet, password, &path)?;
        eprintln!("Identity created: {npub}");
    }

    let id = wallet
        .active_identity()
        .ok_or("wallet has no active identity")?;
    let keys = Keys::parse(&id.nsec)?;
    let npub = keys.public_key().to_bech32()?;
    Ok((wallet, keys, npub))
}

/// Load wallet + active identity; error if wallet is absent.
fn load_existing_identity(
    password: &str,
) -> Result<(Wallet, Keys, String), Box<dyn std::error::Error>> {
    use nostr_sdk::ToBech32;

    let path = wallet_path();
    let wallet = load_wallet_from(password, &path)?
        .ok_or("no wallet found; run `ec-connect id new` first, or use --join to create one")?;

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

/// Print the `SessionOutcome` and convert to a Result.
fn report_outcome(outcome: SessionOutcome) -> Result<(), Box<dyn std::error::Error>> {
    match outcome {
        SessionOutcome::Done {
            exit_code: 0,
            notice,
        } => {
            if let Some(msg) = notice {
                eprintln!("{msg}");
            }
            Ok(())
        }
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

// ── Usage ─────────────────────────────────────────────────────────────────────

fn print_usage() {
    println!(
        "\
ec-connect — Esterian Conquest multiplayer client

Usage:
  ec-connect                                       Picker mode (game list)
  ec-connect <SERVER> --gate <NPUB>                Direct mode (connect to server)
  ec-connect --join <INVITE-CODE> --gate <NPUB>    Join a new game

Identity:
  ec-connect id                        Show active identity (npub)
  ec-connect id --secret               Show npub + nsec (for backup)
  ec-connect id list                   List all wallet identities
  ec-connect id new                    Generate a new keypair
  ec-connect id import                 Import an existing nsec
  ec-connect id switch <N>             Switch active identity

Options:
  --gate <NPUB>    Gate server Nostr public key (required for connect/join)
  --relay <URL>    Override Nostr relay URL
  --maps-dir <PATH> Override where downloaded starmap bundles are stored
  --version        Print version
  --help           Print this help"
    );
}
