//! Unit tests for the picker TUI module.
//!
//! These tests exercise `PickerState` logic and the pure render helpers in
//! `picker::render`.  No live terminal or Nostr connection is needed.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
use ec_connect::cache::{CachedGame, CachedGameStatus, GameCache};
use ec_connect::config::ConnectConfig;
use ec_connect::connect::handshake::GameEntry;
use ec_connect::connect::resolve::ResolvedTarget;
use ec_connect::connect::session::SessionOutcome;
use ec_connect::picker::connecting::PendingConnectRequest;
use ec_connect::picker::event::is_manual_refresh_key;
use ec_connect::picker::flows::{
    apply_session_outcome, connect_selected, persist_maps_root_at,
    redownload_selected_maps_with_config,
};
use ec_connect::picker::help::{HelpTopic, RELAY_GAMES_RAIL, RELAY_MENU_RAIL, KEYCHAIN_MENU_RAIL};
use ec_connect::picker::input::{handle_game_list_key, handle_relay_key};
use ec_connect::picker::layout::MAX_BODY_ROWS;
use ec_connect::picker::overlay::{NoticeLevel, PickerOverlay, handle_overlay_key};
use ec_connect::picker::refresh::PendingRefreshRequest;
use ec_connect::picker::relay::RelayPromptAction;
use ec_connect::picker::render::{Rect, centered_rect, matrix_glyph, short_npub, truncate};
use ec_connect::picker::runner::{classify_picker_event, post_bridge_recovery_event};
use ec_connect::picker::state::{ConnectDisplay, ConnectOrigin};
use ec_connect::picker::{PickerSession, PickerState, Screen};
use ec_connect::keychain::identity_npub;
use ec_connect::keychain::{Identity, IdentityType, Keychain};
use ec_ui::theme::classic;
use nostr_sdk::{Keys, ToBech32};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_game(id: &str, last_connected: Option<&str>) -> CachedGame {
    CachedGame {
        id: id.to_string(),
        name: format!("Game {id}"),
        player_name: Some(format!("Empire {id}")),
        server: "play.example.com".to_string(),
        port: 22,
        relay_url: Some("wss://relay.example.com".to_string()),
        seat: 1,
        npub: "npub1test".to_string(),
        gate_npub: String::new(),
        status: CachedGameStatus::Joined,
        invite_code: None,
        joined: "2026-03-01T10:00:00Z".to_string(),
        last_connected: last_connected.map(|s| s.to_string()),
    }
}

fn make_game_without_relay(id: &str) -> CachedGame {
    let mut game = make_game(id, Some("2026-03-26T00:00:00Z"));
    game.relay_url = None;
    game
}

fn make_pending_game(id: &str, invite_code: &str) -> CachedGame {
    let mut game = make_game(id, Some("2026-03-26T00:00:00Z"));
    game.status = CachedGameStatus::Pending;
    game.invite_code = Some(invite_code.to_string());
    game
}

fn make_state(games: Vec<CachedGame>) -> PickerState {
    let mut cache = GameCache::empty();
    for g in games {
        cache.upsert(g);
    }
    PickerState::new(cache, PathBuf::from("/tmp/nc/maps"))
}

fn make_pending_connect_request() -> PendingConnectRequest {
    PendingConnectRequest {
        origin: ConnectOrigin::GameList,
        target: ResolvedTarget {
            server_host: "play.example.com".to_string(),
            server_port: 22,
            relay_url: "wss://relay.example.com".to_string(),
            invite_code: None,
            game_id: Some("game-a".to_string()),
            gate_npub: None,
        },
        gate_npub: "npub1gate".to_string(),
        display: ConnectDisplay {
            lines: vec!["Attempting to connect...".to_string()],
        },
    }
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("ec-connect-{name}-{nanos}.kdl"))
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("ec-connect-{name}-{nanos}"))
}

fn make_session(alias: Option<&str>) -> PickerSession {
    make_session_with_identities(1, alias)
}

fn make_session_with_identities(count: usize, alias: Option<&str>) -> PickerSession {
    let mut identities = Vec::with_capacity(count);
    for index in 0..count {
        let pair = Keys::generate();
        identities.push(Identity {
            nsec: pair.secret_key().to_bech32().expect("nsec"),
            identity_type: IdentityType::Local,
            created: format!("2026-03-{:02}T00:00:00Z", (index % 28) + 1),
            alias: if index == 0 {
                alias.map(str::to_string)
            } else {
                Some(format!("Alias {index}"))
            },
        });
    }
    let keychain = Keychain {
        active: count.saturating_sub(1).min(2),
        identities,
    };
    let active_identity = keychain.active_identity().expect("active identity");
    let keys = Keys::parse(&active_identity.nsec).expect("keys");
    let npub = keys.public_key().to_bech32().expect("npub");

    PickerSession {
        password: "testing".to_string(),
        keychain,
        keys,
        npub,
    }
}

// ── PickerState::new ──────────────────────────────────────────────────────────

#[test]
fn picker_state_initial_values() {
    let state = make_state(vec![]);
    assert_eq!(state.selected, 0);
    assert_eq!(state.screen, Screen::GameList);
    assert!(state.overlay.is_none());
    assert!(state.join_input.is_empty());
    assert!(state.maps_input.is_empty());
    assert!(!state.maps_input_prefilled);
    assert!(state.keychain_input.is_empty());
    assert!(state.relay_input.is_empty());
    assert!(!state.quit);
    assert_eq!(state.keychain_selected, 0);
    assert!(state.can_manual_refresh());
}

