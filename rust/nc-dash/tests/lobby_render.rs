use nc_dash::lobby::LobbyApp;
use nc_dash::lobby::models::{JoinedGameRow, OpenGameRow};
use nc_dash::lobby::state::{LobbyNetworkStatus, LobbyRoute};
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

fn render_app_buffer(app: LobbyApp) -> nc_ui::PlayfieldBuffer {
    app.render_for_test().expect("render lobby")
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
fn home_route_renders_three_pane_shell_copy() {
    let lines = render_lines_at(LobbyRoute::Home, 180, 40);

    assert!(lines.contains("NOSTRIAN CONQUEST LOBBY"));
    assert!(lines.contains("NETWORK: NO RELAY"));
    assert!(lines.contains("JOINED GAMES"));
    assert!(lines.contains("Status"));
    assert!(lines.contains("Seat"));
    assert!(lines.contains("Year"));
    assert!(lines.contains("INBOX"));
    assert!(lines.contains("? Help"));
    assert!(lines.contains("I<N>vite"));
    assert!(lines.contains("S>ettings"));
    assert!(lines.contains("GAMES"));
    assert!(lines.contains("Map"));
    assert!(lines.contains("Size"));
    assert!(lines.contains("Date"));
    assert!(lines.contains("Created"));
    assert!(lines.contains("Seats"));
    assert!(lines.contains("Turn"));
    assert!(lines.contains("NOTICES"));
    assert!(lines.contains("THREAD"));
    assert!(!lines.contains("COMMANDS <-"));
    assert!(!lines.contains("HANDLE:"));
}

#[test]
fn open_games_header_stacks_open_above_seats() {
    let buffer = render_app_buffer(LobbyApp::new_for_tests(
        LobbyRoute::Home,
        ScreenGeometry::new(180, 40),
    ));
    let open_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("Open"))
        .expect("open header row");
    let seats_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("Seats"))
        .expect("seats header row");

    assert_eq!(open_row + 1, seats_row);
}

#[test]
fn home_route_keeps_empty_messages_under_table_headers() {
    let lines = render_lines(LobbyRoute::Home);

    assert!(lines.contains("<no joined hosted games>"));
    assert!(lines.contains("<no hosted games>"));
}

#[test]
fn home_route_tables_split_year_and_turn_columns() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(180, 40));
    app.state.joined_games = vec![JoinedGameRow::new(
        "friday-night",
        "joined",
        "Friday Night",
        "nc-host",
        "ws://127.0.0.1:8080",
        "daemon",
        Some(1),
        "Y3004 T4",
    )];
    app.state.open_games = vec![OpenGameRow::new(
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

    let lines = render_app_lines(app);

    assert!(lines.contains("3004"));
    assert!(lines.contains("3005"));
    assert!(lines.contains("27x27"));
    assert!(lines.contains("2026-04-13"));
    assert!(lines.contains("Open"));
    assert!(!lines.contains("Y3004"));
    assert!(!lines.contains("y3005"));
    assert!(!lines.contains("T4"));
    assert!(!lines.contains("t2"));
}

#[test]
fn home_route_centers_footer_and_uses_toast_overlay() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.network_status = LobbyNetworkStatus::Synced;
    app.state.status_tone = nc_dash::lobby::state::LobbyStatusTone::Success;
    app.state.status_message = Some("Invite request sent.".to_string());

    let buffer = app.render_for_test().expect("render lobby");
    let footer_row = buffer.height() - 3;
    let footer = buffer.plain_line(footer_row);
    let footer_start = footer.find("? Help").expect("footer labels");
    let toast_row = (0..buffer.height())
        .find(|&row| buffer.plain_line(row).contains("Invite request sent."))
        .expect("toast row");

    assert!(buffer.plain_line(2).contains("NETWORK: SYNCED"));
    assert!(footer.contains("I<N>vite"));
    assert!(footer.contains("M>essage"));
    assert!(footer.contains("S>ettings"));
    assert!(footer_start > 0);
    assert!(toast_row < footer_row);
}

#[test]
fn home_route_footer_spans_width_below_columns() {
    let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    let buffer = app.render_for_test().expect("render lobby");
    let inbox = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .find(" INBOX ")
                .map(|col| (row, col))
        })
        .expect("inbox title");
    let footer_border = buffer.height() - 5;
    let footer_line = buffer.plain_line(footer_border);

    assert!(footer_border > inbox.0);
    assert_eq!(footer_line.chars().next(), Some('┌'));
    assert_eq!(footer_line.chars().last(), Some('┐'));
}

#[test]
fn home_route_help_popup_renders_as_overlay() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.show_help = true;

    let lines = render_app_lines(app);

    assert!(lines.contains("LOBBY HELP"));
    assert!(lines.contains("Tab        : cycle focus across lobby panels"));
    assert!(lines.contains("S          : open lobby settings, including local handle"));
    assert!(lines.contains("? / Esc    : close this help popup"));
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
    let screen_bg = buffer.row(10)[39].style.bg;
    let popup_bg = buffer.row(title_row + 1)[title_col].style.bg;

    assert_eq!(buffer.row(20)[39].style.bg, screen_bg);
    assert_eq!(
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
        .find_map(|idx| {
            buffer
                .plain_line(idx)
                .find(" LOBBY HELP ")
                .map(|col| (idx, col))
        })
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
    assert!(lines.contains("Mouse Follow"));
    assert!(lines.contains("Grid Dots"));
    assert!(lines.contains("Theme"));
    assert!(lines.contains("Tokyo Night"));
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
