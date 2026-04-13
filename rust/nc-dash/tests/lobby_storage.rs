use nc_dash::lobby::storage::{
    cache::{
        JoinedGameCacheEntry, LobbyCacheRecord, PendingRequestCacheEntry, parse_cache_kdl,
        render_cache_kdl,
    },
    config::{LobbyConfigRecord, parse_config_kdl, render_config_kdl},
    keychain::{
        KeychainIdentityRecord, LobbyKeychainRecord, parse_keychain_kdl, render_keychain_kdl,
    },
    settings::{LobbySettingsRecord, parse_settings_kdl, render_settings_kdl},
};

#[test]
fn keychain_round_trip_uses_keychain_root_and_identity_rows() {
    let record = LobbyKeychainRecord {
        active: 0,
        handle: Some("StarRider".to_string()),
        identities: vec![KeychainIdentityRecord {
            npub: "npub1example".to_string(),
            nsec: "nsec1example".to_string(),
            identity_type: "local".to_string(),
            created: "2026-04-12T00:00:00Z".to_string(),
        }],
    };

    let rendered = render_keychain_kdl(&record);
    let reparsed = parse_keychain_kdl(&rendered).expect("reparse keychain");

    assert_eq!(reparsed, record);
}

#[test]
fn cache_round_trip_preserves_joined_and_pending_rows() {
    let record = LobbyCacheRecord {
        joined_games: vec![JoinedGameCacheEntry {
            game_id: "friday-night".to_string(),
            game_name: "Friday Night NC".to_string(),
            host_alias: "Green Host".to_string(),
            seat: Some(2),
            state: "joined".to_string(),
        }],
        pending_requests: vec![PendingRequestCacheEntry {
            game_id: "friday-night".to_string(),
            status: "received".to_string(),
        }],
    };

    let rendered = render_cache_kdl(&record);
    let reparsed = parse_cache_kdl(&rendered).expect("reparse cache");

    assert_eq!(reparsed, record);
}

#[test]
fn config_and_settings_round_trip_through_kdl() {
    let config = LobbyConfigRecord {
        relay: Some("wss://relay.example.com".to_string()),
    };
    let settings = LobbySettingsRecord {
        lock_timeout_minutes: 15,
        follow_mouse_on_map: false,
        dense_empty_sector_dots: true,
        theme_key: "rose-pine".to_string(),
    };

    let reparsed_config = parse_config_kdl(&render_config_kdl(&config)).expect("config");
    let reparsed_settings = parse_settings_kdl(&render_settings_kdl(&settings)).expect("settings");

    assert_eq!(reparsed_config, config);
    assert_eq!(reparsed_settings, settings);
}

#[test]
fn settings_default_lock_timeout_is_ten_minutes() {
    let settings = parse_settings_kdl("settings\n").expect("settings");

    assert_eq!(settings.lock_timeout_minutes, 10);
}