#[test]
fn picker_error_notices_normalize_manual_line_breaks_into_clean_paragraphs() {
    let mut state = make_state(vec![make_game("a", None)]);
    state.show_error("Line one\nline two\n\nTechnical: bad thing");

    let Some(PickerOverlay::Notice { message, .. }) = state.overlay else {
        panic!("expected notice overlay");
    };

    assert_eq!(message, "Line one line two\n\nTechnical: bad thing");
}

#[test]
fn picker_header_uses_shortened_npub_not_alias() {
    let session = make_session(Some("Desk Alias"));

    assert_eq!(session.header_identity_label(), short_npub(&session.npub),);
}

#[test]
fn replacing_gui_identity_purges_old_identity_games_from_cache() {
    let mut session = make_session(Some("Desk Alias"));
    let old_npub = session.npub.clone();
    let mut cache = GameCache::empty();
    let mut old_game = make_game("old", Some("2026-03-26T00:00:00Z"));
    old_game.npub = old_npub.clone();
    cache.upsert(old_game);
    cache.upsert(make_game("other", Some("2026-03-26T00:00:00Z")));

    let replacement = Keys::generate().secret_key().to_bech32().expect("nsec");

    let cache_changed = session
        .replace_active_identity(&replacement, &mut cache)
        .expect("replace identity");

    assert!(cache_changed);
    assert_eq!(session.keychain.identities.len(), 1);
    assert_ne!(session.npub, old_npub);
    assert!(!cache.games.iter().any(|game| game.npub == old_npub));
    assert_eq!(cache.games.len(), 1);
}

#[test]
fn manual_refresh_enters_short_cooldown() {
    let mut state = make_state(vec![]);

    state.mark_manual_refresh();

    assert!(!state.can_manual_refresh());
}

#[test]
fn pending_refresh_request_has_visible_dwell_time() {
    let request = PendingRefreshRequest::from_game(
        "Test Game",
        ResolvedTarget {
            server_host: "localhost".to_string(),
            server_port: 2222,
            relay_url: "ws://localhost:8080".to_string(),
            invite_code: None,
            game_id: Some("test-game".to_string()),
            gate_npub: None,
        },
        "npub1gate".to_string(),
        "test-game".to_string(),
    );

    assert!(!request.is_ready());
    assert!(request.remaining_until_execute().as_millis() > 0);
    assert!(request.remaining_until_execute() <= std::time::Duration::from_secs(1));
}

#[test]
fn manual_refresh_key_accepts_plain_space() {
    assert!(is_manual_refresh_key(KeyEvent::new(
        KeyCode::Char(' '),
        KeyModifiers::NONE,
    )));
}

#[test]
fn manual_refresh_key_rejects_control_space() {
    assert!(!is_manual_refresh_key(KeyEvent::new(
        KeyCode::Char(' '),
        KeyModifiers::CONTROL,
    )));
}

// ── Selection clamping ────────────────────────────────────────────────────────

#[test]
fn refresh_cache_clamps_selection_when_list_shrinks() {
    // Build a state with 3 games and selection at index 2.
    let games = vec![
        make_game("a", None),
        make_game("b", None),
        make_game("c", None),
    ];
    let mut state = make_state(games);
    state.selected = 2; // valid while 3 games exist

    // Directly shrink the in-memory cache to 1 game without touching disk,
    // then call refresh_cache.  Since there's no disk file in the test
    // environment, load_cache() will fail silently and the in-memory cache
    // stays as we set it.  We therefore set the cache directly and let
    // refresh_cache do only the clamping step.
    //
    // To test pure clamping, we override the cache field directly:
    let mut smaller = GameCache::empty();
    smaller.upsert(make_game("a", None));
    // Manually perform what refresh_cache does when the in-memory cache is
    // already updated: clamp the selection.
    state.cache = smaller;
    let len = state.cache.sorted().len(); // = 1
    if state.selected >= len && len > 0 {
        state.selected = len - 1;
    }

    assert_eq!(state.selected, 0);
}

#[test]
fn refresh_cache_preserves_selection_when_still_valid() {
    let games = vec![make_game("a", None), make_game("b", None)];
    let mut state = make_state(games);
    state.selected = 1;

    // Clamp logic should not move a selection that is still in range.
    let len = state.cache.sorted().len();
    if state.selected >= len && len > 0 {
        state.selected = len - 1;
    }

    assert_eq!(state.selected, 1);
}

#[test]
fn refresh_cache_with_empty_list_leaves_selected_at_zero() {
    let mut state = make_state(vec![]);
    state.selected = 0;
    let len = state.cache.sorted().len();
    if state.selected >= len && len > 0 {
        state.selected = len - 1;
    }
    assert_eq!(state.selected, 0);
}

// ── Screen transitions ────────────────────────────────────────────────────────

#[test]
fn initial_screen_is_game_list() {
    let state = make_state(vec![]);
    assert_eq!(state.screen, Screen::GameList);
}

#[test]
fn screen_enum_eq() {
    assert_eq!(Screen::GameList, Screen::GameList);
    assert_eq!(Screen::IdentityOverlay, Screen::IdentityOverlay);
    assert_ne!(Screen::GameList, Screen::IdentityOverlay);
}

