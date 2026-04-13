use nc_dash::lobby::LobbyApp;
use nc_dash::lobby::state::{LobbyNetworkStatus, LobbyRoute};
use nc_ui::ScreenGeometry;

fn render_lines(route: LobbyRoute) -> String {
    let app = LobbyApp::new_for_tests(route, ScreenGeometry::new(120, 40));
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

#[test]
fn home_route_renders_three_pane_shell_copy() {
    let lines = render_lines(LobbyRoute::Home);

    assert!(lines.contains("NOSTRIAN CONQUEST LOBBY"));
    assert!(lines.contains("NETWORK: NO RELAY"));
    assert!(lines.contains("JOINED GAMES"));
    assert!(lines.contains("INBOX"));
    assert!(lines.contains("? Help"));
    assert!(lines.contains("I<N>vite"));
    assert!(lines.contains("S>ettings"));
    assert!(lines.contains("OPEN GAMES"));
    assert!(lines.contains("NOTICES"));
    assert!(lines.contains("THREAD"));
    assert!(!lines.contains("COMMANDS <-"));
    assert!(!lines.contains("HANDLE:"));
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
fn locked_route_renders_logo_and_unlock_copy() {
    let lines = render_lines(LobbyRoute::Locked);

    assert!(lines.contains("____ ___  _   _  ___  _   _ _____ ____ _____"));
    assert!(lines.contains("Enter your keychain password."));
    assert!(lines.contains("Password"));
    assert!(!lines.contains("Keychain path"));
    assert!(!lines.contains("NOSTRIAN CONQUEST LOBBY"));
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
