use crate::config::{host_config::HostConfig, host_identity::HostIdentity, relay::RelayConfig};
use crate::lobby::publish::EventPublisher;
use crate::support::ids::new_outbox_id;
use nc_nostr::lobby_notice::{LobbyNotice, build_lobby_notice_tags};
use nostr_sdk::Keys;
use std::path::PathBuf;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut subcmd = None;
    let mut message = None;
    let mut handle = Some("nc-host".to_string());
    let mut config_path = None;
    let mut identity_path = None;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--message" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --message".into());
                }
                message = Some(args[i + 1].to_string());
                i += 2;
            }
            "--handle" => {
                if i + 1 >= args.len() {
                    return Err("missing value for --handle".into());
                }
                handle = Some(args[i + 1].to_string());
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
            _ => {
                if subcmd.is_none() {
                    subcmd = Some(args[i]);
                } else {
                    return Err(format!("unexpected argument: {}", args[i]).into());
                }
                i += 1;
            }
        }
    }

    match subcmd {
        Some("post") => run_post(
            message.as_deref(),
            handle.as_deref(),
            config_path.as_ref(),
            identity_path.as_ref(),
        ),
        Some(cmd) => Err(format!("unknown subcommand: {}", cmd).into()),
        None => Err("missing subcommand".into()),
    }
}

fn run_post(
    message: Option<&str>,
    handle: Option<&str>,
    config_path: Option<&PathBuf>,
    identity_path: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let message = message
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .ok_or("missing --message argument")?;
    let config = load_config(config_path)?;
    let identity_path = identity_path
        .cloned()
        .unwrap_or_else(|| config.identity_path.clone());
    let identity = HostIdentity::load(&identity_path)?;
    let relay_config = RelayConfig::validate(&config.relay_url)?;
    let keys = Keys::parse(&identity.nsec)?;
    let payload = LobbyNotice {
        notice_id: new_outbox_id("notice", "host"),
        sender_npub: identity.npub.clone(),
        sender_handle: handle
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty()),
        body: message.to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    let content = serde_json::to_string(&payload)?;
    let tags = build_lobby_notice_tags(&payload)
        .into_iter()
        .map(|(key, value)| vec![key.to_string(), value])
        .collect::<Vec<_>>();

    let runtime = tokio::runtime::Runtime::new()?;
    runtime
        .block_on(async move {
            let client = nostr_sdk::Client::new(keys.clone());
            client
                .add_relay(&relay_config.url)
                .await
                .map_err(|err| err.to_string())?;
            client.connect().await;
            let publisher = EventPublisher::new(client, keys);
            publisher
                .publish_multi(30516, &content, tags)
                .await
                .map_err(|err| err.to_string())
        })
        .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
    println!("Posted lobby notice {}", payload.notice_id);
    Ok(())
}

fn load_config(path: Option<&PathBuf>) -> Result<HostConfig, Box<dyn std::error::Error>> {
    if let Some(path) = path {
        return Ok(HostConfig::load(path)?);
    }
    if let Ok(path) = std::env::var("NC_HOST_CONFIG") {
        return Ok(HostConfig::load(&PathBuf::from(path))?);
    }
    Ok(HostConfig::load(&HostConfig::default_config_path())?)
}

fn print_usage() {
    println!("Usage:");
    println!(
        "  nc-host notices post --message \"...\" [--handle <name>] [--config <path>] [--identity <path>]"
    );
}