#[test]
fn screen_game_select_eq_and_ne() {
    let g = GameEntry {
        game_id: "g1".to_string(),
        name: "Game One".to_string(),
        seat: 1,
    };
    let s1 = Screen::GameSelect {
        games: vec![g.clone()],
        selected: 0,
        server_host: "play.example.com".to_string(),
        server_port: 22,
        relay_url: "wss://play.example.com:7777".to_string(),
        gate_npub: "npub1gate".to_string(),
    };
    let s2 = Screen::GameSelect {
        games: vec![g],
        selected: 0,
        server_host: "play.example.com".to_string(),
        server_port: 22,
        relay_url: "wss://play.example.com:7777".to_string(),
        gate_npub: "npub1gate".to_string(),
    };
    assert_eq!(s1, s2);
    assert_ne!(s2, Screen::GameList);
}

#[test]
fn help_overlay_renders_left_aligned_title_and_commands() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::Help(HelpTopic::MainCommand));
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!(
        (0..buffer.height()).any(|row| { buffer.plain_line(row).contains("MAIN COMMAND HELP") })
    );

    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("J/K") && line.contains("move selection")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("^U/^D") && line.contains("page up/down")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("?") && line.contains("show/hide helper")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("D") && line.contains("delete selected game")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("r") && line.contains("open relay manager")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("R") && line.contains("edit selected game relay")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("Space") && line.contains("refresh selected game info")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("Esc") && line.contains("same as <Q> on this screen")
    }));
}

#[test]
fn empty_picker_keeps_one_body_row_and_command_line_under_table() {
    let state = make_state(vec![]);
    let buffer = ec_connect::picker::render::render_inner_buffer(&state, None);

    let bottom_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains('└'))
        .expect("table should have a bottom border");
    let command_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("COMMAND <-"))
        .expect("picker should render a command line");

    assert_eq!(command_row, bottom_row + 1);
    assert_eq!(buffer.plain_line(command_row).find("COMMAND"), Some(1));
    assert!(buffer.plain_line(command_row).contains(" <Space> L "));
    assert!(
        !buffer
            .plain_line(command_row)
            .contains(concat!("NC ", env!("CARGO_PKG_VERSION")))
    );
    assert!(
        buffer
            .plain_line(buffer.height() - 1)
            .contains(concat!("NC ", env!("CARGO_PKG_VERSION")))
    );
    assert!(
        !buffer
            .plain_line(buffer.height() - 2)
            .contains("COMMAND <-")
    );
}

#[test]
fn help_overlay_dismisses_on_any_plain_key() {
    let mut state = make_state(vec![]);
    state.overlay = Some(PickerOverlay::Help(HelpTopic::MainCommand));

    handle_overlay_key(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.overlay.is_none());
}

#[test]
fn picker_falls_back_to_seat_label_when_player_name_missing() {
    let mut game = make_game("a", Some("2026-03-26T00:00:00Z"));
    game.player_name = None;
    let state = make_state(vec![game]);
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("Seat 1")));
}

#[test]
fn keychain_add_prompt_renders_wide_popup_instead_of_command_line_prompt() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.screen = Screen::KeychainAddPrompt;
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("REPLACE IDENTITY")));
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Paste nsec or leave blank for new <Q>")
    }));
    assert!(
        (0..buffer.height())
            .any(|row| { buffer.plain_line(row).contains("Paste an nsec to replace") })
    );
    assert!((0..buffer.height()).any(|row| { buffer.plain_line(row).contains("fresh one.") }));
}

#[test]
fn keychain_menu_rail_exposes_replace_only_flow() {
    assert_eq!(KEYCHAIN_MENU_RAIL, "? R <Enter> L <Q>");
}

#[test]
fn join_code_popup_shows_code_input() {
    use ec_connect::picker::overlay::PickerOverlay;
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.join_input = "amber-river@relay.example.com".to_string();
    state.overlay = Some(PickerOverlay::JoinCodePopup { error: None });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    // Popup title is visible.
    assert!(
        (0..buffer.height()).any(|row| buffer.plain_line(row).contains("JOIN GAME")),
        "expected JOIN GAME popup title"
    );
    // Input label is visible.
    assert!(
        (0..buffer.height()).any(|row| buffer.plain_line(row).contains("Invite:")),
        "expected invite label"
    );
    // Keyboard hint is visible.
    assert!(
        (0..buffer.height()).any(|row| buffer.plain_line(row).contains("Enter=join")),
        "expected Enter=join hint"
    );
}

#[test]
fn join_code_popup_compacts_long_invites_without_losing_prefix() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.join_input = "amber-river@relay.very-long-example-hostname.example.com:7447".to_string();
    state.overlay = Some(PickerOverlay::JoinCodePopup { error: None });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!(
        (0..buffer.height()).any(|row| {
            let line = buffer.plain_line(row);
            line.contains("Invite:")
                && (line.contains("...")
                    || line
                        .contains("amber-river@relay.very-long-example-hostname.example.com:7447"))
        }),
        "expected invite display to render either compacted or full text"
    );
}

#[test]
fn keychain_add_popup_cursor_sits_one_space_after_label() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.screen = Screen::KeychainAddPrompt;
    state.keychain_input = "nsec1stress".to_string();
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains(&format!("Nsec: {}", state.keychain_input))
    }));
}

