use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nc_dash::lobby::LobbyApp;
use nc_dash::lobby::hosted::dashboard::build_hosted_dash_app;
use nc_dash::lobby::models::JoinedGameRow;
use nc_dash::lobby::state::{FirstRunField, HostedGameView, LobbyRoute};
use nc_dash::lobby::update::apply_key;
use nc_nostr::state_sync::{
    GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
    HostedPlayerState, HostedQueuedMail, HostedReportBlock, HostedStardockSlot, HostedStarmapState,
    HostedStatePayload, HostedWorldState,
};
use nc_ui::ScreenGeometry;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn shift_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

#[test]
fn enter_advances_first_run_fields_before_submit() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(120, 40));

    apply_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.state.first_run_field, FirstRunField::Password);
    assert_eq!(app.state.route, LobbyRoute::FirstRun);

    apply_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.state.first_run_field, FirstRunField::Confirm);
    assert_eq!(app.state.route, LobbyRoute::FirstRun);
}

#[test]
fn first_run_uses_up_down_and_ignores_tab_navigation() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(120, 40));

    apply_key(&mut app, key(KeyCode::Down));
    assert_eq!(app.state.first_run_field, FirstRunField::Password);

    apply_key(&mut app, key(KeyCode::Up));
    assert_eq!(app.state.first_run_field, FirstRunField::Handle);

    apply_key(&mut app, key(KeyCode::Tab));
    assert_eq!(app.state.first_run_field, FirstRunField::Handle);

    apply_key(&mut app, key(KeyCode::BackTab));
    assert_eq!(app.state.first_run_field, FirstRunField::Handle);
}

#[test]
fn paste_shortcuts_fill_single_line_lobby_fields() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(120, 40));
    app.set_clipboard_text("dash\r\npilot");

    apply_key(&mut app, ctrl_key(KeyCode::Char('v')));
    assert_eq!(app.state.first_run_handle_input, "dashpilot");

    app.state.route = LobbyRoute::EditHandle;
    app.state.edit_handle_input.clear();
    app.set_clipboard_text("captain\nnova");

    apply_key(&mut app, shift_key(KeyCode::Insert));
    assert_eq!(app.state.edit_handle_input, "captainnova");

    app.state.route = LobbyRoute::ComposeInvite;
    app.state.compose_message_input.clear();
    app.set_clipboard_text("hello\r\nthere");

    apply_key(&mut app, ctrl_key(KeyCode::Char('v')));
    assert_eq!(app.state.compose_message_input, "hellothere");
}

#[test]
fn submit_turn_paste_preserves_newlines() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::SubmitTurn, ScreenGeometry::new(120, 40));
    let snapshot = sample_hosted_snapshot();
    let dashboard =
        build_hosted_dash_app(&snapshot, ScreenGeometry::new(120, 40)).expect("hosted dash app");
    app.state.hosted_game = Some(HostedGameView {
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
    });
    app.set_clipboard_text("fleet-order alpha\nplanet-build beta\r\n");

    apply_key(&mut app, ctrl_key(KeyCode::Char('v')));

    assert_eq!(
        app.state
            .hosted_game
            .as_ref()
            .expect("hosted view")
            .submit_input,
        "fleet-order alpha\nplanet-build beta\r\n"
    );
}

#[test]
fn settings_route_opens_from_home_and_toggles_values() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

    apply_key(&mut app, shift_key(KeyCode::Char('s')));
    assert_eq!(app.state.route, LobbyRoute::Settings);

    let initial = app.state.settings_draft.follow_mouse_on_map;
    apply_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.state.settings_draft.follow_mouse_on_map, !initial);
}

#[test]
fn theme_picker_previews_and_accepts_theme_choice() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

    apply_key(&mut app, shift_key(KeyCode::Char('s')));
    apply_key(&mut app, key(KeyCode::Down));
    apply_key(&mut app, key(KeyCode::Down));
    apply_key(&mut app, key(KeyCode::Enter));

    assert_eq!(app.state.route, LobbyRoute::ThemePicker);

    let original = app.state.settings_draft.theme_key.clone();
    apply_key(&mut app, key(KeyCode::Down));
    assert_ne!(app.state.settings_draft.theme_key, original);

    let preview = app.state.settings_draft.theme_key.clone();
    apply_key(&mut app, key(KeyCode::Enter));

    assert_eq!(app.state.route, LobbyRoute::Settings);
    assert_eq!(app.state.settings_draft.theme_key, preview);
}

#[test]
fn question_mark_toggles_help_and_suppresses_home_navigation() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let original_focus = app.state.focus;

    apply_key(&mut app, key(KeyCode::Char('?')));
    assert!(app.state.show_help);

    apply_key(&mut app, key(KeyCode::Down));
    assert_eq!(app.state.focus, original_focus);

    apply_key(&mut app, key(KeyCode::Enter));
    assert!(!app.state.show_help);

    apply_key(&mut app, key(KeyCode::Char('?')));
    assert!(app.state.show_help);

    apply_key(&mut app, key(KeyCode::Esc));
    assert!(!app.state.show_help);
}

fn sample_hosted_snapshot() -> GameState {
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
