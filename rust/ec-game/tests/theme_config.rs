use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_game::screen::AnsiColor;
use ec_game::theme::classic;
use ec_game::theme::{
    AnsiMode, ansi_mode, bundled_theme_kdl, initialize_from_game_dir, load_theme_from_path,
    toggle_ansi_mode,
};

static TEMP_THEME_SEQ: AtomicU64 = AtomicU64::new(0);
static THEME_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn theme_test_guard() -> MutexGuard<'static, ()> {
    THEME_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("theme test lock")
}

fn temp_game_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{label}-{}-{}-{}",
        std::process::id(),
        TEMP_THEME_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp game dir");
    dir
}

// ─── Bootstrap ────────────────────────────────────────────────────────────────

#[test]
fn game_dir_bootstraps_theme_kdl_when_absent() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-bootstrap");

    initialize_from_game_dir(&game_dir, None).expect("initialize from game dir");

    let theme_file = game_dir.join("theme.kdl");
    assert!(theme_file.exists(), "theme.kdl should be bootstrapped");
    assert_eq!(
        fs::read_to_string(&theme_file).expect("read bootstrapped theme"),
        bundled_theme_kdl()
    );
}

#[test]
fn game_dir_uses_existing_theme_kdl_without_overwriting() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-existing");

    // Write a custom theme (just swap logo color)
    let custom = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"bright_blue\"",
        "style \"logo\" {\n    fg \"bright_cyan\"",
    );
    fs::write(game_dir.join("theme.kdl"), &custom).expect("write custom theme");

    initialize_from_game_dir(&game_dir, None).expect("initialize from game dir");
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightCyan);

    // File must not be overwritten
    assert_eq!(
        fs::read_to_string(game_dir.join("theme.kdl")).expect("read theme"),
        custom
    );
}

// ─── config.kdl theme directive ───────────────────────────────────────────────

#[test]
fn config_kdl_theme_directive_is_followed() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-config-directive");

    // Write a custom theme file
    let custom = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"bright_blue\"",
        "style \"logo\" {\n    fg \"bright_magenta\"",
    );
    fs::write(game_dir.join("my-theme.kdl"), &custom).expect("write custom theme");

    // Caller (cli.rs) would have parsed config.kdl and resolved the theme path;
    // simulate that by passing the path directly.
    initialize_from_game_dir(&game_dir, Some(PathBuf::from("my-theme.kdl")))
        .expect("initialize from game dir");
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightMagenta);
}

#[test]
fn config_kdl_absent_falls_back_to_theme_kdl() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-fallback");

    // No config.kdl; theme.kdl has a custom color
    let custom = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"bright_blue\"",
        "style \"logo\" {\n    fg \"bright_green\"",
    );
    fs::write(game_dir.join("theme.kdl"), &custom).expect("write theme.kdl");

    initialize_from_game_dir(&game_dir, None).expect("initialize from game dir");
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightGreen);
}

// ─── Invalid theme falls back to bundled default ──────────────────────────────

#[test]
fn invalid_theme_falls_back_to_bundled_default() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-invalid");

    // First load a known custom color so we can see it get reset
    let custom = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"bright_blue\"",
        "style \"logo\" {\n    fg \"bright_cyan\"",
    );
    let custom_path = game_dir.join("custom.kdl");
    fs::write(&custom_path, custom).expect("write custom theme");
    load_theme_from_path(&custom_path).expect("load custom theme");
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightCyan);

    // Now put an invalid theme.kdl in the game dir
    fs::write(game_dir.join("theme.kdl"), "this is not valid kdl").expect("write invalid theme");

    initialize_from_game_dir(&game_dir, None).expect("initialize with invalid theme");
    // Should silently fall back to bundled default
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightBlue);
}

// ─── ANSI toggle ──────────────────────────────────────────────────────────────

#[test]
fn toggle_ansi_mode_is_session_only_and_projects_monochrome_theme() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-toggle");

    initialize_from_game_dir(&game_dir, None).expect("initialize theme");
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightBlue);
    assert_eq!(classic::notice_style().fg, AnsiColor::BrightRed);
    assert_eq!(classic::body_style().fg, AnsiColor::White);
    assert_eq!(classic::menu_style().fg, AnsiColor::White);
    assert_eq!(classic::prompt_style().fg, AnsiColor::White);
    assert_eq!(classic::status_label_style().fg, AnsiColor::White);
    assert_eq!(classic::table_header_style().fg, AnsiColor::Cyan);
    assert_eq!(classic::table_chrome_style().fg, AnsiColor::Blue);
    assert_eq!(classic::table_body_style().fg, AnsiColor::BrightWhite);
    assert_eq!(classic::help_panel_style().fg, AnsiColor::White);
    assert_eq!(classic::quote_style().fg, AnsiColor::White);
    assert_eq!(classic::disabled_row_style().fg, AnsiColor::BrightBlack);
    assert_eq!(classic::indicator_off_style().fg, AnsiColor::BrightBlack);

    let next_mode = toggle_ansi_mode().expect("toggle ansi mode off");
    assert_eq!(next_mode, AnsiMode::Off);
    assert_eq!(ansi_mode(), AnsiMode::Off);
    assert_eq!(classic::body_style().fg, AnsiColor::White);
    assert_eq!(classic::menu_style().fg, AnsiColor::White);
    assert_eq!(classic::menu_hotkey_style().fg, AnsiColor::White);
    assert_eq!(classic::prompt_style().fg, AnsiColor::White);
    assert_eq!(classic::status_label_style().fg, AnsiColor::White);
    assert_eq!(classic::table_header_style().fg, AnsiColor::White);
    assert_eq!(classic::table_chrome_style().fg, AnsiColor::White);
    assert_eq!(classic::table_body_style().fg, AnsiColor::White);
    assert_eq!(classic::help_panel_style().fg, AnsiColor::White);
    assert_eq!(classic::quote_style().fg, AnsiColor::White);
    assert_eq!(classic::logo_style().fg, AnsiColor::White);
    assert_eq!(classic::notice_style().fg, AnsiColor::White);
    assert_eq!(classic::disabled_row_style().fg, AnsiColor::BrightBlack);
    assert_eq!(classic::indicator_off_style().fg, AnsiColor::BrightBlack);
    assert_eq!(classic::selected_row_style().fg, AnsiColor::Black);
    assert_eq!(classic::selected_row_style().bg, AnsiColor::BrightBlack);

    initialize_from_game_dir(&game_dir, None).expect("reinitialize resets ansi on");
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightBlue);
}
