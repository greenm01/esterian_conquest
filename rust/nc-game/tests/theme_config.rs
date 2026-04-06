use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_game::screen::GameColor;
use nc_game::theme::classic;
use nc_game::theme::{
    AnsiMode, ansi_mode, apply_door_theme, bundled_theme_file_names, bundled_theme_kdl,
    current_theme_key, door_theme_key, initialize_from_game_dir, load_theme_from_path,
    toggle_ansi_mode,
};

static TEMP_THEME_SEQ: AtomicU64 = AtomicU64::new(0);
static THEME_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn theme_test_guard() -> MutexGuard<'static, ()> {
    THEME_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
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

fn replace_style_fg(theme_kdl: &str, style_name: &str, from: &str, to: &str) -> String {
    let style_header = format!("style \"{style_name}\" {{");
    let fg_line = format!("fg \"{from}\"");
    let replacement = format!("fg \"{to}\"");
    let Some(style_start) = theme_kdl.find(&style_header) else {
        return theme_kdl.to_string();
    };
    let Some(relative_fg_start) = theme_kdl[style_start..].find(&fg_line) else {
        return theme_kdl.to_string();
    };
    let fg_start = style_start + relative_fg_start;
    let fg_end = fg_start + fg_line.len();
    let mut updated = theme_kdl.to_string();
    updated.replace_range(fg_start..fg_end, &replacement);
    updated
}

// ─── Bundled discovery ────────────────────────────────────────────────────────

#[test]
fn game_dir_uses_bundled_themes_without_creating_theme_files() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-bootstrap");

    initialize_from_game_dir(&game_dir, None).expect("initialize from game dir");

    assert_eq!(classic::logo_style().fg, GameColor::Rgb(122, 162, 247));
    assert_eq!(classic::empire_slot_color(1), GameColor::Rgb(122, 162, 247));
    assert_eq!(
        classic::empire_slot_color(12),
        GameColor::Rgb(198, 120, 221)
    );
    assert!(
        fs::read_dir(&game_dir).expect("read dir").next().is_none(),
        "DB-only runtime should not create theme files"
    );
}

#[test]
fn discover_theme_entries_lists_bundled_themes_without_paths() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-existing");

    let entries = nc_game::theme::discover_theme_entries(&game_dir).expect("discover themes");
    let bundled = entries
        .into_iter()
        .filter(|entry| entry.kind == nc_game::theme::ThemeEntryKind::Theme)
        .collect::<Vec<_>>();
    assert_eq!(bundled.len(), bundled_theme_file_names().len());
    assert!(bundled.iter().all(|entry| entry.path.is_none()));
}

// ─── config.kdl theme directive ───────────────────────────────────────────────

#[test]
fn config_kdl_theme_directive_is_followed() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-config-directive");

    // Write a custom theme file
    let custom = replace_style_fg(bundled_theme_kdl(), "logo", "#7aa2f7", "bright_magenta");
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

    initialize_from_game_dir(&game_dir, None).expect("initialize from game dir");
    assert_eq!(classic::logo_style().fg, GameColor::Rgb(122, 162, 247));
}

// ─── Invalid theme falls back to bundled default ──────────────────────────────

