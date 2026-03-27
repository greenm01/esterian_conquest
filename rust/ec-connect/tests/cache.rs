//! Regression tests for game cache I/O (step 4).

use std::path::PathBuf;

use ec_connect::cache::io::{load_cache_from, parse_cache_str, render_cache, save_cache_to};
use ec_connect::cache::{CachedGame, GameCache};

// ---------------------------------------------------------------------------
// parse_cache_str
// ---------------------------------------------------------------------------

#[test]
fn parse_empty_string_returns_empty_cache() {
    let cache = parse_cache_str("").unwrap();
    assert!(cache.games.is_empty());
}

const SAMPLE_CACHE: &str = r#"game id="friday-night" name="Friday Night EC" server="play.example.com" port=22 seat=2 npub="npub1aaa" joined="2026-03-26T12:00:00Z" last-connected="2026-03-28T19:30:00Z"
game id="saturday-showdown" name="Saturday Showdown" server="war.example.com" port=22 seat=5 npub="npub1bbb" joined="2026-03-27T10:00:00Z"
"#;

#[test]
fn parse_sample_cache() {
    let cache = parse_cache_str(SAMPLE_CACHE).unwrap();
    assert_eq!(cache.games.len(), 2);

    let g0 = &cache.games[0];
    assert_eq!(g0.id, "friday-night");
    assert_eq!(g0.name, "Friday Night EC");
    assert_eq!(g0.server, "play.example.com");
    assert_eq!(g0.port, 22);
    assert_eq!(g0.seat, 2);
    assert_eq!(g0.npub, "npub1aaa");
    assert_eq!(g0.joined, "2026-03-26T12:00:00Z");
    assert_eq!(g0.last_connected.as_deref(), Some("2026-03-28T19:30:00Z"));

    let g1 = &cache.games[1];
    assert_eq!(g1.id, "saturday-showdown");
    assert_eq!(g1.seat, 5);
    assert!(g1.last_connected.is_none());
}

#[test]
fn parse_game_without_last_connected() {
    let kdl = "game id=\"x\" name=\"X\" server=\"localhost\" port=22 seat=1 npub=\"npub1x\" joined=\"2026-01-01T00:00:00Z\"\n";
    let cache = parse_cache_str(kdl).unwrap();
    assert!(cache.games[0].last_connected.is_none());
}

#[test]
fn parse_game_port_defaults_to_22_when_omitted() {
    let kdl = "game id=\"x\" name=\"X\" server=\"localhost\" seat=1 npub=\"npub1x\" joined=\"2026-01-01T00:00:00Z\"\n";
    let cache = parse_cache_str(kdl).unwrap();
    assert_eq!(cache.games[0].port, 22);
}

#[test]
fn parse_game_missing_id_is_err() {
    let kdl = "game name=\"X\" server=\"localhost\" port=22 seat=1 npub=\"npub1\" joined=\"2026-01-01T00:00:00Z\"\n";
    assert!(parse_cache_str(kdl).is_err());
}

#[test]
fn parse_game_missing_seat_is_err() {
    let kdl = "game id=\"x\" name=\"X\" server=\"localhost\" port=22 npub=\"npub1\" joined=\"2026-01-01T00:00:00Z\"\n";
    assert!(parse_cache_str(kdl).is_err());
}

#[test]
fn parse_unknown_nodes_are_ignored() {
    let kdl = "future-thing foo=\"bar\"\ngame id=\"x\" name=\"X\" server=\"localhost\" port=22 seat=1 npub=\"npub1\" joined=\"2026-01-01T00:00:00Z\"\n";
    let cache = parse_cache_str(kdl).unwrap();
    assert_eq!(cache.games.len(), 1);
}

// ---------------------------------------------------------------------------
// render_cache / round-trip
// ---------------------------------------------------------------------------

#[test]
fn render_empty_cache_is_empty_string() {
    let cache = GameCache::empty();
    assert_eq!(render_cache(&cache), "");
}

#[test]
fn render_roundtrip() {
    let cache = parse_cache_str(SAMPLE_CACHE).unwrap();
    let rendered = render_cache(&cache);
    let cache2 = parse_cache_str(&rendered).unwrap();

    assert_eq!(cache2.games.len(), 2);
    assert_eq!(cache2.games[0].id, "friday-night");
    assert_eq!(
        cache2.games[0].last_connected.as_deref(),
        Some("2026-03-28T19:30:00Z")
    );
    assert_eq!(cache2.games[1].id, "saturday-showdown");
    assert!(cache2.games[1].last_connected.is_none());
}

