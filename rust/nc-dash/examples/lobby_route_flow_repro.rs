#[path = "support/repro_support.rs"]
mod repro_support;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_dash::{
    LobbyApp, ScreenGeometry,
    lobby::hosted::dashboard::build_hosted_dash_app,
    lobby::models::{JoinedGameRow, LobbyNotice, OpenGameRow},
    lobby::state::{HostedGameView, LobbyNetworkStatus, LobbyRoute, LobbyStatusTone},
    lobby::transport::LobbyLoadedState,
};
use nc_nostr::state_sync::{
    GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
    HostedPlayerRosterEntry, HostedPlayerState, HostedQueuedMail, HostedReportBlock,
    HostedStardockSlot, HostedStarmapState, HostedStatePayload, HostedWorldState,
};
use repro_support::{parse_args, print_usage, run_stateful_rendered_ui_repro};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

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
    }
}

fn hosted_game_view() -> HostedGameView {
    let snapshot = sample_snapshot();
    let dashboard =
        build_hosted_dash_app(&snapshot, ScreenGeometry::new(120, 40)).expect("hosted dash app");
    HostedGameView {
        row: JoinedGameRow::new(
            "friday-night",
            "joined",
            "Friday Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(1),
            "y3004 t4",
        ),
        snapshot,
        dashboard,
        submit_input: String::new(),
        submit_status: None,
    }
}

fn loaded_state() -> LobbyLoadedState {
    let mut open = OpenGameRow::new(
        "sandbox-fresh",
        "Open",
        "Sandbox Fresh",
        "nc-host",
        "ws://127.0.0.1:8080",
        "daemon",
        "new_players",
        3,
        4,
        "2026-04-15",
        "Y3004 T4",
        "fresh sandbox",
    );
    open.game_tier = "Sandbox".to_string();

    let mut joined = JoinedGameRow::new(
        "friday-night",
        "joined",
        "Friday Night",
        "nc-host",
        "ws://127.0.0.1:8080",
        "daemon",
        Some(1),
        "T4",
    );
    joined.game_tier = "League".to_string();
    joined.last_turn = Some(4);
    joined.last_hash = Some("abc123".to_string());

    LobbyLoadedState {
        relay_label: Some("relay: ws://127.0.0.1:8080".to_string()),
        player_handle: Some("Niltempus".to_string()),
        joined_games: vec![joined],
        open_games: vec![open],
        game_inbox: Vec::new(),
        notices: vec![LobbyNotice::new("nc-host", "Welcome back commander.")],
        direct_contacts: Vec::new(),
        thread_messages: Vec::new(),
        game_inbox_messages: Vec::new(),
        network_status: LobbyNetworkStatus::Synced,
        status_message: Some("Session ready".to_string()),
        status_tone: LobbyStatusTone::Success,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = match parse_args() {
        Ok(options) => options,
        Err(err) if err.to_string() == "help requested" => {
            print_usage("lobby_route_flow_repro");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let app = LobbyApp::new_for_tests(LobbyRoute::Locked, ScreenGeometry::new(120, 40));
    run_stateful_rendered_ui_repro(
        "lobby_route_flow_repro",
        options.backend,
        app,
        |app| app.render_ui_for_repro(),
        |app, step| match step {
            0 => {
                app.state.unlock_password_input.push_str("fooBar");
                Some("populate unlock field")
            }
            1 => {
                app.state.apply_loaded(loaded_state());
                app.state.route = LobbyRoute::Home;
                app.state.show_manual = false;
                Some("enter home")
            }
            2 => {
                app.dispatch_key_event_for_test(key(KeyCode::Tab));
                Some("home tab switch")
            }
            3 => {
                app.state.hosted_game = Some(hosted_game_view());
                app.state.route = LobbyRoute::HostedGame;
                Some("enter hosted route")
            }
            4 => {
                app.dispatch_key_event_for_test(key(KeyCode::Char('?')));
                Some("open hosted help")
            }
            5 => {
                app.dispatch_key_event_for_test(key(KeyCode::Esc));
                Some("close hosted help")
            }
            _ => None,
        },
    )
}
