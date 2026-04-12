use crate::config::{daemon_config, identity, relay};
use nostr_sdk::{Client, Filter, Kind, ToBech32, Keys};
use std::path::PathBuf;
use std::sync::Arc;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut games_root = None;
    let mut config_path = None;
    let mut identity_path = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--root" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --root".into());
                }
                games_root = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--config" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --config".into());
                }
                config_path = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            "--identity" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --identity".into());
                }
                identity_path = Some(PathBuf::from(args[i + 1]));
                i += 2;
            }
            _ => return Err(format!("unknown argument: {}", args[i]).into()),
        }
    }

    let games_root = games_root.ok_or("missing --root argument")?;

    let config = if let Some(path) = config_path {
        daemon_config::DaemonConfig::load(&path)?
    } else if let Ok(default_path) = std::env::var("NC_DAEMON_CONFIG") {
        daemon_config::DaemonConfig::load(&PathBuf::from(default_path))?
    } else {
        let default = daemon_config::DaemonConfig {
            games_root: games_root.clone(),
            relay_url: "wss://relay.example.com".to_string(),
            identity_path: PathBuf::from("/etc/nc-daemon/daemon.nsec"),
            sysop_contact_npub: String::new(),
        };
        default
    };

    let identity_path = identity_path.unwrap_or_else(|| config.identity_path.clone());
    let identity = match identity::DaemonIdentity::load(&identity_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Warning: failed to load identity: {}", e);
            eprintln!(
                "Run 'nc-daemon nostr init --path {}' first",
                identity_path.display()
            );
            return Err("identity not configured".into());
        }
    };
    let relay_config = relay::RelayConfig::validate(&config.relay_url)?;

    println!("Starting nc-daemon...");
    println!("  games root: {}", config.games_root.display());
    println!("  relay: {}", relay_config.url);
    println!("  identity: {}", identity.npub);

    run_server(&config, &identity, &relay_config)
}

fn run_server(
    config: &daemon_config::DaemonConfig,
    identity: &identity::DaemonIdentity,
    relay_config: &relay::RelayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Daemon server starting");
    tracing::info!("Games root: {}", config.games_root.display());
    tracing::info!("Relay: {}", relay_config.url);
    tracing::info!("Identity: {}", identity.npub);

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        run_async_server(config, identity, relay_config).await
    })
}

async fn run_async_server(
    config: &daemon_config::DaemonConfig,
    identity: &identity::DaemonIdentity,
    relay_config: &relay::RelayConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let keys = Keys::parse(&identity.nsec)?;
    let public_key = keys.public_key();
    let npub = public_key.to_bech32()?;

    tracing::info!("Public key: {}", npub);

    let client = Client::builder()
        .build();

    client.add_relay(&relay_config.url).await?;

    tracing::info!("Connecting to relay: {}", relay_config.url);
    client.connect().await;

    let _games_root = Arc::new(config.games_root.clone());

    let filter = Filter::new()
        .kind(Kind::Custom(30507))
        .kind(Kind::Custom(30513))
        .kind(Kind::Custom(30522))
        .pubkey(public_key);

    let _ = client.subscribe(filter, None).await;

    tracing::info!("Subscribed to kinds 30507, 30513, 30522");
    tracing::info!("Event loop started. Press Ctrl+C to stop.");

    let mut counter = 0u32;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Shutting down...");
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                counter += 1;
                tracing::debug!("Tick {} - waiting for events", counter);
            }
        }
    }

    tracing::info!("Daemon stopped");

    Ok(())
}

fn print_usage() {
    println!("Usage: nc-daemon serve --root <games-root> [--config <path>]");
    println!();
    println!("Options:");
    println!("  --root <path>     Games root directory (required)");
    println!("  --config <path>  Config file path (default: /etc/nc-daemon/daemon.kdl)");
    println!("  --identity <path> Identity nsec file (default: from config)");
}
