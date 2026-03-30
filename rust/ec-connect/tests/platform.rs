use ec_connect::platform::{
    WINDOWS_CONSOLE_COLS, WINDOWS_CONSOLE_ROWS, centered_start, preferred_console_size,
};

#[test]
fn preferred_console_size_matches_windows_launch_target() {
    assert_eq!(
        preferred_console_size(),
        (WINDOWS_CONSOLE_COLS, WINDOWS_CONSOLE_ROWS)
    );
    assert_eq!(preferred_console_size(), (100, 30));
}

#[test]
fn centered_start_centers_items_that_fit() {
    assert_eq!(centered_start(0, 1920, 1000), 460);
    assert_eq!(centered_start(40, 1200, 800), 240);
}

#[test]
fn centered_start_clamps_oversized_items_to_container_origin() {
    assert_eq!(centered_start(0, 900, 1000), 0);
    assert_eq!(centered_start(40, 700, 800), 40);
}