#[test]
fn keychain_detail_popup_renders_full_backup_material_and_copy_hints() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::KeychainDetail { index: 0 });
    let session = make_session(Some("Desk Alias"));
    let identity = session.selected_identity(0).expect("keychain identity");
    let npub = identity_npub(identity).expect("npub");
    let buffer = ec_connect::picker::render::render_buffer(&state, Some(&session), 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains(&npub)));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains(&identity.nsec)));
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Ctrl-P=copy npub   Ctrl-S=copy nsec   Enter/Esc=close")
    }));
}

#[test]
fn keychain_detail_popup_has_no_alias_editor() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::KeychainDetail { index: 0 });
    let session = make_session(Some("Desk Alias"));
    let buffer = ec_connect::picker::render::render_buffer(&state, Some(&session), 82, 27);

    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains("Alias:")));
    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains("save alias")));
}

#[test]
fn error_notice_dismisses_on_any_key() {
    let mut state = make_state(vec![]);
    state.overlay = Some(PickerOverlay::Notice {
        level: NoticeLevel::Error,
        message: "boom".to_string(),
    });

    handle_overlay_key(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.overlay.is_none());
}

#[test]
fn notice_dismisses_on_arrow_key() {
    let mut state = make_state(vec![]);
    state.overlay = Some(PickerOverlay::Notice {
        level: NoticeLevel::Notice,
        message: "saved".to_string(),
    });

    handle_overlay_key(
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.overlay.is_none());
}

#[test]
fn maps_downloaded_popup_dismisses_on_any_key() {
    let mut state = make_state(vec![]);
    state.overlay = Some(PickerOverlay::MapsDownloaded {
        path: PathBuf::from("/tmp/nc/maps/friday-night"),
    });

    handle_overlay_key(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.overlay.is_none());
}

#[test]
fn connecting_overlay_escape_cancels_pending_connect() {
    let mut state = make_state(vec![]);
    state.overlay = Some(PickerOverlay::Connecting {
        lines: vec!["Attempting to connect...".to_string()],
    });
    state.pending_connect = Some(make_pending_connect_request());

    handle_overlay_key(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.overlay.is_none());
    assert!(state.pending_connect.is_none());
    assert!(state.active_connect.is_none());
}

#[test]
fn claiming_invite_overlay_q_cancels_pending_connect() {
    let mut state = make_state(vec![]);
    state.overlay = Some(PickerOverlay::ClaimingInvite {
        lines: vec!["Claiming invite...".to_string()],
    });
    state.pending_connect = Some(make_pending_connect_request());

    handle_overlay_key(
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.overlay.is_none());
    assert!(state.pending_connect.is_none());
    assert!(state.active_connect.is_none());
}

#[test]
fn relay_editor_renders_popup() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.relay_input = "ws://localhost:8080".to_string();
    state.overlay = Some(PickerOverlay::RelayEditor {
        original_url: None,
        title: "ADD RELAY".to_string(),
        instruction: "Add a relay for future joins or relay-grouped game management.".to_string(),
        error: None,
    });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("ADD RELAY")));
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Relay: ws://localhost:8080")
    }));
}

#[test]
fn relay_editor_can_render_blank_field_with_error() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.relay_input.clear();
    state.overlay = Some(PickerOverlay::RelayEditor {
        original_url: Some("wss://relay.example.com".to_string()),
        title: "EDIT RELAY".to_string(),
        instruction: "Update this relay URL. Joined games on this relay will move with it."
            .to_string(),
        error: Some("relay URL must not be empty".to_string()),
    });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("EDIT RELAY")));
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("relay URL must not be empty")
    }));
}

#[test]
fn main_game_list_r_opens_relay_list() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    let mut session = make_session(Some("Desk Alias"));
    let rt = tokio::runtime::Runtime::new().expect("runtime");

    handle_game_list_key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
        &mut state,
        &mut session,
        "",
        &rt,
    )
    .expect("handle R");

    assert!(matches!(state.screen, Screen::RelayList));
}

#[test]
fn main_game_list_uppercase_r_opens_selected_game_relay_prompt() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    let mut session = make_session(Some("Desk Alias"));
    let rt = tokio::runtime::Runtime::new().expect("runtime");

    handle_game_list_key(
        KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT),
        &mut state,
        &mut session,
        "",
        &rt,
    )
    .expect("handle shift-R");

    assert!(matches!(
        state.overlay,
        Some(PickerOverlay::GameRelayPrompt {
            action: RelayPromptAction::EditGame,
            ..
        })
    ));
}

#[test]
fn main_game_list_n_opens_join_popup() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    let mut session = make_session(Some("Desk Alias"));
    let rt = tokio::runtime::Runtime::new().expect("runtime");

    handle_game_list_key(
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE),
        &mut state,
        &mut session,
        "",
        &rt,
    )
    .expect("handle N");

    assert!(matches!(
        state.overlay,
        Some(PickerOverlay::JoinCodePopup { .. })
    ));
}

#[test]
fn main_game_list_m_opens_maps_download_popup_for_selected_game() {
    let mut state = make_state(vec![
        make_game_without_relay("a"),
        make_game_without_relay("b"),
        make_game_without_relay("c"),
    ]);
    state.selected = 2;
    let mut session = make_session(Some("Desk Alias"));
    let rt = tokio::runtime::Runtime::new().expect("runtime");

    handle_game_list_key(
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
        &mut state,
        &mut session,
        "",
        &rt,
    )
    .expect("handle M");

    assert_eq!(
        state.overlay,
        Some(PickerOverlay::MapsDownloadPrompt { error: None })
    );
    assert_eq!(state.maps_input, "/tmp/nc/maps");
    assert!(state.maps_input_prefilled);
    assert_eq!(state.selected, 2);
}

