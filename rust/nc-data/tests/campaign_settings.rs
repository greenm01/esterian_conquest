use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{
    CampaignSettings, CampaignStore, CoreGameData, HostedSeat, HostedSeatStatus, SeatReservation,
};
use nc_engine::build_seeded_new_game;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp dir");
    root
}

fn seeded_store(prefix: &str) -> (PathBuf, CampaignStore, CoreGameData) {
    let root = temp_dir(prefix);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    let game_data = build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    store
        .save_runtime_state_structured(&game_data, &BTreeSet::new(), &[], &[])
        .expect("save runtime state");
    (root, store, game_data)
}

#[test]
fn campaign_settings_round_trip_and_update_runtime_policy_bytes() {
    let (root, store, _) = seeded_store("nc-data-settings-roundtrip");

    let settings = CampaignSettings {
        slug: "friday-night".to_string(),
        game_name: "Friday Night EC".to_string(),
        default_theme_key: "matrix".to_string(),
        snoop_enabled: false,
        session_max_idle_minutes: 7,
        session_minimum_time_minutes: 9,
        session_local_timeout: true,
        session_remote_timeout: false,
        inactivity_purge_after_turns: 4,
        inactivity_autopilot_after_turns: 3,
        maintenance_enabled: true,
        maintenance_interval_minutes: 30,
        maintenance_next_due_unix_seconds: Some(12_345),
        reservations: vec![SeatReservation {
            player_record_index_1_based: 2,
            alias: "Sysop".to_string(),
        }],
    };
    store
        .save_campaign_settings(&settings)
        .expect("save settings");

    let loaded = store.load_campaign_settings().expect("load settings");
    assert_eq!(loaded, settings);

    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime")
        .expect("runtime snapshot");
    assert!(!runtime.game_data.setup.snoop_enabled());
    assert_eq!(
        runtime.game_data.setup.max_time_between_keys_minutes_raw(),
        7
    );
    assert_eq!(
        runtime.game_data.setup.minimum_time_granted_minutes_raw(),
        9
    );
    assert!(runtime.game_data.setup.local_timeout_enabled());
    assert!(!runtime.game_data.setup.remote_timeout_enabled());
    assert_eq!(runtime.game_data.setup.purge_after_turns_raw(), 4);
    assert_eq!(runtime.game_data.setup.autopilot_inactive_turns_raw(), 3);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn session_leases_allow_same_identity_retries_and_expire() {
    let (root, store, _) = seeded_store("nc-data-session-leases");
    store
        .save_campaign_settings(&CampaignSettings::new("friday-night", "Friday Night EC"))
        .expect("seed settings");

    let pending = store
        .create_pending_session_lease("token-1", 2, "npub-player-1", 100, 60)
        .expect("create pending lease");
    assert_eq!(pending.player_record_index_1_based, 2);
    assert_eq!(pending.state, nc_data::SessionLeaseState::PendingSsh);

    let retried = store
        .create_pending_session_lease("token-2", 2, "npub-player-1", 110, 60)
        .expect("same-identity pending retry should replace the old lease");
    assert_eq!(retried.session_token, "token-2");
    assert_eq!(retried.started_at_unix_seconds, 110);
    assert_eq!(retried.expires_at_unix_seconds, 170);
    assert!(matches!(
        store.load_session_lease("token-1", 110),
        Err(nc_data::SessionLeaseError::InvalidToken)
    ));

    let err = store
        .create_pending_session_lease("token-3", 2, "npub-player-2", 111, 60)
        .expect_err("foreign pending lease for the same seat should still fail");
    assert!(matches!(err, nc_data::SessionLeaseError::SeatBusy { .. }));

    let active = store
        .activate_session_lease("token-2", 120, 60)
        .expect("activate lease");
    assert_eq!(active.state, nc_data::SessionLeaseState::Active);

    let retried_active = store
        .create_pending_session_lease("token-4", 2, "npub-player-1", 121, 60)
        .expect("same-identity active retry should replace the old lease");
    assert_eq!(retried_active.session_token, "token-4");
    assert_eq!(
        retried_active.state,
        nc_data::SessionLeaseState::PendingSsh
    );
    assert!(matches!(
        store.load_session_lease("token-2", 121),
        Err(nc_data::SessionLeaseError::InvalidToken)
    ));

    let err = store
        .create_pending_session_lease("token-5", 2, "npub-player-2", 122, 60)
        .expect_err("foreign retry against a replaced live seat should still fail");
    assert!(matches!(err, nc_data::SessionLeaseError::SeatBusy { .. }));

    let heartbeated = store
        .heartbeat_session_lease("token-4", 150, 60)
        .expect("heartbeat lease");
    assert_eq!(heartbeated.expires_at_unix_seconds, 210);
    assert!(
        store
            .has_live_session_leases(151)
            .expect("check active lease")
    );
    assert!(
        store
            .live_session_for_npub("npub-player-1", 151)
            .expect("lookup live lease")
            .is_some()
    );
    assert!(
        !store
            .has_live_session_leases(211)
            .expect("expired lease should be pruned")
    );
    assert!(
        store
            .live_session_for_npub("npub-player-1", 211)
            .expect("expired lease should disappear")
            .is_none()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn campaign_settings_validate_duplicate_reservations() {
    let err = CampaignSettings {
        slug: "friday-night".to_string(),
        game_name: "Friday Night EC".to_string(),
        default_theme_key: "tokyo_night".to_string(),
        snoop_enabled: true,
        session_max_idle_minutes: 10,
        session_minimum_time_minutes: 0,
        session_local_timeout: false,
        session_remote_timeout: true,
        inactivity_purge_after_turns: 0,
        inactivity_autopilot_after_turns: 0,
        maintenance_enabled: false,
        maintenance_interval_minutes: 60,
        maintenance_next_due_unix_seconds: None,
        reservations: vec![
            SeatReservation {
                player_record_index_1_based: 1,
                alias: "sysop".to_string(),
            },
            SeatReservation {
                player_record_index_1_based: 2,
                alias: "Sysop".to_string(),
            },
        ],
    }
    .validate()
    .expect_err("duplicate aliases should be rejected");

    assert!(format!("{err}").contains("duplicate reservation alias"));
}

#[test]
fn campaign_settings_can_coexist_with_hosted_seats() {
    let (root, store, _) = seeded_store("nc-data-settings-hosted-seats");
    store
        .save_campaign_settings(&CampaignSettings::new("friday-night", "Friday Night EC"))
        .expect("save settings");
    store
        .replace_hosted_seats(&[
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "alpha-beta".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
            HostedSeat {
                player_record_index_1_based: 2,
                invite_code: "gamma-delta".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
        ])
        .expect("save hosted seats");

    let settings = store.load_campaign_settings().expect("load settings");
    let seats = store.hosted_seats().expect("load hosted seats");
    assert_eq!(settings.game_name, "Friday Night EC");
    assert_eq!(seats.len(), 2);

    let _ = fs::remove_dir_all(root);
}
