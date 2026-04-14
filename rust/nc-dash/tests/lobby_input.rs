use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use nc_dash::lobby::LobbyApp;
use nc_dash::lobby::hosted::dashboard::build_hosted_dash_app;
use nc_dash::lobby::models::{DirectContactRow, JoinedGameRow, OpenGameRow, ThreadMessage};
use nc_dash::lobby::state::{
    FirstRunField, HostedGameView, KeychainGateMode, LobbyRoute, LobbyTab,
};
use nc_dash::lobby::transport::LobbyLoadedState;
use nc_dash::lobby::update::apply_key;
use nc_nostr::state_sync::{
    GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
    HostedPlayerRosterEntry, HostedPlayerState, HostedQueuedMail, HostedReportBlock,
    HostedStardockSlot, HostedStarmapState, HostedStatePayload, HostedWorldState,
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

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
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

    app.state.route = LobbyRoute::Comms;
    app.state.active_tab = LobbyTab::Comms;
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: None,
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 0,
        last_activity_at: None,
    }];
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });
    app.state.compose_message_input.clear();
    app.set_clipboard_text("thread\r\nnote");

    apply_key(&mut app, ctrl_key(KeyCode::Char('v')));
    assert_eq!(app.state.compose_message_input, "threadnote");
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

    apply_key(&mut app, key(KeyCode::Down));
    apply_key(&mut app, key(KeyCode::Down));
    let initial = app.state.settings_draft.follow_mouse_on_map;
    apply_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.state.settings_draft.follow_mouse_on_map, !initial);
}

#[test]
fn settings_route_cycles_idle_lock_timeout() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

    apply_key(&mut app, shift_key(KeyCode::Char('s')));
    assert_eq!(app.state.route, LobbyRoute::Settings);
    assert_eq!(app.state.settings_selected, 0);

    apply_key(&mut app, key(KeyCode::Down));
    let initial = app.state.settings_draft.lock_timeout_minutes;
    apply_key(&mut app, key(KeyCode::Enter));

    assert_ne!(app.state.settings_draft.lock_timeout_minutes, initial);
}

#[test]
fn theme_picker_previews_and_accepts_theme_choice() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

    apply_key(&mut app, shift_key(KeyCode::Char('s')));
    apply_key(&mut app, key(KeyCode::Down));
    apply_key(&mut app, key(KeyCode::Down));
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
    let original_tab = app.state.active_tab;

    apply_key(&mut app, key(KeyCode::Char('?')));
    assert!(app.state.show_help);

    apply_key(&mut app, key(KeyCode::Down));
    assert_eq!(app.state.active_tab, original_tab);

    apply_key(&mut app, key(KeyCode::Enter));
    assert!(!app.state.show_help);

    apply_key(&mut app, key(KeyCode::Char('?')));
    assert!(app.state.show_help);

    apply_key(&mut app, key(KeyCode::Esc));
    assert!(!app.state.show_help);
}

#[test]
fn apply_loaded_uses_host_contact_as_games_host_and_default_contact() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let mut open = OpenGameRow::new(
        "friday-night",
        "Open",
        "Friday Night NC",
        "nc_sysop",
        "ws://127.0.0.1:8080",
        "daemon",
        "new_players",
        1,
        4,
        "2026-04-13",
        "Y3000 T0",
        "summary",
    );
    open.host_contact_npub = Some("npub1sysop".to_string());

    app.state.apply_loaded(LobbyLoadedState {
        relay_label: Some("relay: ws://127.0.0.1:8080".to_string()),
        player_handle: Some("niltempus".to_string()),
        joined_games: Vec::new(),
        open_games: vec![open],
        game_inbox: Vec::new(),
        notices: Vec::new(),
        direct_contacts: vec![DirectContactRow {
            npub: "npub1sysop".to_string(),
            label: "nc_sysop".to_string(),
            nip05: Some("nc_sysop@nostrian-conquest.com".to_string()),
            source: "host".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        }],
        thread_messages: Vec::new(),
        game_inbox_messages: Vec::new(),
        network_status: nc_dash::lobby::state::LobbyNetworkStatus::Synced,
        status_message: None,
        status_tone: nc_dash::lobby::state::LobbyStatusTone::Info,
    });

    assert_eq!(app.state.direct_contacts.len(), 1);
    assert_eq!(app.state.open_games[0].host, "nc_sysop");
    assert_eq!(app.state.direct_contacts[0].label, "nc_sysop");
    assert_eq!(app.state.direct_contacts[0].source, "host");
    assert_eq!(
        app.state.selected_direct_contact().map(|contact| contact.npub.as_str()),
        Some("npub1sysop")
    );
}

