use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use ec_connect::cache;
use ec_connect::dev_seed::{SeedUiOptions, seed_ui_to_paths};
use ec_connect::wallet::io::load_wallet_from;

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