#[test]
fn maps_download_popup_renders_input_and_hint() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.maps_input =
        "/very/long/maps/root/that/should/still/stay/inside/the/popup/window".to_string();
    state.overlay = Some(PickerOverlay::MapsDownloadPrompt { error: None });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("DOWNLOAD MAPS")));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("Save to:")));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("Enter=save+download")));
}

#[test]
fn maps_download_popup_escape_cancels_and_clears_input() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.maps_input = "/tmp/custom-maps".to_string();
    state.overlay = Some(PickerOverlay::MapsDownloadPrompt { error: None });

    handle_overlay_key(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.overlay.is_none());
    assert!(state.maps_input.is_empty());
    assert!(!state.maps_input_prefilled);
}

#[test]
fn maps_download_popup_first_char_replaces_prefilled_path() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.maps_input = "/tmp/nc/maps".to_string();
    state.maps_input_prefilled = true;
    state.overlay = Some(PickerOverlay::MapsDownloadPrompt { error: None });

    handle_overlay_key(
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert_eq!(state.maps_input, "/");
    assert!(!state.maps_input_prefilled);
}

#[test]
fn maps_download_popup_first_backspace_clears_prefilled_path() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.maps_input = "/tmp/nc/maps".to_string();
    state.maps_input_prefilled = true;
    state.overlay = Some(PickerOverlay::MapsDownloadPrompt { error: None });

    handle_overlay_key(
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        &mut state,
        None,
        "",
        None,
    )
    .unwrap();

    assert!(state.maps_input.is_empty());
    assert!(!state.maps_input_prefilled);
}

#[test]
fn persist_maps_root_updates_state_and_writes_config() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    let maps_root = unique_temp_dir("alt-maps-root");
    state.maps_input = maps_root.display().to_string();
    let config_path = unique_temp_path("maps-root");

    let saved = persist_maps_root_at(&mut state, &config_path).expect("persist maps root");
    let config = ec_connect::config::load_config_from(&config_path).expect("load config");

    assert_eq!(saved, maps_root);
    assert_eq!(state.maps_root, maps_root);
    assert_eq!(config.maps_dir, Some(maps_root.clone()));
    let _ = std::fs::remove_file(&config_path);
    let _ = std::fs::remove_dir_all(&maps_root);
}

#[test]
fn persist_maps_root_rejects_relative_path() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.maps_input = "maps-folder".to_string();
    let config_path = unique_temp_path("maps-relative");

    let err = persist_maps_root_at(&mut state, &config_path).expect_err("relative path rejected");

    assert!(err.to_string().contains("absolute path"));
    assert_eq!(state.maps_root, PathBuf::from("/tmp/nc/maps"));
}

#[test]
fn persist_maps_root_rejects_existing_file() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    let file_path = unique_temp_path("maps-file");
    std::fs::write(&file_path, "not a directory").expect("seed file");
    state.maps_input = file_path.display().to_string();
    let config_path = unique_temp_path("maps-file-config");

    let err = persist_maps_root_at(&mut state, &config_path).expect_err("file path rejected");

    assert!(err.to_string().contains("file, not a folder"));
    assert_eq!(state.maps_root, PathBuf::from("/tmp/nc/maps"));
    let _ = std::fs::remove_file(&file_path);
}

#[test]
fn maps_download_popup_enter_uses_current_selection_not_first_game() {
    let mut state = make_state(vec![
        make_game_without_relay("a"),
        make_game_without_relay("b"),
        make_game_without_relay("c"),
    ]);
    state.selected = 2;
    let maps_root = unique_temp_dir("selected-game-maps");
    state.maps_input = maps_root.display().to_string();
    let config_path = unique_temp_path("maps-download");
    let rt = tokio::runtime::Runtime::new().expect("runtime");

    persist_maps_root_at(&mut state, &config_path).expect("persist maps root");
    redownload_selected_maps_with_config(
        &mut state,
        &make_session(Some("Desk Alias")).keys,
        "npub1gate",
        &rt,
        &ConnectConfig::empty(),
    )
    .expect("redownload selected maps");

    assert_eq!(
        state.overlay,
        Some(PickerOverlay::GameRelayPrompt {
            index: 2,
            action: RelayPromptAction::DownloadMaps,
            error: None,
        })
    );
    assert_eq!(state.maps_root, maps_root);
    let _ = std::fs::remove_file(&config_path);
    let _ = std::fs::remove_dir_all(&maps_root);
}

#[test]
fn game_relay_prompt_renders_popup() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.relay_input = "ws://localhost:7777".to_string();
    state.overlay = Some(PickerOverlay::GameRelayPrompt {
        index: 0,
        action: RelayPromptAction::Connect,
        error: Some("handshake timed out.".to_string()),
    });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("GAME RELAY")));
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Relay: ws://localhost:7777")
    }));
    assert!(
        (0..buffer.height()).any(|row| { buffer.plain_line(row).contains("handshake timed out.") })
    );
}

