use std::fs;
use std::path::{Path, PathBuf};

use nostr_sdk::{Keys, PublicKey, ToBech32};

use crate::cache::io::{cache_path, save_cache_to};
use crate::cache::{CachedGame, CachedGameStatus, GameCache};
use crate::config::io::config_path;
use crate::config::{ConnectConfig, save_config_to};
use crate::keychain::io::{format_iso8601, keychain_path, save_keychain_to};
use crate::keychain::{Identity, IdentityType, Keychain, identity_npub};

const DEFAULT_IDENTITIES: usize = 32;
const DEFAULT_GAMES: usize = 64;
const DEFAULT_PASSWORD: &str = "testing";

#[derive(Debug, Clone)]
pub struct SeedUiOptions {
    pub identities: usize,
    pub games: usize,
    pub password: String,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct SeedUiSummary {
    pub keychain_path: PathBuf,
    pub cache_path: PathBuf,
    pub identities: usize,
    pub games: usize,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct SeedLocalhostFixtureOptions {
    pub nsec: String,
    pub password: String,
    pub relay_url: String,
    pub game_id: String,
    pub game_name: String,
    pub player_name: Option<String>,
    pub server: String,
    pub port: u16,
    pub seat: u32,
    pub gate_npub: String,
    pub joined: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct SeedLocalhostFixtureSummary {
    pub keychain_path: PathBuf,
    pub cache_path: PathBuf,
    pub config_path: PathBuf,
    pub player_npub: String,
    pub gate_npub: String,
    pub password: String,
}

impl Default for SeedUiOptions {
    fn default() -> Self {
        Self {
            identities: DEFAULT_IDENTITIES,
            games: DEFAULT_GAMES,
            password: DEFAULT_PASSWORD.to_string(),
            force: false,
        }
    }
}

pub fn seed_ui(options: &SeedUiOptions) -> Result<SeedUiSummary, Box<dyn std::error::Error>> {
    let keychain_path = keychain_path();
    let cache_path = cache_path();
    seed_ui_to_paths(options, &keychain_path, &cache_path)
}

pub fn seed_ui_to_paths(
    options: &SeedUiOptions,
    keychain_out: &Path,
    cache_out: &Path,
) -> Result<SeedUiSummary, Box<dyn std::error::Error>> {
    if options.identities == 0 {
        return Err("seed-ui requires at least one identity".into());
    }
    if !options.force {
        refuse_existing(keychain_out, "keychain")?;
        refuse_existing(cache_out, "cache")?;
    }

    let keychain = build_keychain(options.identities)?;
    let cache = build_cache(options.games, &keychain)?;

    save_keychain_to(&keychain, &options.password, keychain_out)?;
    save_cache_to(&cache, cache_out)?;

    Ok(SeedUiSummary {
        keychain_path: keychain_out.to_path_buf(),
        cache_path: cache_out.to_path_buf(),
        identities: keychain.identities.len(),
        games: cache.games.len(),
        password: options.password.clone(),
    })
}

pub fn seed_localhost_fixture(
    options: &SeedLocalhostFixtureOptions,
) -> Result<SeedLocalhostFixtureSummary, Box<dyn std::error::Error>> {
    seed_localhost_fixture_to_paths(options, &keychain_path(), &cache_path(), &config_path())
}

pub fn seed_localhost_fixture_to_paths(
    options: &SeedLocalhostFixtureOptions,
    keychain_out: &Path,
    cache_out: &Path,
    config_out: &Path,
) -> Result<SeedLocalhostFixtureSummary, Box<dyn std::error::Error>> {
    if !options.force {
        refuse_existing(keychain_out, "keychain")?;
        refuse_existing(cache_out, "cache")?;
        refuse_existing(config_out, "config")?;
    }

    let created = options
        .joined
        .clone()
        .unwrap_or_else(nowish_fixture_timestamp);
    let mut keychain = Keychain::empty();
    keychain.identities.push(Identity {
        nsec: normalize_nsec(&options.nsec)?,
        identity_type: IdentityType::Imported,
        created: created.clone(),
        alias: Some("Localhost Fixture".to_string()),
    });
    let player_npub = identity_npub(
        keychain
            .active_identity()
            .ok_or("keychain missing identity")?,
    )?;
    let gate_npub = normalize_pubkey(&options.gate_npub)?;

    let cache = GameCache {
        games: vec![CachedGame {
            id: options.game_id.clone(),
            name: options.game_name.clone(),
            player_name: options.player_name.clone(),
            server: options.server.clone(),
            port: options.port,
            relay_url: Some(options.relay_url.clone()),
            seat: options.seat,
            npub: player_npub.clone(),
            gate_npub: gate_npub.clone(),
            status: CachedGameStatus::Joined,
            invite_code: None,
            joined: created,
            last_connected: None,
        }],
    };

    let mut config = ConnectConfig::empty();
    config.set_default_relay(&options.relay_url);

    save_keychain_to(&keychain, &options.password, keychain_out)?;
    save_cache_to(&cache, cache_out)?;
    save_config_to(&config, config_out)?;

    Ok(SeedLocalhostFixtureSummary {
        keychain_path: keychain_out.to_path_buf(),
        cache_path: cache_out.to_path_buf(),
        config_path: config_out.to_path_buf(),
        player_npub,
        gate_npub,
        password: options.password.clone(),
    })
}

fn refuse_existing(path: &Path, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    match fs::metadata(path) {
        Ok(_) => Err(format!(
            "{label} already exists at {}; rerun with --force to overwrite test data",
            path.display()
        )
        .into()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn build_keychain(count: usize) -> Result<Keychain, Box<dyn std::error::Error>> {
    let mut identities = Vec::with_capacity(count);
    for index in 0..count {
        let keys = Keys::generate();
        identities.push(Identity {
            nsec: keys.secret_key().to_bech32()?,
            identity_type: if index % 2 == 0 {
                IdentityType::Local
            } else {
                IdentityType::Imported
            },
            created: format_iso8601(1_775_300_000 + (index as u64 * 4_321)),
            alias: fake_alias(index),
        });
    }

    Ok(Keychain {
        active: count.saturating_sub(1).min(2),
        identities,
    })
}

fn normalize_nsec(nsec: &str) -> Result<String, Box<dyn std::error::Error>> {
    let keys = Keys::parse(nsec.trim()).map_err(|err| format!("invalid nsec: {err}"))?;
    Ok(keys.secret_key().to_bech32()?)
}

fn normalize_pubkey(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(PublicKey::parse(input.trim())
        .map_err(|err| format!("invalid pubkey/npub: {err}"))?
        .to_bech32()?)
}

fn nowish_fixture_timestamp() -> String {
    format_iso8601(1_775_950_000)
}

fn build_cache(games: usize, keychain: &Keychain) -> Result<GameCache, Box<dyn std::error::Error>> {
    let mut cache = GameCache::empty();
    let identity_npubs = keychain
        .identities
        .iter()
        .map(crate::keychain::identity_npub)
        .collect::<Result<Vec<_>, _>>()?;

    for index in 0..games {
        let npub = &identity_npubs[index % identity_npubs.len()];
        let gate_keys = Keys::generate();
        cache.games.push(CachedGame {
            id: format!("stress-{:03}", index + 1),
            name: fake_game_name(index),
            player_name: fake_empire_name(index),
            server: fake_server(index),
            port: if index % 5 == 0 { 2222 } else { 22 },
            relay_url: Some(fake_relay_url(index)),
            seat: ((index % 25) + 1) as u32,
            npub: npub.clone(),
            gate_npub: gate_keys.public_key().to_bech32()?,
            status: CachedGameStatus::Joined,
            invite_code: None,
            joined: format_iso8601(1_775_400_000 + (index as u64 * 777)),
            last_connected: if index % 6 == 0 {
                None
            } else {
                Some(format_iso8601(1_775_900_000 + (index as u64 * 555)))
            },
        });
    }

    Ok(cache)
}

fn fake_alias(index: usize) -> Option<String> {
    let values = [
        Some("Primary Desk".to_string()),
        Some("Night Watch".to_string()),
        None,
        Some("BBS Archive Key".to_string()),
        Some("Tournament Spare".to_string()),
        None,
        Some("Long Alias For Border Truncation".to_string()),
        Some("Blue Cell".to_string()),
        None,
        Some("Rust Marshal".to_string()),
    ];
    values[index % values.len()].clone()
}

fn fake_empire_name(index: usize) -> Option<String> {
    let values = [
        Some("House Vale".to_string()),
        Some("Crown Meridian".to_string()),
        Some("The Irons".to_string()),
        Some("Archive Dominion".to_string()),
        Some("Nocturne Reach".to_string()),
        Some("Lattice Court".to_string()),
        None,
        Some("Velvet Armada".to_string()),
    ];
    values[index % values.len()].clone()
}

fn fake_game_name(index: usize) -> String {
    let adjectives = [
        "Velvet", "Amber", "Iron", "Silver", "Obsidian", "Ivory", "Crimson", "Echo",
    ];
    let nouns = [
        "Frontier", "Conclave", "Warpath", "Ledger", "Citadel", "Mandate", "Crossing", "Signal",
    ];
    format!(
        "{} {} {}",
        adjectives[index % adjectives.len()],
        nouns[(index / adjectives.len()) % nouns.len()],
        index + 1
    )
}

fn fake_server(index: usize) -> String {
    let hosts = [
        "play.example.com",
        "war-room.example.net",
        "bbs-gate.internal",
        "nostr-hub.example.org",
        "frontier-lane.example.com",
    ];
    let host = hosts[index % hosts.len()];
    if index % 4 == 0 {
        format!("stress-{:02}.{}", (index % 17) + 1, host)
    } else {
        host.to_string()
    }
}

fn fake_relay_url(index: usize) -> String {
    if index % 3 == 0 {
        "ws://localhost:8080".to_string()
    } else {
        format!("wss://relay{}.example.com", (index % 7) + 1)
    }
}
