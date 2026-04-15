use nc_dash::lobby::LobbyApp;
use nc_dash::lobby::models::{DirectContactRow, JoinedGameRow, OpenGameRow, ThreadMessage};
use nc_dash::lobby::onboarding::matrix_glyph;
use nc_dash::lobby::state::{LobbyNetworkStatus, LobbyRoute, LobbyTab};
use nc_ui::ScreenGeometry;

fn render_lines(route: LobbyRoute) -> String {
    let app = LobbyApp::new_for_tests(route, ScreenGeometry::new(120, 40));
    render_app_lines(app)
}

fn render_lines_at(route: LobbyRoute, width: usize, height: usize) -> String {
    let app = LobbyApp::new_for_tests(route, ScreenGeometry::new(width, height));
    render_app_lines(app)
}

fn render_app_lines(app: LobbyApp) -> String {
    let buffer = app.render_for_test().expect("render lobby");
    (0..buffer.height())
        .map(|row| {
            buffer
                .row(row)
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_first_char(buffer: &nc_ui::PlayfieldBuffer, ch: char) -> Option<(usize, usize)> {
    (0..buffer.height()).find_map(|row| {
        buffer
            .row(row)
            .iter()
            .position(|cell| cell.ch == ch)
            .map(|col| (row, col))
    })
}

#[test]
fn home_route_renders_tabbed_shell_copy() {
    let lines = render_lines_at(LobbyRoute::Home, 180, 40);

    assert!(lines.contains("NOSTRIAN CONQUEST LOBBY"));
    assert!(lines.contains("NETWORK: NO RELAY"));
    assert!(lines.contains("[ My Games ]"));
    assert!(lines.contains("[ Open Games ]"));
    assert!(lines.contains("[ Comms ]"));
    assert!(lines.contains("? Help"));
    assert!(lines.contains("J>oin"));
    assert!(lines.contains("Alt-Lock"));
    assert!(lines.contains("S>ettings"));
    assert!(lines.contains("OPEN GAMES AVAILABLE TO JOIN"));
    assert!(lines.contains("Status"));
    assert!(lines.contains("Game"));
    assert!(lines.contains("Host"));
    assert!(lines.contains("Seats"));
    assert!(!lines.contains("COMMANDS <-"));
    assert!(!lines.contains("HANDLE:"));
}

#[test]
fn home_route_keeps_empty_messages_under_table_headers() {
    let mut my_games = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    my_games.state.active_tab = LobbyTab::MyGames;
    assert!(render_app_lines(my_games).contains("<no games yet - press 'j' to join an open game>"));

    let mut open_games = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    open_games.state.active_tab = LobbyTab::OpenGames;
    assert!(
        render_app_lines(open_games).contains("<no open games - press 'h' to host a new game>")
    );
}

#[test]
fn home_route_tables_split_year_and_turn_columns() {
    let mut my_games = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(180, 40));
    my_games.state.active_tab = LobbyTab::MyGames;
    my_games.state.joined_games = vec![JoinedGameRow::new(
        "friday-night",
        "joined",
        "Friday Night",
        "nc-host",
        "ws://127.0.0.1:8080",
        "daemon",
        Some(1),
        "Y3004 T4",
    )];
    let my_lines = render_app_lines(my_games);

    assert!(my_lines.contains("Y3004:T4"));
    assert!(!my_lines.contains("Y3004 T4"));

    let mut open_games = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(180, 40));
    open_games.state.active_tab = LobbyTab::OpenGames;
    open_games.state.open_games = vec![OpenGameRow::new(
        "saturday-night",
        "Open",
        "Saturday Night",
        "nc-host",
        "ws://127.0.0.1:8080",
        "daemon",
        "new_players",
        3,
        9,
        "2026-04-13",
        "y3005 t2",
        "summary",
    )];

    let lines = render_app_lines(open_games);

    assert!(lines.contains("3005"));
    assert!(lines.contains("27x27"));
    assert!(lines.contains("2026-04-13"));
    assert!(lines.contains("Open"));
    assert!(!lines.contains("y3005"));
    assert!(!lines.contains("t2"));
}

#[test]
fn home_route_centers_footer_and_uses_toast_overlay() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.network_status = LobbyNetworkStatus::Synced;
    app.state.status_tone = nc_dash::lobby::state::LobbyStatusTone::Success;
    app.state.status_message = Some("Join request sent.".to_string());

    let buffer = app.render_for_test().expect("render lobby");
    let footer_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("? Help"))
        .expect("footer labels");
    let footer = buffer.plain_line(footer_row);
    let footer_start = footer.find("? Help").expect("footer labels");
    let toast_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("Join request sent."))
        .expect("toast row");

    assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("NETWORK: SYNCED")));
    assert!(footer.contains("J>oin"));
    assert!(footer.contains("Alt-Lock"));
    assert!(footer.contains("Tab Next Tab"));
    assert!(footer.contains("S>ettings"));
    assert!(footer_start > 0);
    assert!(toast_row < footer_row);
}

