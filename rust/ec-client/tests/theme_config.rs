use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_client::screen::AnsiColor;
use ec_client::theme::classic;
use ec_client::theme::{
    AnsiMode, PlatformKind, ThemeEnv, ansi_mode, bundled_theme_kdl, ensure_theme_file_for,
    initialize_theme_for, load_theme_from_path, toggle_ansi_mode,
};

static TEMP_THEME_SEQ: AtomicU64 = AtomicU64::new(0);
static THEME_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn theme_test_guard() -> MutexGuard<'static, ()> {
    THEME_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("theme test lock")
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}-{}",
        std::process::id(),
        TEMP_THEME_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

#[test]
fn linux_theme_path_uses_xdg_config_home() {
    let _guard = theme_test_guard();
    let env = ThemeEnv {
        home: Some(PathBuf::from("/home/tester")),
        xdg_config_home: Some(PathBuf::from("/tmp/ec-theme-config")),
        appdata: None,
    };

    let theme_file =
        ec_client::theme::resolve_theme_file_for(PlatformKind::Unix, &env).expect("resolve path");
    assert_eq!(
        theme_file,
        PathBuf::from("/tmp/ec-theme-config/ec-rust/theme.kdl")
    );
}

#[test]
fn windows_theme_path_uses_appdata() {
    let _guard = theme_test_guard();
    let env = ThemeEnv {
        home: Some(PathBuf::from("C:\\Users\\tester")),
        xdg_config_home: None,
        appdata: Some(PathBuf::from("C:\\Users\\tester\\AppData\\Roaming")),
    };

    let theme_file = ec_client::theme::resolve_theme_file_for(PlatformKind::Windows, &env)
        .expect("resolve windows path");
    let normalized = theme_file.to_string_lossy().replace('\\', "/");
    assert_eq!(
        normalized,
        "C:/Users/tester/AppData/Roaming/ec-rust/theme.kdl"
    );
}

#[test]
fn ensure_theme_file_bootstraps_default_once() {
    let _guard = theme_test_guard();
    let root = temp_dir("ec-client-theme-bootstrap");
    let env = ThemeEnv {
        home: Some(root.clone()),
        xdg_config_home: Some(root.join(".config")),
        appdata: None,
    };

    let theme_file = ensure_theme_file_for(PlatformKind::Unix, &env).expect("bootstrap theme file");
    assert!(theme_file.exists());
    assert_eq!(
        fs::read_to_string(&theme_file).expect("read bootstrapped theme"),
        bundled_theme_kdl()
    );
}

#[test]
fn invalid_user_theme_falls_back_to_bundled_default() {
    let _guard = theme_test_guard();
    let root = temp_dir("ec-client-theme-invalid");

    let custom_theme = bundled_theme_kdl().replace(
        "style \"logo\" {\n    fg \"bright_blue\"",
        "style \"logo\" {\n    fg \"bright_cyan\"",
    );
    let custom_path = root.join("custom-theme.kdl");
    fs::create_dir_all(&root).expect("create temp root");
    fs::write(&custom_path, custom_theme).expect("write custom theme");
    load_theme_from_path(&custom_path).expect("load custom theme");
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightCyan);

    let theme_dir = root.join(".config");
    let env = ThemeEnv {
        home: Some(root.clone()),
        xdg_config_home: Some(theme_dir.clone()),
        appdata: None,
    };
    let theme_file = theme_dir.join("ec-rust").join("theme.kdl");
    fs::create_dir_all(theme_file.parent().expect("theme parent")).expect("create theme dir");
    fs::write(&theme_file, "this is not valid kdl").expect("write invalid theme");

    initialize_theme_for(PlatformKind::Unix, &env).expect("initialize with invalid override");
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightBlue);
}

#[test]
fn toggle_ansi_mode_is_session_only_and_projects_monochrome_theme() {
    let _guard = theme_test_guard();
    let root = temp_dir("ec-client-ui-toggle");
    let env = ThemeEnv {
        home: Some(root.clone()),
        xdg_config_home: Some(root.join(".config")),
        appdata: None,
    };

    initialize_theme_for(PlatformKind::Unix, &env).expect("initialize theme and ui");
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightBlue);
    assert_eq!(classic::notice_style().fg, AnsiColor::BrightRed);

    let next_mode = toggle_ansi_mode().expect("toggle ansi mode off");
    assert_eq!(next_mode, AnsiMode::Off);
    assert_eq!(ansi_mode(), AnsiMode::Off);
    assert_eq!(classic::body_style().fg, AnsiColor::BrightBlack);
    assert_eq!(classic::menu_hotkey_style().fg, AnsiColor::BrightBlack);
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightBlack);
    assert_eq!(classic::notice_style().fg, AnsiColor::BrightBlack);
    assert_eq!(classic::selected_row_style().fg, AnsiColor::Black);
    assert_eq!(classic::selected_row_style().bg, AnsiColor::BrightBlack);

    initialize_theme_for(PlatformKind::Unix, &env).expect("reinitialize resets ansi on");
    assert_eq!(ansi_mode(), AnsiMode::On);
    assert_eq!(classic::logo_style().fg, AnsiColor::BrightBlue);
}
