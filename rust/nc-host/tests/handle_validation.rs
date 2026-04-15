mod common;

use common::create_test_game;
use nc_host::game::effects::GameEffects;
use nc_host::game::worker::GameWorker;
use nc_nostr::invite_request::{InviteRequest, InviteRequestReceipt, InviteRequestReceiptStatus};
use nc_nostr::state_sync::{StateErrorCode, StateErrorPayload, StateRequest};
use nc_nostr::turn_commands::{TurnCommands, TurnReceipt, TurnReceiptStatus};

fn seed_taken_handle(game_dir: &std::path::Path, npub: &str, handle: &str) {
    let roster_path = game_dir.parent().expect("games root").join("roster.db");
    let roster = nc_data::hosted::RosterStore::open(&roster_path).expect("open roster");
    nc_data::hosted::upsert_player_seen(
        roster.connection(),
        npub,
        Some(handle),
        game_dir
            .file_name()
            .and_then(|name| name.to_str())
            .expect("game id"),
    )
    .expect("seed roster");
}

#[tokio::test]
async fn invite_request_rejects_taken_handle() {
    let (_temp, game_dir, store) = create_test_game("handle-invite", 4);
    seed_taken_handle(&game_dir, "existing-player", "StarRider");

    let worker = GameWorker::new("handle-invite".to_string(), game_dir.join("hosted.db"));
    worker
        .handle_effect(GameEffects::HandleInviteRequest {
            request: InviteRequest {
                request_id: "invite-001".to_string(),
                game_id: "handle-invite".to_string(),
                player_pubkey: "new-player".to_string(),
                message: "Let me in".to_string(),
                handle: Some("starrider".to_string()),
            },
            game_id: "handle-invite".to_string(),
        })
        .await;

    let pending = nc_data::hosted::get_pending(store.connection(), "handle-invite", 10)
        .expect("pending outbox");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, 30514);

    let receipt: InviteRequestReceipt =
        serde_json::from_str(&pending[0].content).expect("invite receipt");
    assert_eq!(receipt.status, InviteRequestReceiptStatus::HandleTaken);

    let requests =
        nc_data::hosted::list_requests(store.connection(), "handle-invite").expect("list requests");
    assert!(requests.is_empty());
}

#[tokio::test]
async fn state_request_rejects_taken_handle_for_claimed_player() {
    let (_temp, game_dir, store) = create_test_game("handle-state", 4);
    nc_data::hosted::claim_seat(store.connection(), "handle-state", 1, "player-001", 3000)
        .expect("claim seat");
    seed_taken_handle(&game_dir, "existing-player", "StarRider");

    let worker = GameWorker::new("handle-state".to_string(), game_dir.join("hosted.db"));
    worker
        .handle_effect(GameEffects::HandleStateRequest {
            request: StateRequest {
                request_id: "state-001".to_string(),
                game_id: "handle-state".to_string(),
                player_pubkey: "player-001".to_string(),
                last_turn: None,
                last_hash: None,
                handle: Some("STARrider".to_string()),
            },
        })
        .await;

    let pending = nc_data::hosted::get_pending(store.connection(), "handle-state", 10)
        .expect("pending outbox");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, 30520);

    let payload: StateErrorPayload =
        serde_json::from_str(&pending[0].content).expect("state error payload");
    assert_eq!(payload.code, StateErrorCode::HandleTaken);
}

#[tokio::test]
async fn turn_submit_rejects_taken_handle() {
    let (_temp, game_dir, store) = create_test_game("handle-turn", 4);
    nc_data::hosted::claim_seat(store.connection(), "handle-turn", 1, "player-001", 3000)
        .expect("claim seat");
    seed_taken_handle(&game_dir, "existing-player", "StarRider");

    let worker = GameWorker::new("handle-turn".to_string(), game_dir.join("hosted.db"));
    worker
        .handle_effect(GameEffects::HandleTurnCommands {
            commands: TurnCommands {
                submit_id: "turn-001".to_string(),
                game_id: "handle-turn".to_string(),
                turn: 5,
                player_pubkey: "player-001".to_string(),
                commands: "fleet 1 { order kind=\"guard\" }".to_string(),
                handle: Some("starrider".to_string()),
            },
            game_id: "handle-turn".to_string(),
        })
        .await;

    let pending = nc_data::hosted::get_pending(store.connection(), "handle-turn", 10)
        .expect("pending outbox");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, 30524);

    let receipt: TurnReceipt = serde_json::from_str(&pending[0].content).expect("turn receipt");
    assert_eq!(receipt.status, TurnReceiptStatus::Rejected);
    assert!(
        receipt
            .errors
            .iter()
            .any(|error| error.path == "handle" && error.message == "handle_taken")
    );

    assert!(
        nc_data::hosted::get_pending_turn(store.connection(), "handle-turn", 5, "player-001")
            .expect("pending turn lookup")
            .is_none()
    );
}
