#![allow(dead_code)]

use nc_daemon::invite::generate_invite_code;
use blake3::Hasher;
use nc_data::hosted::HostedStore;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub fn hash_invite_code(code: &str) -> String {
    Hasher::new()
        .update(code.as_bytes())
        .finalize()
        .to_hex()
        .to_string()
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn rust_workspace() -> PathBuf {
    repo_root().join("rust")
}

pub fn temp_game_dir(prefix: &str) -> (TempDir, PathBuf) {
    let temp = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .expect("temp dir should create");
    let path = temp.path().to_path_buf();
    (temp, path)
}

pub fn create_test_game(game_id: &str, player_count: u32) -> (TempDir, PathBuf, HostedStore) {
    let (temp, path) = temp_game_dir(&format!("nc-daemon-test-{}-", game_id));
    let path = path.clone();
    let game_dir = path.join(game_id);
    fs::create_dir_all(&game_dir).expect("game dir should create");

    let db_path = game_dir.join("hosted.db");
    let store = HostedStore::create(&db_path).expect("store should create");

    let now = chrono::Utc::now().timestamp();
    store
        .connection()
        .execute(
            "INSERT INTO game_metadata (id, name, status, created_at, updated_at, current_year, current_turn, players)
             VALUES (?1, ?2, 'setup', ?3, ?3, 3000, 0, ?4)",
            rusqlite::params![game_id, game_id, now, player_count],
        )
        .expect("game metadata should insert");

    let mut existing = std::collections::HashSet::new();
    let invite_codes = (0..player_count)
        .map(|_| {
            let code = generate_invite_code(&existing);
            existing.insert(code.clone());
            code
        })
        .collect::<Vec<_>>();

    nc_data::hosted::create_seats(store.connection(), game_id, &invite_codes)
        .expect("seats should create");

    nc_data::hosted::update_settings(
        store.connection(),
        game_id,
        &nc_data::hosted::GameSettings {
            recruiting: nc_data::hosted::RecruitingMode::NewPlayers,
            lobby_visibility: nc_data::hosted::LobbyVisibility::Public,
            host_alias: Some("Test Host".to_string()),
            summary: Some("Test game for nc-daemon".to_string()),
            maintenance_enabled: true,
            maintenance_interval_minutes: 1440,
            maintenance_next_due_unix_seconds: None,
        },
    )
    .expect("settings should update");

    (temp, game_dir, store)
}

pub fn create_seat_with_code(
    store: &HostedStore,
    game_id: &str,
    seat_number: u32,
    invite_code: &str,
) {
    nc_data::hosted::open_seat(store.connection(), game_id, seat_number, invite_code)
        .expect("seat should open");
    let seats = nc_data::hosted::list_seats(store.connection(), game_id).expect("should list");
    assert!(
        seats.iter().any(|s| s.seat_number == seat_number),
        "seat should exist"
    );
}

pub fn cleanup_temp_dir(temp: TempDir) {
    drop(temp);
}
