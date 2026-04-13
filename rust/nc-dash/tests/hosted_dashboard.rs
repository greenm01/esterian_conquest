use nc_dash::lobby::hosted::dashboard::build_hosted_dash_app;
use nc_nostr::state_sync::{
    GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
    HostedPlayerRosterEntry, HostedPlayerState, HostedQueuedMail, HostedReportBlock,
    HostedStardockSlot, HostedStarmapState, HostedStatePayload, HostedWorldState,
};
use nc_ui::ScreenGeometry;

#[test]
fn hosted_snapshot_builds_real_dash_app() {
    let snapshot = GameState {
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
                worlds: vec![
                    HostedWorldState {
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
                    },
                    HostedWorldState {
                        planet_index: 2,
                        coords: [10, 10],
                        intel_tier: "partial".to_string(),
                        known_name: Some("Rigel".to_string()),
                        known_owner_empire_id: Some(2),
                        known_owner_empire_name: Some("Rigel Empire".to_string()),
                        known_potential_production: Some(80),
                        known_armies: None,
                        known_ground_batteries: None,
                        known_starbase_count: None,
                        known_current_production: None,
                        known_stored_points: None,
                        known_docked_summary: None,
                        known_orbit_summary: Some("1 hostile fleet".to_string()),
                    },
                ],
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
    };

    let app =
        build_hosted_dash_app(&snapshot, ScreenGeometry::new(120, 40)).expect("hosted dash app");

    assert_eq!(app.game_data.conquest.game_year(), 3004);
    assert_eq!(app.player_record_index_1_based, 1);
    assert_eq!(app.game_data.player.records[0].tax_rate(), 33);
    assert_eq!(app.game_data.planets.records[0].planet_name(), "Sol");
    assert_eq!(
        app.game_data.fleets.records[0]
            .standing_order_kind()
            .as_str(),
        "move"
    );
    assert_eq!(app.queued_mail.len(), 1);
    assert_eq!(app.report_block_rows.len(), 1);
    assert_eq!(app.planet_intel_snapshots.len(), 2);
}