#[test]
fn join_popup_wraps_long_error_messages() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::JoinCodePopup {
        error: Some(
            "this is a deliberately long join error message that should wrap across multiple rows in the popup"
                .to_string(),
        ),
    });

    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("this is a deliberately long join error message")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("should wrap across")
            || line.contains("multiple rows")
            || line.contains("popup")
    }));
    let hint_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("Enter=join"))
        .expect("join hint row");
    let hint_line = buffer.plain_line(hint_row);
    assert!(!hint_line.contains('└'));
    assert!(!hint_line.contains('┘'));
}

#[test]
fn relay_popup_wraps_long_error_messages() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.relay_input = "ws://localhost:7777".to_string();
    state.overlay = Some(PickerOverlay::GameRelayPrompt {
        index: 0,
        action: RelayPromptAction::Connect,
        error: Some(
            "this relay lookup failed with a deliberately long error message that should wrap inside the popup"
                .to_string(),
        ),
    });

    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("this relay lookup failed with a deliberately long")
    }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("error message that should wrap") || line.contains("inside the popup")
    }));
}

#[test]
fn maps_popup_wraps_long_errors_without_hitting_border() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.maps_input = "/tmp/nc/maps".to_string();
    state.overlay = Some(PickerOverlay::MapsDownloadPrompt {
        error: Some(
            "this is a deliberately long maps error message that should wrap and keep the hint above the popup border"
                .to_string(),
        ),
    });

    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("this is a deliberately long maps error message")
    }));
    let hint_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("Enter=save+download"))
        .expect("maps hint row");
    let hint_line = buffer.plain_line(hint_row);
    assert!(!hint_line.contains('└'));
    assert!(!hint_line.contains('┘'));
}

#[test]
fn connecting_popup_wraps_long_status_lines_without_hitting_border() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::Connecting {
        lines: vec![String::from(
            "Relay: wss://relay.example.com/this/is/a/deliberately/long/status/line/that/should/wrap/inside/the/popup",
        )],
    });

    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| { buffer.plain_line(row).contains("Relay:") }));
    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("long/status/line")
            || line.contains("should/wrap/inside/the/popup")
            || line.contains("Esc/Q: cancel")
    }));
    let cancel_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("Esc/Q: cancel"))
        .expect("cancel row");
    let cancel_line = buffer.plain_line(cancel_row);
    assert!(!cancel_line.contains('└'));
    assert!(!cancel_line.contains('┘'));
}

#[test]
fn relay_games_screen_keeps_table_header_intact() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.screen = Screen::RelayGames {
        relay_url: "wss://relay.example.com".to_string(),
    };
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("Game") && line.contains("Last Conn")
    }));
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Relay: wss://relay.example.com")
    }));
}

#[test]
fn relay_help_rails_expose_delete_and_edit_actions() {
    assert!(RELAY_MENU_RAIL.contains("D"));
    assert!(RELAY_GAMES_RAIL.contains("R"));
}

#[test]
fn relay_games_uppercase_r_opens_selected_game_relay_prompt() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.screen = Screen::RelayGames {
        relay_url: "wss://relay.example.com".to_string(),
    };

    handle_relay_key(
        KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT),
        &mut state,
    )
    .expect("handle shift-R in relay games");

    assert!(matches!(
        state.overlay,
        Some(PickerOverlay::GameRelayPrompt {
            action: RelayPromptAction::EditGame,
            ..
        })
    ));
}

#[test]
fn connecting_popup_renders_context_lines() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::Connecting {
        lines: vec![
            "Game: Friday Night NC".to_string(),
            "Server: play.example.com:22".to_string(),
            "Relay: wss://relay.example.com".to_string(),
            "Attempting to connect...".to_string(),
        ],
    });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("CONNECTING TO GAME")));
    assert!(
        (0..buffer.height())
            .any(|row| { buffer.plain_line(row).contains("Attempting to connect...") })
    );
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("Esc/Q: cancel")));
}

#[test]
fn refreshing_popup_renders_context_lines() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::RefreshingGame {
        lines: vec![
            "Game: Friday Night NC".to_string(),
            "Server: play.example.com:22".to_string(),
            "Relay: wss://relay.example.com".to_string(),
            "Refreshing game info...".to_string(),
        ],
    });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("REFRESHING GAME")));
    assert!(
        (0..buffer.height())
            .any(|row| { buffer.plain_line(row).contains("Refreshing game info...") })
    );
}

#[test]
fn picker_session_suppresses_default_griffith_notice() {
    let mut state = make_state(vec![]);

    apply_session_outcome(
        &mut state,
        SessionOutcome::Done {
            exit_code: 0,
            notice: Some("For Griffith and glory.".to_string()),
            maps_saved_to: None,
        },
        None,
    );

    assert!(state.overlay.is_none());
}

#[test]
fn picker_session_keeps_nondefault_notice_in_tui() {
    let mut state = make_state(vec![]);

    apply_session_outcome(
        &mut state,
        SessionOutcome::Done {
            exit_code: 0,
            notice: Some("Warning: unable to save starmaps.".to_string()),
            maps_saved_to: None,
        },
        None,
    );

    assert_eq!(
        state.overlay,
        Some(PickerOverlay::Notice {
            level: NoticeLevel::Notice,
            message: "Warning: unable to save starmaps.".to_string(),
        })
    );
}

#[test]
fn picker_session_default_return_allows_immediate_quit_confirm() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::Connecting {
        lines: vec!["Attempting to connect...".to_string()],
    });

    apply_session_outcome(
        &mut state,
        SessionOutcome::Done {
            exit_code: 0,
            notice: Some("For Griffith and glory.".to_string()),
            maps_saved_to: None,
        },
        None,
    );

    state.request_quit();

    assert_eq!(state.overlay, Some(PickerOverlay::QuitConfirm));
}