#[test]
fn home_route_footer_sits_inside_double_shell() {
    let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render lobby");
    let body = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .find(" OPEN GAMES AVAILABLE TO JOIN ")
                .map(|col| (row, col))
        })
        .expect("open games title");
    let footer_labels = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("? Help"))
        .expect("footer labels");
    let table_left = buffer
        .row(body.0)
        .iter()
        .position(|cell| cell.ch == '┌')
        .expect("table left border");
    let shell_border = footer_labels + 1;
    let shell_left = buffer
        .row(shell_border)
        .iter()
        .position(|cell| cell.ch == '╚')
        .expect("shell left border");
    let shell_right = buffer
        .row(shell_border)
        .iter()
        .rposition(|cell| cell.ch == '╝')
        .expect("shell right border");

    assert!(shell_border > body.0);
    assert!(table_left > shell_left);
    assert_eq!(buffer.row(footer_labels)[shell_left].ch, '║');
    assert_eq!(buffer.row(footer_labels)[shell_right].ch, '║');
}

#[test]
fn home_route_help_popup_renders_as_overlay() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.show_help = true;

    let lines = render_app_lines(app);

    assert!(lines.contains("HELP"));
    assert!(lines.contains("cycle dashboard tabs"));
    assert!(lines.contains("request to join the selected game"));
    assert!(lines.contains("compose a join request"));
    assert!(lines.contains("lock nc-dash"));
}

#[test]
fn comms_tab_help_popup_uses_comms_specific_commands() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.active_tab = LobbyTab::Comms;
    app.state.show_help = true;

    let lines = render_app_lines(app);

    assert!(lines.contains("cycle Chat / New / Threads"));
    assert!(lines.contains("open the address book"));
    assert!(lines.contains("hide the selected direct contact"));
    assert!(!lines.contains("compose a join request"));
    assert!(!lines.contains("request to join the selected game"));
}

#[test]
fn help_popup_wraps_to_dynamic_content_size() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.show_help = true;
    let buffer = app.render_for_test().expect("render help");
    let title_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains(" HELP "))
        .expect("help title row");
    let left = buffer
        .row(title_row)
        .iter()
        .position(|cell| cell.ch == '┌')
        .expect("help popup left border");
    let right = buffer
        .row(title_row)
        .iter()
        .rposition(|cell| cell.ch == '┐')
        .expect("help popup right border");
    let bottom_row = (title_row + 1..buffer.height())
        .find(|&row| buffer.row(row)[left].ch == '└')
        .expect("help popup bottom border");

    assert!(right - left + 1 < 72);
    assert!(bottom_row - title_row + 1 < 17);
}