#[test]
fn apply_loaded_adds_host_contact_when_threads_list_starts_empty() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let mut open = OpenGameRow::new(
        "friday-night",
        "Open",
        "Friday Night NC",
        "nc_sysop",
        "ws://127.0.0.1:8080",
        "daemon",
        "new_players",
        1,
        4,
        "2026-04-13",
        "Y3000 T0",
        "summary",
    );
    open.host_contact_npub = Some("npub1sysop".to_string());

    app.state.apply_loaded(LobbyLoadedState {
        relay_label: Some("relay: ws://127.0.0.1:8080".to_string()),
        player_handle: Some("niltempus".to_string()),
        joined_games: Vec::new(),
        open_games: vec![open],
        game_inbox: Vec::new(),
        notices: Vec::new(),
        direct_contacts: Vec::new(),
        thread_messages: Vec::new(),
        game_inbox_messages: Vec::new(),
        network_status: nc_dash::lobby::state::LobbyNetworkStatus::Synced,
        status_message: None,
        status_tone: nc_dash::lobby::state::LobbyStatusTone::Info,
    });

    assert_eq!(app.state.direct_contacts.len(), 1);
    assert_eq!(app.state.direct_contacts[0].label, "nc_sysop");
    assert_eq!(
        app.state.selected_direct_contact().map(|contact| contact.npub.as_str()),
        Some("npub1sysop")
    );
}

#[test]
fn apply_loaded_updates_stale_host_contact_label() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let mut open = OpenGameRow::new(
        "friday-night",
        "Open",
        "Friday Night NC",
        "nc_sysop",
        "ws://127.0.0.1:8080",
        "daemon",
        "new_players",
        1,
        4,
        "2026-04-13",
        "Y3000 T0",
        "summary",
    );
    open.host_contact_npub = Some("npub1sysop".to_string());

    app.state.apply_loaded(LobbyLoadedState {
        relay_label: Some("relay: ws://127.0.0.1:8080".to_string()),
        player_handle: Some("niltempus".to_string()),
        joined_games: Vec::new(),
        open_games: vec![open],
        game_inbox: Vec::new(),
        notices: Vec::new(),
        direct_contacts: vec![DirectContactRow {
            npub: "npub1sysop".to_string(),
            label: "niltempus".to_string(),
            nip05: None,
            source: "host".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        }],
        thread_messages: Vec::new(),
        game_inbox_messages: Vec::new(),
        network_status: nc_dash::lobby::state::LobbyNetworkStatus::Connected,
        status_message: None,
        status_tone: nc_dash::lobby::state::LobbyStatusTone::Info,
    });

    assert_eq!(app.state.open_games[0].host, "nc_sysop");
    assert_eq!(app.state.direct_contacts[0].label, "nc_sysop");
}

#[test]
fn typing_in_chat_focus_appends_to_comms_draft() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: Some("nc_sysop@nostrian-conquest.com".to_string()),
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 0,
        last_activity_at: None,
    }];
    app.state.active_tab = LobbyTab::Comms;
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });

    apply_key(&mut app, key(KeyCode::Char('m')));

    assert_eq!(app.state.route, LobbyRoute::Home);
    assert_eq!(app.state.active_tab, LobbyTab::Comms);
    assert_eq!(app.state.compose_message_input, "m");
}

#[test]
fn hjkl_do_not_navigate_when_chat_is_focused() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: Some("nc_sysop@nostrian-conquest.com".to_string()),
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 0,
        last_activity_at: None,
    }];
    app.state.active_tab = LobbyTab::Comms;
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });
    app.state.thread_pane_focus = nc_dash::lobby::state::ThreadPaneFocus::Chat;

    apply_key(&mut app, key(KeyCode::Char('h')));

    assert_eq!(app.state.route, LobbyRoute::Home);
    assert_eq!(app.state.compose_message_input, "h");
}