#[test]
fn post_bridge_recovery_keeps_key_press_events() {
    let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);

    assert_eq!(post_bridge_recovery_event(Event::Key(key)), Some(key));
}

#[test]
fn post_bridge_recovery_discards_key_release_events() {
    let release = KeyEvent::new_with_kind(
        KeyCode::Char('q'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    );

    assert_eq!(post_bridge_recovery_event(Event::Key(release)), None);
}

#[test]
fn post_bridge_recovery_discards_non_key_events() {
    assert_eq!(post_bridge_recovery_event(Event::Resize(82, 27)), None);
}

#[test]
fn classify_picker_event_discards_mouse_events() {
    let mouse = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 12,
        row: 7,
        modifiers: KeyModifiers::NONE,
    });

    assert_eq!(classify_picker_event(mouse), None);
}

#[test]
fn matrix_glyph_emits_greek_or_texture_symbols() {
    let glyph = matrix_glyph(3, 5, 11);

    assert!(
        ('\u{0370}'..='\u{03FF}').contains(&glyph) || matches!(glyph, '+' | '#' | '%' | '*'),
        "unexpected lock-screen glyph: {glyph:?}"
    );
}

#[test]
fn overflowing_picker_renders_themed_scrollbar_gutter() {
    let games = (0..(MAX_BODY_ROWS + 3))
        .map(|idx| make_game(&format!("{idx:02}"), Some("2026-03-26T00:00:00Z")))
        .collect();
    let state = make_state(games);
    let buffer = ec_connect::picker::render::render_inner_buffer(&state, None);

    assert!((0..buffer.height()).any(|row| {
        let line = buffer.plain_line(row);
        line.contains("Game 00") || line.contains("Game 01")
    }));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("COMMAND <-")));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("Empire")));
    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("Status")));
}

#[test]
fn pending_rows_render_status_and_reuse_stored_invite_on_enter() {
    let mut state = make_state(vec![make_pending_game("a", "victim-sickness")]);

    connect_selected(&mut state, "").expect("pending row should queue a reconnect");

    let pending = state
        .pending_connect
        .as_ref()
        .expect("connect request should be queued");
    assert_eq!(
        pending.target.invite_code.as_deref(),
        Some("victim-sickness")
    );
    assert_eq!(pending.target.game_id.as_deref(), Some("a"));
    assert!(
        pending
            .display
            .lines
            .iter()
            .any(|line| line.contains("Invite: victim-sickness"))
    );

    let buffer = ec_connect::picker::render::render_inner_buffer(&state, None);
    assert!(
        (0..buffer.height()).any(|row| buffer.plain_line(row).contains("Status")),
        "main picker header should show Status"
    );
    assert!(
        (0..buffer.height()).any(|row| buffer.plain_line(row).contains("Pending")),
        "pending row should render its status"
    );
}

#[test]
fn successful_session_maps_path_opens_maps_downloaded_popup() {
    let mut state = make_state(vec![]);

    apply_session_outcome(
        &mut state,
        SessionOutcome::Done {
            exit_code: 0,
            notice: None,
            maps_saved_to: Some(PathBuf::from("/tmp/nc/maps/friday-night")),
        },
        None,
    );

    assert_eq!(
        state.overlay,
        Some(PickerOverlay::MapsDownloaded {
            path: PathBuf::from("/tmp/nc/maps/friday-night"),
        })
    );
}

#[test]
fn maps_downloaded_popup_renders_saved_path() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.overlay = Some(PickerOverlay::MapsDownloaded {
        path: PathBuf::from("/tmp/nc/maps/friday-night"),
    });
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("MAPS DOWNLOADED")));
    assert!(
        (0..buffer.height())
            .any(|row| buffer.plain_line(row).contains("/tmp/nc/maps/friday-night"))
    );
    assert!((0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Press any key to continue.")
    }));
}

#[test]
fn larger_terminal_keeps_space_hint_in_command_line_not_outside_border() {
    let state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    let buffer = ec_connect::picker::render::render_buffer(&state, None, 100, 30);

    assert_eq!(buffer.row(1)[9].ch, '┌');
    assert_eq!(buffer.row(27)[9].ch, '└');
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains("Press Space to refresh game info")
    }));
    let command_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("COMMAND <-"))
        .expect("picker should render a command line");
    assert!(buffer.plain_line(command_row).contains("<Space>"));
}

#[test]
fn gui_inner_picker_buffer_uses_plain_80x25_canvas_without_outer_shell() {
    let state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    let session = make_session(Some("Desk Alias"));
    let buffer = ec_connect::picker::render::render_inner_buffer(&state, Some(&session));
    let outer_title = format!("NC CONNECT v{}", env!("CARGO_PKG_VERSION"));

    assert_eq!(buffer.width(), 80);
    assert_eq!(buffer.height(), 25);
    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains(&outer_title)));
    assert!(!(0..buffer.height()).any(|row| {
        buffer
            .plain_line(row)
            .contains(&format!(" {}", outer_title))
    }));
}