#[test]
fn home_route_themes_screen_background_and_widget_chrome() {
    let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render lobby");
    let screen_bg = buffer.row(10)[39].style.bg;
    let chrome_bg = buffer.row(0)[0].style.bg;

    assert_eq!(buffer.row(20)[39].style.bg, screen_bg);
    assert_eq!(buffer.row(0)[0].style.bg, chrome_bg);
    assert_eq!(buffer.row(1)[1].style.bg, chrome_bg);
    assert_eq!(buffer.row(buffer.height() - 1)[0].style.bg, chrome_bg);
}

#[test]
fn settings_popup_themes_base_screen_and_popup_borders() {
    let app = LobbyApp::new_for_tests(LobbyRoute::Settings, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render settings");
    let (title_row, title_col) = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .find(" LOBBY SETTINGS ")
                .map(|col| (row, col))
        })
        .expect("settings popup");
    let popup_bg = buffer.row(title_row + 1)[title_col].style.bg;
    assert_ne!(
        buffer.row(title_row)[title_col.saturating_sub(2)].style.bg,
        popup_bg
    );
}

#[test]
fn help_popup_themes_popup_border_background() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.show_help = true;
    let buffer = app.render_for_test().expect("render help");
    let (row, col) = (0..buffer.height())
        .find_map(|idx| buffer.plain_line(idx).find(" HELP ").map(|col| (idx, col)))
        .expect("help popup");
    let popup_bg = buffer.row(row + 1)[col].style.bg;

    assert_eq!(buffer.row(row)[col.saturating_sub(2)].style.bg, popup_bg);
}

#[test]
fn home_route_panel_text_uses_panel_background() {
    let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render home");
    let row = (0..buffer.height())
        .find(|&idx| buffer.plain_line(idx).contains("NETWORK: NO RELAY"))
        .expect("header text row");
    let text_col = buffer
        .plain_line(row)
        .find("NETWORK: NO RELAY")
        .expect("header text col");

    assert_eq!(
        buffer.row(row)[text_col].style.bg,
        buffer.row(row)[1].style.bg
    );
}

#[test]
fn settings_route_renders_theme_controls() {
    let lines = render_lines(LobbyRoute::Settings);

    assert!(lines.contains("NOSTRIAN CONQUEST LOBBY"));
    assert!(lines.contains("? Help"));
    assert!(lines.contains("LOBBY SETTINGS"));
    assert!(lines.contains("Handle"));
    assert!(lines.contains("Idle Lock"));
    assert!(lines.contains("10 min"));
    assert!(lines.contains("Mouse Follow"));
    assert!(lines.contains("Grid Dots"));
    assert!(lines.contains("Theme"));
    assert!(lines.contains("Tokyo Night"));
}

#[test]
fn thread_panel_renders_irc_style_transcript_and_prompt() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(140, 40));
    app.state.active_tab = nc_dash::lobby::state::LobbyTab::Comms;
    app.state.player_handle = Some("niltempus".to_string());
    app.state.direct_contacts = vec![DirectContactRow {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: None,
        source: "host".to_string(),
        blocked: false,
        hidden: false,
        unread_count: 2,
        last_activity_at: Some("2026-04-13T22:15:00Z".to_string()),
    }];
    app.state.thread_messages = vec![
        ThreadMessage {
            message_id: "one".to_string(),
            contact_npub: "npub1sysop".to_string(),
            sender: "sysop".to_string(),
            body: "hello from the frontier".to_string(),
            outgoing: false,
            created_at: String::new(),
        },
        ThreadMessage {
            message_id: "two".to_string(),
            contact_npub: "npub1sysop".to_string(),
            sender: "niltempus".to_string(),
            body: "reply acknowledged".to_string(),
            outgoing: true,
            created_at: String::new(),
        },
    ];
    app.state
        .set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
            contact_npub: "npub1sysop".to_string(),
        });
    app.state.compose_message_input = "draft line".to_string();

    let lines = render_app_lines(app);

    assert!(lines.contains("NEW (2)"));
    assert!(lines.contains("THREADS"));
    assert!(lines.contains("BROADCAST"));
    assert!(lines.contains("DIRECT"));
    assert!(lines.contains("THREAD: nc_sysop"));
    assert!(lines.contains("[?] [ESC]"));
    assert!(lines.contains("sysop"));
    assert!(lines.contains("draft line"));
    assert!(!lines.contains("<niltempus>: draft line"));
    assert!(!lines.contains("<no game threads>"));
}