#[test]
fn enter_on_thread_focus_activates_selected_buffer_before_popup() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: None,
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 2,
        last_activity_at: None,
    }];
    app.state.active_tab = LobbyTab::Comms;
    app.state.thread_pane_focus = nc_dash::lobby::state::ThreadPaneFocus::New;

    apply_key(&mut app, key(KeyCode::Enter));

    assert_eq!(app.state.route, LobbyRoute::Home);
    assert_eq!(
        app.state.thread_pane_focus,
        nc_dash::lobby::state::ThreadPaneFocus::Chat
    );
    assert_eq!(app.state.direct_contacts[0].unread_count, 0);
}

#[test]
fn enter_in_chat_focus_keeps_comms_open_when_no_draft() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.active_tab = LobbyTab::Comms;
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: None,
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 0,
        last_activity_at: None,
    }];
    app.state.thread_pane_focus = nc_dash::lobby::state::ThreadPaneFocus::Chat;
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });

    apply_key(&mut app, key(KeyCode::Enter));

    assert_eq!(app.state.route, LobbyRoute::Home);
}

#[test]
fn thread_contact_list_moves_selection_before_transcript_scroll() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.active_tab = LobbyTab::Comms;
    app.state.direct_contacts = vec![
        DirectContactRow {
            npub: "npub1host".to_string(),
            label: "nc_sysop".to_string(),
            nip05: None,
            source: "host".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        },
        DirectContactRow {
            npub: "npub1ally".to_string(),
            label: "ally".to_string(),
            nip05: None,
            source: "manual".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 4,
            last_activity_at: Some("2026-04-13T22:15:00Z".to_string()),
        },
    ];
    app.state.contact_selected = 0;
    app.state.contact_picker_selected = 0;
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1host".to_string(),
    });
    app.state.thread_pane_focus = nc_dash::lobby::state::ThreadPaneFocus::Threads;

    apply_key(&mut app, key(KeyCode::Up));

    assert_eq!(
        app.state.selected_direct_contact().map(|contact| contact.npub.as_str()),
        Some("npub1ally")
    );
}

#[test]
fn tab_cycles_comms_focus() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.active_tab = LobbyTab::Comms;
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: None,
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 0,
        last_activity_at: None,
    }];
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });

    apply_key(&mut app, key(KeyCode::Right));
    assert_eq!(
        app.state.thread_pane_focus,
        nc_dash::lobby::state::ThreadPaneFocus::New
    );

    apply_key(&mut app, key(KeyCode::Right));
    assert_eq!(
        app.state.thread_pane_focus,
        nc_dash::lobby::state::ThreadPaneFocus::Threads
    );
}

#[test]
fn thread_transcript_scrolls_when_chat_focus_is_active() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.active_tab = LobbyTab::Comms;
    app.state.thread_pane_focus = nc_dash::lobby::state::ThreadPaneFocus::Chat;
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: None,
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 0,
        last_activity_at: None,
    }];
    app.state.thread_messages = vec![
        ThreadMessage::incoming("npub1sysop", "sysop", "first"),
        ThreadMessage::incoming("npub1sysop", "sysop", "second"),
    ];
    app.state.joined_games = vec![JoinedGameRow::new(
        "friday-night",
        "joined",
        "Friday Night",
        "nc-host",
        "ws://127.0.0.1:8080",
        "daemon",
        Some(1),
        "y3004 t4",
    )];
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });

    apply_key(&mut app, key(KeyCode::Down));

    assert_eq!(app.state.thread_scroll, 1);
}

#[test]
fn address_book_blocks_selected_contact_locally() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.direct_contacts = vec![
        DirectContactRow {
            npub: "npub1sysop".to_string(),
            label: "nc_sysop".to_string(),
            nip05: None,
            source: "host".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 1,
            last_activity_at: Some("2026-04-13T22:15:00Z".to_string()),
        },
        DirectContactRow {
            npub: "npub1ally".to_string(),
            label: "ally".to_string(),
            nip05: None,
            source: "manual".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        },
    ];
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });
    app.state.route = LobbyRoute::ContactPicker;
    app.state.contact_picker_selected = 0;

    apply_key(&mut app, shift_key(KeyCode::Char('b')));

    assert!(app.state.direct_contacts[0].blocked);
    assert_eq!(app.state.direct_contacts[0].unread_count, 0);
}