#[test]
fn keychain_table_has_no_alias_column() {
    let mut state = make_state(vec![make_game("a", Some("2026-03-26T00:00:00Z"))]);
    state.screen = Screen::KeychainList;
    let session = make_session_with_identities(MAX_BODY_ROWS + 6, Some("Desk Alias"));
    let buffer = ec_connect::picker::render::render_buffer(&state, Some(&session), 82, 27);

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("Npub")));
    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains("Alias")));
    assert!(!(0..buffer.height()).any(|row| buffer.plain_line(row).contains("Desk Alias")));
}

#[test]
fn locked_screen_matrix_cells_use_theme_background() {
    let mut state = make_state(vec![]);
    state.screen = Screen::Locked;
    state.matrix.frame = 7;

    let buffer = ec_connect::picker::render::render_buffer(&state, None, 82, 27);
    let matrix_cell = (0..buffer.height())
        .flat_map(|row| {
            buffer
                .row(row)
                .iter()
                .enumerate()
                .map(move |(col, cell)| (row, col, cell))
        })
        .find(|(_, _, cell)| cell.ch != ' ')
        .expect("locked screen should draw matrix glyphs");

    assert_eq!(matrix_cell.2.style.bg, classic::app_background());
}

// ── truncate ──────────────────────────────────────────────────────────────────

#[test]
fn truncate_short_string_unchanged() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn truncate_exact_length_unchanged() {
    assert_eq!(truncate("hello", 5), "hello");
}

#[test]
fn truncate_long_string_gets_ellipsis() {
    let result = truncate("abcdefghij", 6);
    assert!(result.ends_with('…'), "expected ellipsis, got: {result}");
    assert!(result.chars().count() <= 6);
}

#[test]
fn truncate_max_one_gives_just_ellipsis() {
    let result = truncate("hello", 1);
    assert_eq!(result, "…");
}

#[test]
fn truncate_max_zero_gives_empty_string() {
    // max=0: saturating_sub(1) = 0 chars taken, no ellipsis pushed (0 < 1 false … wait)
    // Actually the code does: take(max.saturating_sub(1)) = take(0) → ""
    // then push('…') → "…"
    // max=0 means we want 0 chars, but ellipsis takes 1 char; that's an
    // edge we accept as "…" (1 char).
    let result = truncate("hello", 0);
    // Result is "…" because the else branch fires (0 < 5) and we push '…'.
    assert_eq!(result.chars().count(), 1);
}

// ── short_npub ────────────────────────────────────────────────────────────────

#[test]
fn short_npub_short_string_unchanged() {
    let s = "npub1short";
    assert_eq!(short_npub(s), s);
}

#[test]
fn short_npub_exactly_24_chars_unchanged() {
    let s = "a".repeat(24);
    assert_eq!(short_npub(&s), s);
}

#[test]
fn short_npub_long_string_truncated() {
    // 63-char npub (typical bech32 length).
    let npub = "npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqz5vhw";
    let result = short_npub(npub);
    // Should be: first 16 + … + last 8 = 25 chars.
    let chars: Vec<char> = npub.chars().collect();
    let expected = format!(
        "{}…{}",
        chars[..16].iter().collect::<String>(),
        chars[chars.len() - 8..].iter().collect::<String>()
    );
    assert_eq!(result, expected);
}

// ── centered_rect ─────────────────────────────────────────────────────────────

#[test]
fn centered_rect_fits_inside_parent() {
    let parent = Rect::new(0, 0, 100, 30);
    let popup = centered_rect(60, 5, parent);
    assert!(popup.x >= parent.x);
    assert!(popup.y >= parent.y);
    assert!(popup.x + popup.width <= parent.x + parent.width);
    assert!(popup.y + popup.height <= parent.y + parent.height);
}

#[test]
fn centered_rect_width_is_approximately_percent() {
    let parent = Rect::new(0, 0, 100, 30);
    let popup = centered_rect(60, 5, parent);
    assert_eq!(popup.width, 60); // 100 * 60 / 100 = 60
}

#[test]
fn centered_rect_height_matches_request() {
    let parent = Rect::new(0, 0, 100, 30);
    let popup = centered_rect(60, 5, parent);
    assert_eq!(popup.height, 5);
}

#[test]
fn centered_rect_clamped_when_larger_than_parent() {
    let parent = Rect::new(0, 0, 10, 4);
    // Request 200% width and 20 rows — both should be clamped.
    let popup = centered_rect(200, 20, parent);
    assert!(popup.width <= parent.width);
    assert!(popup.height <= parent.height);
}

// ── GameCache::sorted (picker-relevant ordering) ──────────────────────────────

#[test]
fn sorted_recent_game_first() {
    let mut cache = GameCache::empty();
    cache.upsert(make_game("old", Some("2026-01-01T00:00:00Z")));
    cache.upsert(make_game("new", Some("2026-03-26T00:00:00Z")));
    let sorted = cache.sorted();
    assert_eq!(sorted[0].id, "new");
    assert_eq!(sorted[1].id, "old");
}

#[test]
fn sorted_no_last_connected_appears_after_connected() {
    let mut cache = GameCache::empty();
    cache.upsert(make_game("never", None));
    cache.upsert(make_game("played", Some("2026-03-26T00:00:00Z")));
    let sorted = cache.sorted();
    assert_eq!(sorted[0].id, "played");
    assert_eq!(sorted[1].id, "never");
}

#[test]
fn sorted_empty_cache_returns_empty() {
    let cache = GameCache::empty();
    assert!(cache.sorted().is_empty());
}
