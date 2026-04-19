mod common;

use common::{create_test_game, seed_runtime_snapshot};
use nc_data::hosted::{
    self, CatalogState, GameSettings, GameTier, LobbyVisibility, RecruitingMode,
};
use nc_host::game::effects::GameEffects;
use nc_host::game::worker::GameWorker;
use nc_nostr::sandbox_release::SandboxReleaseRequest;

#[tokio::test]
async fn sandbox_release_resets_claimed_seat_and_allows_rejoin() {
    let (_temp, game_dir, store) = create_test_game("sandbox-release-reset", 4);
    let game_id = "sandbox-release-reset";
    seed_runtime_snapshot(&game_dir, game_id, "Sandbox Release Reset", 4, &[], &[]);
    hosted::update_settings(
        store.connection(),
        game_id,
        &GameSettings {
            maintenance_enabled: true,
            maintenance_interval_minutes: 1440,
            maintenance_next_due_unix_seconds: None,
            lobby_visibility: LobbyVisibility::Public,
            recruiting: RecruitingMode::None,
            catalog_state: CatalogState::Listed,
            host_alias: Some("Test Host".to_string()),
            summary: Some("Sandbox release test".to_string()),
            game_tier: GameTier::Sandbox,
        },
    )
    .expect("sandbox settings");
    hosted::claim_seat(store.connection(), game_id, 1, "player-alpha", 3000).expect("claim seat");

    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));
    worker
        .handle_effect(GameEffects::HandleSandboxRelease {
            request: SandboxReleaseRequest {
                request_id: "release-001".to_string(),
                game_id: game_id.to_string(),
                player_pubkey: "player-alpha".to_string(),
            },
            game_id: game_id.to_string(),
        })
        .await;

    let seat = hosted::get_seat_by_number(store.connection(), game_id, 1)
        .expect("seat lookup")
        .expect("seat exists");
    assert_eq!(seat.status, hosted::SeatStatus::Pending);
    assert_eq!(seat.player_pubkey, None);

    let settings = hosted::get_settings(store.connection(), game_id).expect("settings");
    assert_eq!(settings.recruiting, RecruitingMode::NewPlayers);

    hosted::claim_seat(store.connection(), game_id, 1, "player-alpha", 3000).expect("reclaim seat");
    let reclaimed = hosted::get_seat_by_number(store.connection(), game_id, 1)
        .expect("seat lookup")
        .expect("seat exists");
    assert_eq!(reclaimed.status, hosted::SeatStatus::Claimed);
    assert_eq!(reclaimed.player_pubkey.as_deref(), Some("player-alpha"));
}

#[tokio::test]
async fn sandbox_release_rejects_non_sandbox_games() {
    let (_temp, game_dir, store) = create_test_game("sandbox-release-league", 4);
    let game_id = "sandbox-release-league";
    seed_runtime_snapshot(&game_dir, game_id, "Sandbox Release League", 4, &[], &[]);
    hosted::claim_seat(store.connection(), game_id, 1, "player-alpha", 3000).expect("claim seat");

    let worker = GameWorker::new(game_id.to_string(), game_dir.join("hosted.db"));
    worker
        .handle_effect(GameEffects::HandleSandboxRelease {
            request: SandboxReleaseRequest {
                request_id: "release-002".to_string(),
                game_id: game_id.to_string(),
                player_pubkey: "player-alpha".to_string(),
            },
            game_id: game_id.to_string(),
        })
        .await;

    let seat = hosted::get_seat_by_number(store.connection(), game_id, 1)
        .expect("seat lookup")
        .expect("seat exists");
    assert_eq!(seat.status, hosted::SeatStatus::Claimed);
    assert_eq!(seat.player_pubkey.as_deref(), Some("player-alpha"));
}
