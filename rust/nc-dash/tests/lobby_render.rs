use nc_dash::lobby::state::LobbyRoute;
use nc_dash::lobby::LobbyApp;
use nc_ui::ScreenGeometry;

#[test]
fn home_route_renders_three_pane_shell_copy() {
    let app = LobbyApp::new_for_tests(LobbyRoute::Home, ScreenGeometry::new(120, 40));

    let buffer = app.render_for_test().expect("render lobby");
    let lines = (0..buffer.height())
        .map(|row| {
            buffer
                .row(row)
                .iter()
                .map(|cell| cell.ch)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(lines.contains("NOSTRIAN CONQUEST LOBBY"));
    assert!(lines.contains("JOINED GAMES"));
    assert!(lines.contains("OPEN GAMES"));
    assert!(lines.contains("NOTICES"));
    assert!(lines.contains("THREAD"));
}