#[test]
fn comms_route_renders_full_screen_chat_scene() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(140, 40));
    app.state.active_tab = nc_dash::lobby::state::LobbyTab::Comms;
    app.state.player_handle = Some("niltempus".to_string());
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
    app.state
        .set_active_comms(nc_dash::lobby::models::CommsConversationKey::Direct {
            contact_npub: "npub1sysop".to_string(),
        });
    app.state.compose_message_input = "draft".to_string();

    let lines = render_app_lines(app);

    assert!(lines.contains("THREAD: nc_sysop"));
    assert!(lines.contains("NEW"));
    assert!(lines.contains("THREADS"));
    assert!(lines.contains("[?] [ESC]"));
    assert!(lines.contains("draft"));
    assert!(!lines.contains("<niltempus>: draft"));
}

#[test]
fn blocked_contacts_stay_visible_in_threads_pane_with_marker() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(140, 40));
    app.state.active_tab = LobbyTab::Comms;
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
            npub: "npub1spam".to_string(),
            label: "spam".to_string(),
            nip05: None,
            source: "manual".to_string(),
            blocked: true,
            hidden: false,
            unread_count: 9,
            last_activity_at: Some("2026-04-13T22:16:00Z".to_string()),
        },
    ];

    let lines = render_app_lines(app);

    assert!(lines.contains("nc_sysop"));
    assert!(lines.contains("spam"));
    assert!(lines.contains(" !"));
}

#[test]
fn hidden_contacts_stay_visible_in_threads_pane_with_marker() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(140, 40));
    app.state.active_tab = LobbyTab::Comms;
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
            npub: "npub1old".to_string(),
            label: "old-friend".to_string(),
            nip05: None,
            source: "manual".to_string(),
            blocked: false,
            hidden: true,
            unread_count: 0,
            last_activity_at: Some("2026-04-13T22:14:00Z".to_string()),
        },
    ];

    let lines = render_app_lines(app);

    assert!(lines.contains("nc_sysop"));
    assert!(lines.contains("old-friend"));
    assert!(lines.contains(" h"));
}

#[test]
fn resume_sync_overlay_renders_network_modal() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.show_resume_sync_overlay = true;
    app.state.network_status = LobbyNetworkStatus::Connecting;

    let lines = render_app_lines(app);

    assert!(lines.contains("NETWORK"));
    assert!(lines.contains("Network : Connecting"));
}

#[test]
fn theme_picker_route_renders_theme_list() {
    let lines = render_lines(LobbyRoute::ThemePicker);

    assert!(lines.contains("NOSTRIAN CONQUEST LOBBY"));
    assert!(lines.contains("? Help"));
    assert!(lines.contains("THEME PICKER"));
    assert!(lines.contains("Themes"));
    assert!(lines.contains("Preview"));
    assert!(lines.contains("Tokyo Night"));
    assert!(lines.contains("Current :"));
}

#[test]
fn first_run_route_renders_logo_and_handle_copy() {
    let lines = render_lines(LobbyRoute::FirstRun);

    assert!(lines.contains("____ ___  _   _  ___  _   _ _____ ____ _____"));
    assert!(lines.contains("Create your local hosted identity."));
    assert!(lines.contains("Choose a handle, set a password, and confirm it."));
    assert!(lines.contains("Handle"));
    assert!(lines.contains("Set Password"));
    assert!(lines.contains("Confirm"));
    assert!(!lines.contains("Keychain path"));
    assert!(!lines.contains("NOSTRIAN CONQUEST LOBBY"));
}