#[test]
fn delete_hides_selected_thread_conversation_locally() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.active_tab = LobbyTab::Comms;
    app.state.direct_contacts = vec![
        DirectContactRow {
            npub: "npub1sysop".to_string(),
            label: "nc_sysop".to_string(),
            nip05: None,
            source: "host".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        },
        DirectContactRow {
            npub: "npub1ally".to_string(),
            label: "ally".to_string(),
            nip05: None,
            source: "manual".to_string(),
            blocked: false,
            hidden: false,
            unread_count: 0,
            last_activity_at: None,
        },
    ];
    app.state.set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
        contact_npub: "npub1sysop".to_string(),
    });
    app.state.thread_pane_focus = nc_dash::lobby::state::ThreadPaneFocus::Threads;

    apply_key(&mut app, key(KeyCode::Delete));

    assert!(app.state.direct_contacts.iter().any(|contact| contact.hidden));
    assert_eq!(
        app.state.selected_direct_contact().map(|contact| contact.npub.as_str()),
        Some("npub1sysop")
    );
}

#[test]
fn matrix_lock_key_starts_unlock_prompt() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::MatrixLocked, ScreenGeometry::new(120, 40));

    apply_key(&mut app, key(KeyCode::Char('x')));

    assert_eq!(app.state.route, LobbyRoute::Locked);
}

#[test]
fn locked_resume_escape_returns_to_matrix_lock() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Locked, ScreenGeometry::new(120, 40));
    app.state.gate_mode = KeychainGateMode::ResumeSession;
    app.state.unlock_password_input = "secret".to_string();
    app.state.status_message = Some("bad password".to_string());

    apply_key(&mut app, key(KeyCode::Esc));

    assert_eq!(app.state.route, LobbyRoute::MatrixLocked);
    assert!(app.state.unlock_password_input.is_empty());
    assert!(app.state.status_message.is_none());
}

#[test]
fn resume_sync_overlay_blocks_home_input_until_dismissed() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.show_resume_sync_overlay = true;
    app.state.active_tab = LobbyTab::OpenGames;

    app.dispatch_key_event_for_test(key(KeyCode::Tab));

    assert!(app.state.show_resume_sync_overlay);
    assert_eq!(app.state.active_tab, LobbyTab::OpenGames);

    app.dispatch_key_event_for_test(key(KeyCode::Enter));

    assert!(!app.state.show_resume_sync_overlay);
    assert_eq!(app.state.active_tab, LobbyTab::OpenGames);
}

#[test]
fn handle_moves_to_settings_and_returns_there_after_cancel() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.player_handle = Some("Current".to_string());

    apply_key(&mut app, shift_key(KeyCode::Char('h')));
    assert_eq!(app.state.route, LobbyRoute::Home);

    apply_key(&mut app, shift_key(KeyCode::Char('s')));
    assert_eq!(app.state.route, LobbyRoute::Settings);

    apply_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.state.route, LobbyRoute::EditHandle);

    apply_key(&mut app, key(KeyCode::Esc));

    assert_eq!(app.state.route, LobbyRoute::Settings);
}

#[test]
fn clicking_home_rows_focuses_pane_and_selects_clicked_row() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.joined_games = vec![
        JoinedGameRow::new(
            "friday-night",
            "joined",
            "Friday Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(1),
            "y3004 t4",
        ),
        JoinedGameRow::new(
            "saturday-night",
            "joined",
            "Saturday Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            Some(2),
            "y3005 t2",
        ),
    ];

    app.state.active_tab = LobbyTab::MyGames;
    let buffer = app.render_for_test().expect("render lobby");
    let (title_row, title_col) = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .find(" MY ACTIVE GAMES ")
                .map(|col| (row, col))
        })
        .expect("joined panel");

    app.dispatch_mouse_event_for_test(mouse(
        MouseEventKind::Down(MouseButton::Left),
        (title_col + 2) as u16,
        (title_row + 4) as u16,
    ));

    assert_eq!(app.state.active_tab, LobbyTab::MyGames);
    assert_eq!(app.state.joined_selected, 1);
}

