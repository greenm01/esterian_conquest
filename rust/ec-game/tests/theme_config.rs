use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_game::screen::GameColor;
use ec_game::theme::classic;
use ec_game::theme::{
    AnsiMode, ansi_mode, bundled_theme_file_names, bundled_theme_kdl, initialize_from_game_dir,
    load_theme_from_path, toggle_ansi_mode,
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

    let theme_file = game_dir.join("themes").join("tokyo_night.kdl");
    assert!(
        theme_file.exists(),
        "themes/tokyo_night.kdl should be bootstrapped"
    );
    assert_eq!(
        fs::read_to_string(&theme_file).expect("read bootstrapped theme"),
        bundled_theme_kdl()
    );
    for name in bundled_theme_file_names() {
        assert!(
            game_dir.join("themes").join(name).exists(),
            "bootstrapped themes should include {name}"
        );
    }
}

#[test]
fn game_dir_uses_existing_theme_kdl_without_overwriting() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-existing");

    // Write a custom theme at the default location (just swap logo color)
    let custom = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"#7aa2f7\"",
        "style \"logo\" {\n    fg \"bright_cyan\"",
    );
    let themes_dir = game_dir.join("themes");
    fs::create_dir_all(&themes_dir).expect("create themes dir");
    fs::write(themes_dir.join("tokyo_night.kdl"), &custom).expect("write custom theme");

    initialize_from_game_dir(&game_dir, None).expect("initialize from game dir");
    assert_eq!(classic::logo_style().fg, GameColor::BrightCyan);

    // File must not be overwritten
    assert_eq!(
        fs::read_to_string(themes_dir.join("tokyo_night.kdl")).expect("read theme"),
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
        "style \"logo\" {\n    fg \"#7aa2f7\"",
        "style \"logo\" {\n    fg \"bright_magenta\"",
    );
    fs::write(game_dir.join("my-theme.kdl"), &custom).expect("write custom theme");

    // Caller (cli.rs) would have parsed config.kdl and resolved the theme path;
    // simulate that by passing the path directly.
    initialize_from_game_dir(&game_dir, Some(PathBuf::from("my-theme.kdl")))
        .expect("initialize from game dir");
    assert_eq!(classic::logo_style().fg, GameColor::BrightMagenta);
}

#[test]
fn config_kdl_absent_falls_back_to_themes_default() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-fallback");

    // No config.kdl; themes/tokyo_night.kdl has a custom color
    let custom = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"#7aa2f7\"",
        "style \"logo\" {\n    fg \"bright_green\"",
    );
    let themes_dir = game_dir.join("themes");
    fs::create_dir_all(&themes_dir).expect("create themes dir");
    fs::write(themes_dir.join("tokyo_night.kdl"), &custom).expect("write themes/tokyo_night.kdl");

    initialize_from_game_dir(&game_dir, None).expect("initialize from game dir");
    assert_eq!(classic::logo_style().fg, GameColor::BrightGreen);
}

// ─── Invalid theme falls back to bundled default ──────────────────────────────

#[test]
fn invalid_theme_falls_back_to_bundled_default() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-invalid");

    // First load a known custom color so we can see it get reset
    let custom = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"#7aa2f7\"",
        "style \"logo\" {\n    fg \"bright_cyan\"",
    );
    let custom_path = game_dir.join("custom.kdl");
    fs::write(&custom_path, custom).expect("write custom theme");
    load_theme_from_path(&custom_path).expect("load custom theme");
    assert_eq!(classic::logo_style().fg, GameColor::BrightCyan);

    // Now put an invalid themes/tokyo_night.kdl in the game dir
    let themes_dir = game_dir.join("themes");
    fs::create_dir_all(&themes_dir).expect("create themes dir");
    fs::write(themes_dir.join("tokyo_night.kdl"), "this is not valid kdl")
        .expect("write invalid theme");

    initialize_from_game_dir(&game_dir, None).expect("initialize with invalid theme");
    // Should silently fall back to bundled default (tokyo_night)
    assert_eq!(classic::logo_style().fg, GameColor::Rgb(122, 162, 247));
}

// ─── ANSI toggle ──────────────────────────────────────────────────────────────

#[test]
fn toggle_ansi_mode_is_session_only_and_projects_monochrome_theme() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-toggle");

    initialize_from_game_dir(&game_dir, None).expect("initialize theme");
    assert_eq!(ansi_mode(), AnsiMode::On);
    // tokyo_night palette (hex → GameColor::Rgb)
    assert_eq!(classic::logo_style().fg, GameColor::Rgb(122, 162, 247));
    assert_eq!(classic::notice_style().fg, GameColor::Rgb(247, 118, 142));
    assert_eq!(classic::body_style().fg, GameColor::Rgb(192, 202, 245));
    assert_eq!(classic::menu_style().fg, GameColor::Rgb(192, 202, 245));
    assert_eq!(classic::prompt_style().fg, GameColor::Rgb(192, 202, 245));
    assert_eq!(
        classic::status_label_style().fg,
        GameColor::Rgb(86, 95, 137)
    );
    assert_eq!(
        classic::table_header_style().fg,
        GameColor::Rgb(122, 162, 247)
    );
    assert_eq!(classic::table_chrome_style().fg, GameColor::Rgb(59, 66, 97));
    assert_eq!(
        classic::table_body_style().fg,
        GameColor::Rgb(169, 177, 214)
    );
    assert_eq!(
        classic::help_panel_style().fg,
        GameColor::Rgb(192, 202, 245)
    );
    assert_eq!(classic::quote_style().fg, GameColor::Rgb(86, 95, 137));
    assert_eq!(
        classic::disabled_row_style().fg,
        GameColor::Rgb(86, 95, 137)
    );
    assert_eq!(
        classic::indicator_off_style().fg,
        GameColor::Rgb(59, 66, 97)
    );

    let next_mode = toggle_ansi_mode().expect("toggle ansi mode off");
    assert_eq!(next_mode, AnsiMode::Off);
    assert_eq!(ansi_mode(), AnsiMode::Off);
    // Mono projection always yields named ANSI colors regardless of base theme
    assert_eq!(classic::body_style().fg, GameColor::White);
    assert_eq!(classic::menu_style().fg, GameColor::White);
    assert_eq!(classic::menu_hotkey_style().fg, GameColor::White);
    assert_eq!(classic::prompt_style().fg, GameColor::White);
    assert_eq!(classic::status_label_style().fg, GameColor::White);
    assert_eq!(classic::table_header_style().fg, GameColor::White);
    assert_eq!(classic::table_chrome_style().fg, GameColor::White);
    assert_eq!(classic::table_body_style().fg, GameColor::White);
    assert_eq!(classic::help_panel_style().fg, GameColor::White);
    assert_eq!(classic::quote_style().fg, GameColor::White);
    assert_eq!(classic::logo_style().fg, GameColor::White);
    assert_eq!(classic::notice_style().fg, GameColor::White);
    assert_eq!(classic::disabled_row_style().fg, GameColor::BrightBlack);
    assert_eq!(classic::indicator_off_style().fg, GameColor::BrightBlack);
    assert_eq!(classic::selected_row_style().fg, GameColor::Black);
    assert_eq!(classic::selected_row_style().bg, GameColor::BrightBlack);

    initialize_from_game_dir(&game_dir, None).expect("reinitialize resets ansi on");
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(classic::logo_style().fg, GameColor::Rgb(122, 162, 247));
}