// ---------------------------------------------------------------------------
// GameCache helpers
// ---------------------------------------------------------------------------

fn make_game(id: &str, joined: &str, last: Option<&str>) -> CachedGame {
    CachedGame {
        id: id.to_string(),
        name: id.to_string(),
        server: "localhost".to_string(),
        port: 22,
        seat: 1,
        npub: "npub1test".to_string(),
        joined: joined.to_string(),
        last_connected: last.map(str::to_string),
    }
}

#[test]
fn upsert_adds_new_game() {
    let mut cache = GameCache::empty();
    cache.upsert(make_game("alpha", "2026-01-01T00:00:00Z", None));
    assert_eq!(cache.games.len(), 1);
}

#[test]
fn upsert_replaces_existing_game() {
    let mut cache = GameCache::empty();
    cache.upsert(make_game("alpha", "2026-01-01T00:00:00Z", None));
    cache.upsert(make_game(
        "alpha",
        "2026-01-01T00:00:00Z",
        Some("2026-03-01T00:00:00Z"),
    ));
    assert_eq!(cache.games.len(), 1);
    assert!(cache.games[0].last_connected.is_some());
}

#[test]
fn touch_updates_last_connected() {
    let mut cache = GameCache::empty();
    cache.upsert(make_game("alpha", "2026-01-01T00:00:00Z", None));
    cache.touch("alpha", "2026-05-01T00:00:00Z");
    assert_eq!(
        cache.games[0].last_connected.as_deref(),
        Some("2026-05-01T00:00:00Z")
    );
}

#[test]
fn touch_noop_on_missing_game() {
    let mut cache = GameCache::empty();
    cache.touch("nonexistent", "2026-01-01T00:00:00Z");
    assert!(cache.games.is_empty());
}

#[test]
fn sorted_puts_last_connected_first_in_descending_order() {
    let mut cache = GameCache::empty();
    cache.upsert(make_game(
        "a",
        "2026-01-01T00:00:00Z",
        Some("2026-03-01T00:00:00Z"),
    ));
    cache.upsert(make_game(
        "b",
        "2026-01-01T00:00:00Z",
        Some("2026-05-01T00:00:00Z"),
    ));
    cache.upsert(make_game("c", "2026-01-01T00:00:00Z", None));

    let sorted = cache.sorted();
    assert_eq!(sorted[0].id, "b"); // most recent last-connected
    assert_eq!(sorted[1].id, "a");
    assert_eq!(sorted[2].id, "c"); // no last-connected, goes last
}

#[test]
fn sorted_no_last_connected_sorted_by_joined_descending() {
    let mut cache = GameCache::empty();
    cache.upsert(make_game("early", "2026-01-01T00:00:00Z", None));
    cache.upsert(make_game("late", "2026-06-01T00:00:00Z", None));

    let sorted = cache.sorted();
    assert_eq!(sorted[0].id, "late");
    assert_eq!(sorted[1].id, "early");
}

// ---------------------------------------------------------------------------
// save_cache_to / load_cache_from
// ---------------------------------------------------------------------------

fn tmp_cache_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("ec_connect_test_cache_{name}.kdl"));
    p
}

#[test]
fn load_cache_from_missing_file_returns_empty() {
    let path = tmp_cache_path("missing_xyz_99999");
    let _ = std::fs::remove_file(&path);
    let cache = load_cache_from(&path).unwrap();
    assert!(cache.games.is_empty());
}

#[test]
fn save_load_cache_roundtrip() {
    let path = tmp_cache_path("roundtrip");
    let _ = std::fs::remove_file(&path);

    let mut cache = GameCache::empty();
    cache.upsert(make_game(
        "game-one",
        "2026-01-01T00:00:00Z",
        Some("2026-03-01T00:00:00Z"),
    ));
    cache.upsert(make_game("game-two", "2026-02-01T00:00:00Z", None));

    save_cache_to(&cache, &path).unwrap();
    let loaded = load_cache_from(&path).unwrap();

    assert_eq!(loaded.games.len(), 2);
    assert_eq!(loaded.games[0].id, "game-one");
    assert_eq!(
        loaded.games[0].last_connected.as_deref(),
        Some("2026-03-01T00:00:00Z")
    );
    assert_eq!(loaded.games[1].id, "game-two");
    assert!(loaded.games[1].last_connected.is_none());

    let _ = std::fs::remove_file(&path);
}