#[test]
fn enter_on_requested_or_rejected_row_does_not_open_hosted_game() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.active_tab = LobbyTab::MyGames;
    app.state.joined_games = vec![
        JoinedGameRow::new(
            "friday-night",
            "requested",
            "Friday Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            None,
            "- -",
        ),
        JoinedGameRow::new(
            "saturday-night",
            "rejected",
            "Saturday Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            None,
            "- -",
        ),
    ];

    apply_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.state.route, LobbyRoute::Home);
    assert_eq!(
        app.state.status_message.as_deref(),
        Some("Join request is still waiting for nc-host approval.")
    );

    app.state.joined_selected = 1;
    apply_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.state.route, LobbyRoute::Home);
    assert_eq!(
        app.state.status_message.as_deref(),
        Some("Join request was rejected. Select the game in Games to request again.")
    );
}

#[test]
fn clicking_pane_border_focuses_without_changing_selection() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.open_games = vec![
        OpenGameRow::new(
            "friday-night",
            "Open",
            "Friday Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            "new_players",
            3,
            4,
            "2026-04-13",
            "y3004 t4",
            "summary",
        ),
        OpenGameRow::new(
            "saturday-night",
            "Live",
            "Saturday Night",
            "nc-host",
            "ws://127.0.0.1:8080",
            "daemon",
            "new_players",
            2,
            9,
            "2026-04-14",
            "y3005 t2",
            "summary",
        ),
    ];
    app.state.active_tab = LobbyTab::MyGames;
    app.state.open_selected = 1;

    app.state.active_tab = LobbyTab::OpenGames;
    let buffer = app.render_for_test().expect("render lobby");
    let (header_row, header_col) = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .find(" OPEN GAMES AVAILABLE TO JOIN ")
                .map(|col| (row, col))
        })
        .expect("open games header");

    app.dispatch_mouse_event_for_test(mouse(
        MouseEventKind::Down(MouseButton::Left),
        header_col as u16,
        header_row as u16,
    ));

    assert_eq!(app.state.active_tab, LobbyTab::OpenGames);
    assert_eq!(app.state.open_selected, 1);
}

#[test]
fn settings_popup_drags_from_title_row() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Settings, ScreenGeometry::new(120, 40));

    let buffer = app.render_for_test().expect("render settings");
    let row = (0..buffer.height())
        .find(|&idx| buffer.plain_line(idx).contains(" LOBBY SETTINGS "))
        .expect("settings title row");
    let column = buffer
        .plain_line(row)
        .find("LOBBY SETTINGS")
        .expect("settings title") as u16;

    app.dispatch_mouse_event_for_test(mouse(MouseEventKind::Down(MouseButton::Left), column, row as u16));
    app.dispatch_mouse_event_for_test(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        column.saturating_add(8),
        row as u16 + 3,
    ));
    app.dispatch_mouse_event_for_test(mouse(
        MouseEventKind::Up(MouseButton::Left),
        column.saturating_add(8),
        row as u16 + 3,
    ));

    assert!(app.popup_position.is_some());
}

#[test]
fn settings_popup_does_not_drag_from_side_border() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Settings, ScreenGeometry::new(120, 40));

    let buffer = app.render_for_test().expect("render settings");
    let title_row = (0..buffer.height())
        .find(|&idx| buffer.plain_line(idx).contains(" LOBBY SETTINGS "))
        .expect("settings title row");
    let left_border = buffer
        .plain_line(title_row)
        .find('┌')
        .expect("left border") as u16;

    app.dispatch_mouse_event_for_test(mouse(
        MouseEventKind::Down(MouseButton::Left),
        left_border,
        title_row as u16 + 2,
    ));
    app.dispatch_mouse_event_for_test(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        left_border.saturating_add(8),
        title_row as u16 + 5,
    ));

    assert!(app.popup_position.is_none());
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
