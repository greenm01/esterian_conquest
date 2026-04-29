use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use nc_client::keychain::{
    Identity, IdentityType, Keychain, active_keys, load_keychain_from, now_iso8601,
    push_new_identity, save_keychain_to,
};
use nostr_sdk::ToBech32;
use ratatui::{Terminal, backend::CrosstermBackend};
use rpassword::read_password;
use std::path::PathBuf;
use std::{
    io,
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

mod app;
mod network;
mod ui;

use crate::app::{
    App, SysopMessage,
    update::{self, UpdateResult},
};
use crate::network::{NetworkEvent, SysopClient};

const SYSOP_APP_DIR_NAME: &str = "nc-nostr-sysop";

fn sysop_data_root() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".local").join("share"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join(SYSOP_APP_DIR_NAME)
}

fn sysop_keychain_path() -> PathBuf {
    sysop_data_root().join("keychain.kdl")
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long)]
    relay: Option<String>,

    #[arg(long)]
    password: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new sysop keychain
    Init,
    /// Import an existing nsec into the keychain
    Import {
        #[arg(long)]
        nsec: String,
        #[arg(long)]
        handle: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Some(Commands::Init) = args.command {
        return run_init(args.password.as_deref());
    }

    if let Some(Commands::Import { nsec, handle }) = args.command {
        return run_import(nsec, handle, args.password.as_deref());
    }

    // Default: Start TUI
    run_tui(args).await
}

fn run_init(password: Option<&str>) -> Result<()> {
    let keychain_path = sysop_keychain_path();
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter a password to encrypt your new keychain:");
            let p1 = read_password().context("failed to read password")?;
            println!("Confirm password:");
            let p2 = read_password().context("failed to read password")?;
            if p1 != p2 {
                return Err(anyhow::anyhow!("passwords do not match"));
            }
            p1
        }
    };

    let mut keychain = Keychain::empty();
    let npub = push_new_identity(&mut keychain, now_iso8601(), Some("sysop".to_string()))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    save_keychain_to(&keychain, &password, &keychain_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("Keychain initialized!");
    println!("Npub: {}", npub);
    println!("Keychain: {}", keychain_path.display());
    Ok(())
}

fn run_import(nsec: String, handle: Option<String>, password: Option<&str>) -> Result<()> {
    let keychain_path = sysop_keychain_path();
    let password = match password {
        Some(p) => p.to_string(),
        None => {
            println!("Enter password to unlock/encrypt keychain:");
            read_password().context("failed to read password")?
        }
    };

    let mut keychain = load_keychain_from(&password, &keychain_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .unwrap_or_else(Keychain::empty);
    keychain.identities.push(Identity {
        nsec,
        identity_type: IdentityType::Imported,
        created: now_iso8601(),
        handle,
    });
    save_keychain_to(&keychain, &password, &keychain_path).map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("Identity imported successfully.");
    println!("Keychain: {}", keychain_path.display());
    Ok(())
}

async fn run_tui(args: Args) -> Result<()> {
    let keychain_path = sysop_keychain_path();
    let password = match args.password {
        Some(p) => p,
        None => {
            println!("Enter keychain password:");
            read_password().context("failed to read password")?
        }
    };

    let keychain = load_keychain_from(&password, &keychain_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .ok_or_else(|| anyhow::anyhow!("keychain not found; run `nc-nostr-sysop init` first"))?;

    let keys = active_keys(&keychain).map_err(|e| anyhow::anyhow!("{e}"))?;
    let relay = args
        .relay
        .unwrap_or_else(|| "ws://127.0.0.1:8080".to_string());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut app = App::new();
    app.sysop_npub = keys
        .public_key()
        .to_bech32()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    app.relay_count = 1; // Basic for now
    app.connection_status = "Connected".to_string();

    let (net_tx, mut net_rx) = mpsc::unbounded_channel();

    // Network client
    let client = SysopClient::new_with_keys(keys).await?;
    client.connect(vec![relay.clone()]).await?;
    client.start_listening(net_tx).await?;

    app.status_line = format!("Connected to {}", relay);

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        tokio::select! {
            // Handle terminal events
            res = tokio::task::spawn_blocking(move || {
                if event::poll(timeout).unwrap_or(false) {
                    Some(event::read().unwrap())
                } else {
                    None
                }
            }) => {
                if let Ok(Some(event)) = res {
                    match event {
                        CrosstermEvent::Key(key) => {
                            match update::handle_input(&mut app, key) {
                                UpdateResult::Quit => break,
                                UpdateResult::MessageSent(content) => {
                                    let channel = app.active_channel().clone();
                                    if let Err(err) = client.send_text(&channel, &content).await {
                                        app.status_line = format!("Send Error: {}", err);
                                    }
                                }

                                _ => {}
                            }
                        }
                        CrosstermEvent::Mouse(mouse) => {
                            update::handle_mouse(&mut app, mouse);
                        }
                        _ => {}
                    }
                }
            }
            // Handle network events
            Some(net_event) = net_rx.recv() => {
                match net_event {
                    NetworkEvent::Connected => {
                        app.status_line = format!("Connected to {}", relay);
                    }
                    NetworkEvent::MessageReceived { sender, content, channel, is_direct } => {
                        if is_direct {
                            if !app.channels.contains(&channel) {
                                app.channels.push(channel.clone());
                            }
                        }
                        app.push_message(SysopMessage {
                            timestamp: Utc::now(),
                            channel,
                            sender,
                            content,
                            is_own: false,
                        });
                    }
                    NetworkEvent::GameDiscovered { id, name } => {
                        if !app.channels.iter().any(|c| matches!(c, crate::app::SysopChannel::Game(gid) if gid == &id)) {
                            app.channels.push(crate::app::SysopChannel::Game(id));
                        }
                        app.status_line = format!("Discovered game: {}", name);
                    }
                    NetworkEvent::Error(err) => {
                        app.status_line = format!("Network Error: {}", err);
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
