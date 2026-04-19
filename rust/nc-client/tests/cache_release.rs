use nc_client::cache::{CachedGame, ClientCache, parse_cache_str, render_cache};

fn cached_game(id: &str) -> CachedGame {
    CachedGame {
        id: id.to_string(),
        name: "Sandbox".to_string(),
        game_tier: Some("sandbox".to_string()),
        host_alias: Some("daemon".to_string()),
        host_contact_npub: None,
        host_contact_label: None,
        host_contact_nip05: None,
        relay_url: "ws://127.0.0.1:8080".to_string(),
        daemon_pubkey: "daemon-pubkey".to_string(),
        seat: Some(1),
        status: "joined".to_string(),
        invite_address: None,
        last_turn: Some(1),
        last_hash: Some("hash-1".to_string()),
        updated_at: "2026-04-19T00:00:00Z".to_string(),
    }
}

#[test]
fn marking_game_released_round_trips_through_kdl_cache() {
    let mut cache = ClientCache::empty();
    cache.mark_game_released("phase-sapling-awful");

    let rendered = render_cache(&cache);
    let parsed = parse_cache_str(&rendered).expect("parse rendered cache");

    assert!(parsed.is_game_released("phase-sapling-awful"));
}

#[test]
fn upserting_game_clears_release_tombstone() {
    let mut cache = ClientCache::empty();
    cache.mark_game_released("phase-sapling-awful");

    cache.upsert_game(cached_game("phase-sapling-awful"));

    assert!(!cache.is_game_released("phase-sapling-awful"));
}
