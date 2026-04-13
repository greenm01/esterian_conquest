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
    assert!(lines.contains("COMMANDS"));
    assert!(lines.contains("OPEN GAMES"));
    assert!(lines.contains("NOTICES"));
    assert!(lines.contains("THREAD"));
    assert!(!lines.contains("COMMANDS <-"));
}

#[test]
fn home_route_places_commands_under_inbox_and_uses_network_hud() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.network_status = LobbyNetworkStatus::Synced;
    app.state.status_message = Some("Invite request sent.".to_string());

    let buffer = app.render_for_test().expect("render lobby");
    let inbox = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .find(" INBOX ")
                .map(|col| (row, col))
        })
        .expect("inbox title");
    let commands = (0..buffer.height())
        .find_map(|row| {
            buffer
                .plain_line(row)
                .find(" COMMANDS ")
                .map(|col| (row, col))
        })
        .expect("commands title");

    assert!(buffer.plain_line(1).contains("NETWORK: SYNCED"));
    assert!(buffer.plain_line(commands.0 + 1).contains("Tab cycle"));
    assert!(buffer.plain_line(commands.0 + 2).contains("Invite request sent."));
    assert_eq!(commands.1, inbox.1);
    assert!(commands.0 > inbox.0);
}

#[test]
fn home_route_help_popup_renders_as_overlay() {
    let mut app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));
    app.state.show_help = true;

    let lines = render_app_lines(app);

    assert!(lines.contains("LOBBY HELP"));
    assert!(lines.contains("Tab        : cycle focus across lobby panels"));
    assert!(lines.contains("? / Esc    : close this help popup"));
}

#[test]
fn settings_route_renders_theme_controls() {
    let lines = render_lines(LobbyRoute::Settings);

    assert!(lines.contains("LOBBY SETTINGS"));
    assert!(lines.contains("Mouse Follow"));
    assert!(lines.contains("Grid Dots"));
    assert!(lines.contains("Theme"));
    assert!(lines.contains("Tokyo Night"));
}

#[test]
fn theme_picker_route_renders_theme_list() {
    let lines = render_lines(LobbyRoute::ThemePicker);

    assert!(lines.contains("THEME PICKER"));
    assert!(lines.contains("Themes"));
    assert!(lines.contains("Preview"));
    assert!(lines.contains("Tokyo Night"));
    assert!(lines.contains("Rose Pine"));
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
