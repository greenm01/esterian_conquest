use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_client::app::{App, AppConfig};
use ec_client::screen::{MainMenuScreen, ScreenId, PLAYFIELD_HEIGHT};
use ec_compat::import_directory_snapshot_with_seed;
use ec_data::{CampaignStore, DEFAULT_CAMPAIGN_DB_NAME};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy file");
        }
    }
}

fn seeded_fixture_copy(seed: u64) -> PathBuf {
    let root = temp_dir("ec-client-cosmetic-seed");
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let store =
        CampaignStore::open(root.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open campaign store");
    import_directory_snapshot_with_seed(&store, &root, Some(seed)).expect("seed sqlite snapshot");
    root
}

#[test]
fn main_menu_renders_a_quote() {
    let mut screen = MainMenuScreen::new();

    let buffer = screen.render_with_notice(None).expect("render menu");

    let quote_rows: Vec<_> = (8..=23)
        .map(|row| buffer.plain_line(row))
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert!(
        quote_rows.iter().any(|line| line.starts_with(" --")),
        "main menu should display a quote with an author attribution"
    );
}

#[test]
fn startup_splash_styles_are_deterministic_for_campaign_seed() {
    let seed = 0xEC15_5350_4C41_5348u64;
    let fixture_a = seeded_fixture_copy(seed);
    let fixture_b = seeded_fixture_copy(seed);

    let mut app_a = App::load(AppConfig {
        game_dir: fixture_a,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("load first app");
    let mut app_b = App::load(AppConfig {
        game_dir: fixture_b,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
    })
    .expect("load second app");

    app_a.current_screen = ScreenId::Startup(app_a.startup_sequence.current());
    app_b.current_screen = ScreenId::Startup(app_b.startup_sequence.current());

    let buffer_a = ec_client::domains::startup::views::render(&mut app_a).expect("render app a");
    let buffer_b = ec_client::domains::startup::views::render(&mut app_b).expect("render app b");

    for row in 0..PLAYFIELD_HEIGHT {
        assert_eq!(buffer_a.row(row), buffer_b.row(row), "row {row} differs");
    }
}
