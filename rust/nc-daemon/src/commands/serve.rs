use crate::config::{daemon_config, identity, relay};
use std::path::PathBuf;

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
    let _relay = relay::RelayConfig::validate(&config.relay_url)?;

    println!("Starting nc-daemon...");
    println!("  games root: {}", config.games_root.display());
    println!("  relay: {}", config.relay_url);
    println!("  identity: {}", identity.npub);

    run_server(&config, &identity)
}

fn run_server(
    config: &daemon_config::DaemonConfig,
    identity: &identity::DaemonIdentity,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Daemon server starting");
    tracing::info!("Games root: {}", config.games_root.display());
    tracing::info!("Relay: {}", config.relay_url);
    tracing::info!("Identity: {}", identity.npub);

    Ok(())
}

fn print_usage() {
    println!("Usage: nc-daemon serve --root <games-root> [--config <path>]");
    println!();
    println!("Options:");
    println!("  --root <path>     Games root directory (required)");
    println!("  --config <path>  Config file path (default: /etc/nc-daemon/daemon.kdl)");
}