#[test]
fn first_run_route_themes_screen_and_gate_backgrounds() {
    let app = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render first run");
    let (top, left) = find_first_char(&buffer, '┌').expect("gate border");
    let screen_bg = buffer.row(0)[0].style.bg;
    let gate_bg = buffer.row(top + 1)[left + 1].style.bg;

    assert_eq!(buffer.row(buffer.height() - 1)[0].style.bg, screen_bg);
    assert_eq!(buffer.row(top)[left].style.bg, gate_bg);
    assert_eq!(buffer.row(top + 1)[left + 1].style.bg, gate_bg);
}

#[test]
fn locked_route_renders_logo_and_unlock_copy() {
    let lines = render_lines(LobbyRoute::Locked);

    assert!(lines.contains("____ ___  _   _  ___  _   _ _____ ____ _____"));
    assert!(lines.contains("Enter your keychain password."));
    assert!(lines.contains("Password"));
    assert!(!lines.contains("Keychain path"));
    assert!(!lines.contains("NOSTRIAN CONQUEST LOBBY"));
}

#[test]
fn locked_route_themes_screen_and_gate_backgrounds() {
    let app = LobbyApp::new_for_tests(LobbyRoute::Locked, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render locked");
    let (top, left) = find_first_char(&buffer, '┌').expect("gate border");
    let screen_bg = buffer.row(0)[0].style.bg;
    let gate_bg = buffer.row(top + 1)[left + 1].style.bg;

    assert_eq!(buffer.row(buffer.height() - 1)[0].style.bg, screen_bg);
    assert_eq!(buffer.row(top)[left].style.bg, gate_bg);
    assert_eq!(buffer.row(top + 1)[left + 1].style.bg, gate_bg);
}

#[test]
fn matrix_locked_route_uses_greek_glyph_stream() {
    let app = LobbyApp::new_for_tests(LobbyRoute::MatrixLocked, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render matrix lock");
    let glyph = (0..buffer.height())
        .flat_map(|row| {
            buffer
                .row(row)
                .iter()
                .map(|cell| cell.ch)
                .collect::<Vec<_>>()
        })
        .find(|ch| "ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ+#%*".contains(*ch))
        .expect("matrix glyph");

    assert!(
        "ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ+#%*".contains(glyph),
        "unexpected matrix glyph: {glyph:?}"
    );
}

#[test]
fn matrix_glyph_emits_greek_or_texture_symbols() {
    for x in 0..8 {
        for y in 0..8 {
            for frame in [0, 1, 9, 17] {
                let glyph = matrix_glyph(x, y, frame);
                assert!(
                    "ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ+#%*".contains(glyph),
                    "unexpected lock-screen glyph: {glyph:?}"
                );
            }
        }
    }
}

#[test]
fn first_run_status_wraps_inside_the_gate() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::FirstRun, ScreenGeometry::new(80, 30));
    app.state.status_message = Some(
        "This is a deliberately long onboarding error message that must stay inside the gate box."
            .to_string(),
    );

    let buffer = app.render_for_test().expect("render lobby");
    let top = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains(" FIRST RUN "))
        .expect("top border");
    let left = buffer
        .row(top)
        .iter()
        .position(|cell| cell.ch == '┌')
        .expect("left border");
    let right = buffer
        .row(top)
        .iter()
        .position(|cell| cell.ch == '┐')
        .expect("right border");
    let bottom = (top + 1..buffer.height())
        .find(|&row| buffer.row(row)[left].ch == '└' && buffer.row(row)[right].ch == '┘')
        .expect("bottom border");

    for row in top..=bottom {
        assert!(buffer.row(row)[..left].iter().all(|cell| cell.ch == ' '));
        assert!(
            buffer.row(row)[right + 1..]
                .iter()
                .all(|cell| cell.ch == ' ')
        );
    }
}
