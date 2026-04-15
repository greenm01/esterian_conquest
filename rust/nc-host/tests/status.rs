mod common;

use common::create_test_game_in_root;
use nc_data::hosted::{
    CatalogState, GameSettings, LobbyVisibility, OutboxStatus, RecruitingMode, approve_request,
    count_by_status, count_pending_requests, count_pending_turns, count_unpublished_decisions,
    create_request, enqueue, enqueue_turn, mark_failed, update_settings,
};
use nc_host::config::host_config::HostConfig;
use nc_host::status::{collect, render};

#[test]
fn collect_status_aggregates_games_and_queues() {
    let temp = tempfile::tempdir().expect("temp dir should create");
    let games_root = temp.path();

    let (_alpha_dir, alpha_store) = create_test_game_in_root(games_root, "alpha", 4);
    let (_beta_dir, beta_store) = create_test_game_in_root(games_root, "beta", 3);

    alpha_store
        .connection()
        .execute(
            "UPDATE game_metadata
             SET status = 'active',
                 current_year = 3004,
                 current_turn = 7,
                 maintenance_next_due_unix_seconds = ?1
             WHERE id = 'alpha'",
            rusqlite::params![chrono::Utc::now().timestamp() - 60],
        )
        .expect("alpha metadata should update");
    beta_store
        .connection()
        .execute(
            "UPDATE game_metadata
             SET status = 'active',
                 current_year = 3002,
                 current_turn = 2,
                 maintenance_enabled = 0
             WHERE id = 'beta'",
            [],
        )
        .expect("beta metadata should update");
    update_settings(
        beta_store.connection(),
        "beta",
        &GameSettings {
            recruiting: RecruitingMode::None,
            lobby_visibility: LobbyVisibility::Private,
            catalog_state: CatalogState::Listed,
            host_alias: Some("Hidden Host".to_string()),
            summary: Some("Private game".to_string()),
            maintenance_enabled: false,
            maintenance_interval_minutes: 1440,
            maintenance_next_due_unix_seconds: None,
            game_tier: nc_data::hosted::GameTier::League,
        },
    )
    .expect("beta settings should update");

    create_request(
        alpha_store.connection(),
        "req-a",
        "alpha",
        "player-a",
        "invite me",
    )
    .expect("request should create");
    create_request(
        alpha_store.connection(),
        "req-b",
        "alpha",
        "player-b",
        "another invite",
    )
    .expect("request should create");
    approve_request(alpha_store.connection(), "req-b", "Approved", 2, None)
        .expect("request should approve");
    enqueue_turn(
        alpha_store.connection(),
        "turn-a",
        "alpha",
        7,
        "player-a",
        "{\"orders\":[]}",
    )
    .expect("turn should enqueue");
    enqueue(
        alpha_store.connection(),
        "outbox-pending",
        "alpha",
        30514,
        "player-a",
        "{\"ok\":true}",
        "[]",
    )
    .expect("pending outbox should enqueue");
    enqueue(
        alpha_store.connection(),
        "outbox-failed",
        "alpha",
        30524,
        "player-a",
        "{\"ok\":false}",
        "[]",
    )
    .expect("failed outbox should enqueue");
    for _ in 0..5 {
        mark_failed(alpha_store.connection(), "outbox-failed", "relay down")
            .expect("failure should record");
    }

    assert_eq!(
        count_pending_requests(alpha_store.connection(), "alpha").expect("count should work"),
        1
    );
    assert_eq!(
        count_unpublished_decisions(alpha_store.connection(), "alpha").expect("count should work"),
        1
    );
    assert_eq!(
        count_pending_turns(alpha_store.connection(), "alpha").expect("count should work"),
        1
    );
    assert_eq!(
        count_by_status(alpha_store.connection(), "alpha", OutboxStatus::Pending)
            .expect("count should work"),
        1
    );
    assert_eq!(
        count_by_status(alpha_store.connection(), "alpha", OutboxStatus::Failed)
            .expect("count should work"),
        1
    );

    let config = HostConfig {
        games_root: games_root.to_path_buf(),
        relay_url: "not-a-relay-url".to_string(),
        invite_relay_host: "relay.example.com".to_string(),
        identity_path: temp.path().join("host.nsec"),
        sysop_contact_npub: "npub1test".to_string(),
        sysop_contact_label: None,
        sysop_contact_nip05: None,
    };

    let report = collect::collect_status(&config, None).expect("status should collect");
    assert_eq!(report.totals.discovered_games, 2);
    assert_eq!(report.totals.public_recruiting_games, 1);
    assert_eq!(report.totals.due_maintenance_games, 1);
    assert_eq!(report.totals.pending_requests, 1);
    assert_eq!(report.totals.pending_decisions, 1);
    assert_eq!(report.totals.pending_turns, 1);
    assert_eq!(report.totals.outbox_pending, 1);
    assert_eq!(report.totals.outbox_failed, 1);
    assert!(!report.relay.reachable);
    assert_eq!(report.relay.status, "invalid");

    let alpha = report
        .games
        .iter()
        .find(|game| game.game_id == "alpha")
        .expect("alpha status should exist");
    assert_eq!(alpha.name, "alpha");
    assert_eq!(alpha.status, "active");
    assert_eq!(alpha.year, 3004);
    assert_eq!(alpha.turn, 7);
    assert_eq!(alpha.pending_requests, 1);
    assert_eq!(alpha.pending_decisions, 1);
    assert_eq!(alpha.pending_turns, 1);
    assert_eq!(alpha.outbox_pending, 1);
    assert_eq!(alpha.outbox_failed, 1);
    assert!(alpha.maintenance_due_now);

    let beta = report
        .games
        .iter()
        .find(|game| game.game_id == "beta")
        .expect("beta status should exist");
    assert_eq!(beta.recruiting, "none");
    assert_eq!(beta.lobby_visibility, "private");
    assert!(!beta.maintenance_enabled);
}

#[test]
fn render_human_mentions_totals_and_games() {
    let temp = tempfile::tempdir().expect("temp dir should create");
    let games_root = temp.path();
    create_test_game_in_root(games_root, "gamma", 2);

    let config = HostConfig {
        games_root: games_root.to_path_buf(),
        relay_url: "not-a-relay-url".to_string(),
        invite_relay_host: "relay.example.com".to_string(),
        identity_path: temp.path().join("host.nsec"),
        sysop_contact_npub: "npub1test".to_string(),
        sysop_contact_label: None,
        sysop_contact_nip05: None,
    };

    let report = collect::collect_status(&config, None).expect("status should collect");
    let rendered = render::render_human(&report);
    assert!(rendered.contains("nc-host status"));
    assert!(rendered.contains("Totals: games=1"));
    assert!(rendered.contains("gamma"));
}
