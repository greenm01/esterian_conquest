use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_client::hosted::store::{HostedDraftStatus, HostedStateStore};
use nc_data::TurnSubmission;
use nc_nostr::state_sync::{
    GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
    HostedPlayerRosterEntry, HostedPlayerState, HostedQueuedMail, HostedReportBlock,
    HostedStardockSlot, HostedStarmapState, HostedStatePayload, HostedWorldState,
};

fn sample_snapshot() -> GameState {
    GameState {
        game_id: "friday-night".to_string(),
        turn: 4,
        year: 3004,
        player_seat: 1,
        player_name: "Terran Union".to_string(),
        state_hash: "abc123".to_string(),
        state: HostedStatePayload {
            player: HostedPlayerState {
                seat: 1,
                empire_name: "Terran Union".to_string(),
                handle: Some("StarRider".to_string()),
                mode: "active".to_string(),
                tax_rate: 33,
                planet_count: 1,
                starbase_count: 1,
                homeworld_planet_index: 1,
                last_run_year: 3004,
                diplomacy: vec![HostedDiplomacyState {
                    empire_id: 2,
                    relation: "enemy".to_string(),
                }],
            },
            roster: vec![
                HostedPlayerRosterEntry {
                    empire_id: 1,
                    empire_name: "Terran Union".to_string(),
                    is_self: true,
                },
                HostedPlayerRosterEntry {
                    empire_id: 2,
                    empire_name: "Rigel Empire".to_string(),
                    is_self: false,
                },
            ],
            starmap: HostedStarmapState {
                map_width: 18,
                map_height: 18,
                viewer_empire_id: 1,
                year: 3004,
                worlds: vec![HostedWorldState {
                    planet_index: 1,
                    coords: [8, 8],
                    intel_tier: "owned".to_string(),
                    known_name: Some("Sol".to_string()),
                    known_owner_empire_id: Some(1),
                    known_owner_empire_name: Some("Terran Union".to_string()),
                    known_potential_production: Some(100),
                    known_armies: Some(20),
                    known_ground_batteries: Some(5),
                    known_starbase_count: Some(1),
                    known_current_production: Some(40),
                    known_stored_points: Some(12),
                    known_docked_summary: None,
                    known_orbit_summary: None,
                }],
            },
            owned_planets: vec![HostedOwnedPlanet {
                planet_index: 1,
                name: "Sol".to_string(),
                coords: [8, 8],
                potential_production: 100,
                current_production: 40,
                stored_points: 12,
                armies: 20,
                ground_batteries: 5,
                starbase_count: 1,
                stardock: vec![HostedStardockSlot {
                    slot: 1,
                    kind: "destroyer".to_string(),
                    count: 2,
                }],
            }],
            owned_fleets: vec![HostedOwnedFleet {
                fleet_id: 1,
                local_slot: 1,
                coords: [8, 8],
                target_coords: [10, 10],
                order: "move".to_string(),
                order_summary: "Move fleet to Sector (10,10)".to_string(),
                rules_of_engagement: 4,
                current_speed: 5,
                max_speed: 6,
                ships: HostedFleetShips {
                    scout: 1,
                    battleship: 0,
                    cruiser: 2,
                    destroyer: 0,
                    transport: 0,
                    army: 0,
                    etac: 0,
                    total_starships: 3,
                    summary: "1 SC 2 CA".to_string(),
                },
            }],
        },
        queued_mail: vec![HostedQueuedMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3004,
            subject: "Scout".to_string(),
            body: "Hostiles near Rigel.".to_string(),
        }],
        report_blocks: vec![HostedReportBlock {
            viewer_empire_id: 1,
            block_index: 1,
            decoded_text: "Battle report".to_string(),
        }],
    }
}

fn temp_db_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("nc-client-{name}-{unique}.db"))
}

#[test]
fn hosted_snapshot_round_trips_through_local_store() {
    let path = temp_db_path("snapshot");
    let store = HostedStateStore::open(&path).expect("store");
    let snapshot = sample_snapshot();

    store
        .save_snapshot("hunter2", "player-001", &snapshot)
        .expect("save snapshot");

    let loaded = store
        .load_snapshot("hunter2", "player-001", "friday-night")
        .expect("load snapshot")
        .expect("cached snapshot");

    assert_eq!(loaded.game_id, "friday-night");
    assert_eq!(loaded.player_pubkey, "player-001");
    assert_eq!(loaded.turn, 4);
    assert_eq!(loaded.state_hash, "abc123");
    assert_eq!(loaded.snapshot, snapshot);

    assert!(
        store
            .load_snapshot("wrong-password", "player-001", "friday-night")
            .is_err()
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn hosted_draft_round_trips_and_clears() {
    let path = temp_db_path("draft");
    let store = HostedStateStore::open(&path).expect("store");
    let draft = TurnSubmission::parse_kdl_str(
        r#"
turn player=1 year=3004
tax rate=40
"#,
    )
    .expect("draft");

    store
        .save_draft(
            "hunter2",
            "player-001",
            "friday-night",
            "abc123",
            &draft,
            HostedDraftStatus::Local,
            None,
        )
        .expect("save draft");

    let loaded = store
        .load_draft("hunter2", "player-001", "friday-night")
        .expect("load draft")
        .expect("cached draft");

    assert_eq!(loaded.turn, 4);
    assert_eq!(loaded.base_hash, "abc123");
    assert_eq!(loaded.status, HostedDraftStatus::Local);
    assert_eq!(loaded.draft, draft);

    store
        .clear_draft("player-001", "friday-night")
        .expect("clear draft");
    assert!(
        store
            .load_draft("hunter2", "player-001", "friday-night")
            .expect("load cleared draft")
            .is_none()
    );

    let _ = std::fs::remove_file(path);
}