#[test]
fn invalid_theme_falls_back_to_bundled_default() {
    let _guard = theme_test_guard();
    let game_dir = temp_game_dir("ec-theme-invalid");

    // First load a known custom color so we can see it get reset
    let custom = replace_style_fg(bundled_theme_kdl(), "logo", "#7aa2f7", "bright_cyan");
    let custom_path = game_dir.join("custom.kdl");
    fs::write(&custom_path, custom).expect("write custom theme");
    load_theme_from_path(&custom_path).expect("load custom theme");
    assert_eq!(classic::logo_style().fg, GameColor::BrightCyan);

    let bad_path = game_dir.join("bad-theme.kdl");
    fs::write(&bad_path, "this is not valid kdl").expect("write bad theme");

    initialize_from_game_dir(&game_dir, Some(PathBuf::from("bad-theme.kdl")))
        .expect("initialize with invalid theme");
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
    assert_eq!(
        classic::menu_featured_label_style().fg,
        GameColor::Rgb(125, 207, 255)
    );
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
    assert_eq!(classic::menu_featured_label_style().fg, GameColor::White);
    assert_eq!(classic::prompt_style().fg, GameColor::White);
    assert_eq!(classic::status_label_style().fg, GameColor::White);
    assert_eq!(classic::table_header_style().fg, GameColor::White);
    assert_eq!(classic::table_chrome_style().fg, GameColor::White);
    assert_eq!(classic::table_body_style().fg, GameColor::White);
    assert_eq!(classic::help_panel_style().fg, GameColor::White);
    assert_eq!(classic::quote_style().fg, GameColor::White);
    assert_eq!(classic::logo_style().fg, GameColor::White);
    assert_eq!(classic::notice_style().fg, GameColor::White);
    assert_eq!(classic::empire_slot_color(1), GameColor::BrightWhite);
    assert_eq!(classic::empire_slot_color(12), GameColor::BrightWhite);
    assert_eq!(classic::disabled_row_style().fg, GameColor::BrightBlack);
    assert_eq!(classic::indicator_off_style().fg, GameColor::BrightBlack);
    assert_eq!(classic::selected_row_style().fg, GameColor::Black);
    assert_eq!(classic::selected_row_style().bg, GameColor::BrightBlack);
    assert!(!classic::body_style().bold);
    assert!(!classic::menu_hotkey_style().bold);
    assert!(!classic::menu_featured_label_style().bold);
    assert!(!classic::prompt_hotkey_style().bold);
    assert!(!classic::notice_style().bold);
    assert!(!classic::logo_style().bold);
    assert!(!classic::selected_row_style().bold);

    initialize_from_game_dir(&game_dir, None).expect("reinitialize resets ansi on");
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(classic::logo_style().fg, GameColor::Rgb(122, 162, 247));
}

#[test]
fn apply_door_theme_forces_mag16_and_restores_it_after_ansi_toggle() {
    let _guard = theme_test_guard();

    apply_door_theme();
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(current_theme_key().as_deref(), Some(door_theme_key()));
    assert_eq!(classic::logo_style().fg, GameColor::BrightBlue);
    assert_eq!(classic::notice_style().fg, GameColor::BrightRed);
    assert_eq!(classic::empire_slot_color(1), GameColor::BrightBlue);
    assert_eq!(classic::empire_slot_color(6), GameColor::BrightCyan);
    assert_eq!(classic::empire_slot_color(12), GameColor::White);
    assert_eq!(classic::body_style().fg, GameColor::White);
    assert_eq!(classic::table_header_style().fg, GameColor::Cyan);
    assert_eq!(classic::selected_row_style().fg, GameColor::BrightWhite);
    assert_eq!(classic::selected_row_style().bg, GameColor::Cyan);

    let next_mode = toggle_ansi_mode().expect("toggle ansi mode off");
    assert_eq!(next_mode, AnsiMode::Off);
    assert_eq!(ansi_mode(), AnsiMode::Off);
    assert_eq!(current_theme_key().as_deref(), Some(door_theme_key()));
    assert_eq!(classic::body_style().fg, GameColor::White);
    assert_eq!(classic::logo_style().fg, GameColor::White);
    assert_eq!(classic::notice_style().fg, GameColor::White);
    assert_eq!(classic::selected_row_style().fg, GameColor::Black);
    assert_eq!(classic::selected_row_style().bg, GameColor::BrightBlack);
    assert!(!classic::body_style().bold);
    assert!(!classic::menu_hotkey_style().bold);
    assert!(!classic::prompt_hotkey_style().bold);
    assert!(!classic::notice_style().bold);
    assert!(!classic::logo_style().bold);
    assert!(!classic::selected_row_style().bold);

    let next_mode = toggle_ansi_mode().expect("toggle ansi mode on");
    assert_eq!(next_mode, AnsiMode::On);
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(current_theme_key().as_deref(), Some(door_theme_key()));
    assert_eq!(classic::logo_style().fg, GameColor::BrightBlue);
    assert_eq!(classic::notice_style().fg, GameColor::BrightRed);
    assert_eq!(classic::selected_row_style().fg, GameColor::BrightWhite);
    assert_eq!(classic::selected_row_style().bg, GameColor::Cyan);
}
