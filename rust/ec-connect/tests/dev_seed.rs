use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ec_connect::cache;
use ec_connect::config::load_config_from;
use ec_connect::dev_seed::{
    SeedLocalhostFixtureOptions, SeedUiOptions, seed_localhost_fixture_to_paths, seed_ui_to_paths,
};
use ec_connect::wallet::io::load_wallet_from;
use ec_connect::wallet::{IdentityType, active_identity_npub};
use nostr_sdk::{Keys, ToBech32};

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ec-connect-{name}-{unique}"));
    fs::create_dir_all(&path).expect("temp dir");
    path
}

#[test]
fn seed_ui_writes_wallet_and_cache() {
    let dir = temp_dir("seed-write");
    let wallet_path = dir.join("wallet.kdl");
    let cache_path = dir.join("cache.kdl");
    let options = SeedUiOptions {
        identities: 4,
        games: 19,
        password: "testing".to_string(),
        force: false,
    };

    let summary = seed_ui_to_paths(&options, &wallet_path, &cache_path).expect("seed data");
    let wallet = load_wallet_from("testing", &wallet_path)
        .expect("load wallet")
        .expect("wallet exists");
    let cache = cache::load_cache_from(&cache_path).expect("load cache");

    assert_eq!(summary.identities, 4);
    assert_eq!(summary.games, 19);
    assert_eq!(wallet.identities.len(), 4);
    assert_eq!(cache.games.len(), 19);
    assert!(cache.games.iter().all(|game| !game.gate_npub.is_empty()));

    let _ = fs::remove_file(wallet_path);
    let _ = fs::remove_file(cache_path);
    let _ = fs::remove_dir(dir);
}

#[test]
fn seed_ui_refuses_to_overwrite_without_force() {
    let dir = temp_dir("seed-overwrite");
    let wallet_path = dir.join("wallet.kdl");
    let cache_path = dir.join("cache.kdl");
    fs::write(&wallet_path, b"occupied").expect("wallet marker");
    fs::write(&cache_path, b"occupied").expect("cache marker");

    let options = SeedUiOptions {
        identities: 2,
        games: 5,
        password: "testing".to_string(),
        force: false,
    };
    let err = seed_ui_to_paths(&options, &wallet_path, &cache_path).expect_err("must refuse");
    let message = err.to_string();
    assert!(message.contains("rerun with --force"));

    let force_options = SeedUiOptions {
        force: true,
        ..options
    };
    let summary = seed_ui_to_paths(&force_options, &wallet_path, &cache_path).expect("overwrite");
    assert_eq!(summary.identities, 2);

    let wallet = load_wallet_from("testing", &wallet_path)
        .expect("load wallet")
        .expect("wallet exists");
    assert_eq!(wallet.identities.len(), 2);

    let _ = fs::remove_file(wallet_path);
    let _ = fs::remove_file(cache_path);
    let _ = fs::remove_dir(dir);
}

#[test]
fn seed_ui_allows_oversized_wallet_for_ui_stress() {
    let dir = temp_dir("seed-max");
    let wallet_path = dir.join("wallet.kdl");
    let cache_path = dir.join("cache.kdl");
    let options = SeedUiOptions {
        identities: 24,
        games: 5,
        password: "testing".to_string(),
        force: false,
    };

    let summary = seed_ui_to_paths(&options, &wallet_path, &cache_path).expect("oversized seed");
    let wallet = load_wallet_from("testing", &wallet_path)
        .expect("load wallet")
        .expect("wallet exists");
    assert_eq!(summary.identities, 24);
    assert_eq!(wallet.identities.len(), 24);

    let _ = fs::remove_file(wallet_path);
    let _ = fs::remove_file(cache_path);
    let _ = fs::remove_dir(dir);
}

#[test]
fn seed_localhost_fixture_writes_isolated_wallet_cache_and_config() {
    let dir = temp_dir("seed-localhost");
    let wallet_path = dir.join("wallet.kdl");
    let cache_path = dir.join("cache.kdl");
    let config_path = dir.join("config.kdl");
    let keys = Keys::generate();
    let gate = Keys::generate();
    let options = SeedLocalhostFixtureOptions {
        nsec: keys.secret_key().to_secret_hex(),
        password: "testing".to_string(),
        relay_url: "ws://localhost:8080".to_string(),
        game_id: "player1-ui".to_string(),
        game_name: "Player 1 TUI Stress".to_string(),
        player_name: Some("Aurora".to_string()),
        server: "localhost".to_string(),
        port: 22,
        seat: 1,
        gate_npub: gate.public_key().to_bech32().expect("gate npub"),
        joined: Some("2026-03-30T20:00:00Z".to_string()),
        force: false,
    };

    let summary =
        seed_localhost_fixture_to_paths(&options, &wallet_path, &cache_path, &config_path)
            .expect("seed localhost fixture");
    let wallet = load_wallet_from("testing", &wallet_path)
        .expect("load wallet")
        .expect("wallet exists");
    let cache = cache::load_cache_from(&cache_path).expect("load cache");
    let config = load_config_from(&config_path).expect("load config");

    assert_eq!(wallet.identities.len(), 1);
    assert_eq!(wallet.identities[0].identity_type, IdentityType::Imported);
    assert_eq!(
        active_identity_npub(&wallet).expect("player npub"),
        summary.player_npub
    );

    assert_eq!(cache.games.len(), 1);
    let game = &cache.games[0];
    assert_eq!(game.id, "player1-ui");
    assert_eq!(game.name, "Player 1 TUI Stress");
    assert_eq!(game.player_name.as_deref(), Some("Aurora"));
    assert_eq!(game.server, "localhost");
    assert_eq!(game.port, 22);
    assert_eq!(game.seat, 1);
    assert_eq!(game.relay_url.as_deref(), Some("ws://localhost:8080"));
    assert_eq!(game.npub, summary.player_npub);
    assert_eq!(game.gate_npub, summary.gate_npub);
    assert_eq!(game.joined, "2026-03-30T20:00:00Z");

    assert_eq!(config.default_relay_url(), Some("ws://localhost:8080"));

    let _ = fs::remove_file(wallet_path);
    let _ = fs::remove_file(cache_path);
    let _ = fs::remove_file(config_path);
    let _ = fs::remove_dir(dir);
}
