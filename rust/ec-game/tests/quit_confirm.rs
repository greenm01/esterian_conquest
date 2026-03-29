use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::Path};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ec_compat::import_directory_snapshot;
use ec_data::{CampaignStore, GameConfig};
use ec_game::app::{Action, App, AppConfig, AppOutcome, apply_action};
use ec_game::screen::ScreenId;
use ec_game::terminal::Terminal;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_fixture_copy() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ec-game-quit-confirm-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
    root
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

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn load_app() -> App {
    let fixture_dir = temp_fixture_copy();
    let config = AppConfig {
        game_dir: fixture_dir,
        player_record_index_1_based: 1,
        export_root: None,
        queue_dir: None,
        session_timeout_secs: None,
        game_config: GameConfig::default(),
    };
    App::load(config).expect("load app")
}

#[derive(Default)]
struct CaptureTerminal {
    last_lines: Vec<String>,
}

impl Terminal for CaptureTerminal {
    fn render(
        &mut self,
        playfield: &ec_game::screen::PlayfieldBuffer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.last_lines = (0..playfield.height())
            .map(|row| playfield.plain_line(row))
            .collect();
        Ok(())
    }

    fn dump_text_capture(&mut self, _text: &str) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn read_key(&mut self) -> Result<KeyEvent, Box<dyn std::error::Error>> {
        unreachable!("read_key is not used in render tests")
    }

    fn clear_and_restore(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

#[test]
fn main_menu_quit_opens_confirmation_and_can_cancel() {
    let mut app = load_app();
    app.current_screen = ScreenId::MainMenu;

    assert_eq!(app.handle_key(key(KeyCode::Char('q'))), Action::RequestQuit);
    assert_eq!(
        apply_action(&mut app, Action::RequestQuit),
        AppOutcome::Continue
    );
    assert!(app.quit_confirm_open);

    assert_eq!(app.handle_key(key(KeyCode::Esc)), Action::CancelQuitPrompt);
    assert_eq!(
        apply_action(&mut app, Action::CancelQuitPrompt),
        AppOutcome::Continue
    );
    assert!(!app.quit_confirm_open);
}

#[test]
fn quit_confirm_renders_and_only_y_exits() {
    let mut app = load_app();
    app.current_screen = ScreenId::MainMenu;
    app.request_quit();

    let mut terminal = CaptureTerminal::default();
    app.render(&mut terminal).expect("render");
    assert!(
        terminal
            .last_lines
            .iter()
            .any(|line| line.contains("Are you sure Y/[N] ->"))
    );

    let cancel = app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(apply_action(&mut app, cancel), AppOutcome::Continue);
    assert!(!app.quit_confirm_open);

    app.request_quit();
    let confirm = app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(apply_action(&mut app, confirm), AppOutcome::Quit);
}

#[test]
fn first_time_menu_quit_confirm_reuses_the_menu_prompt_row() {
    let mut app = load_app();
    app.current_screen = ScreenId::FirstTimeMenu;
    app.request_quit();

    let mut terminal = CaptureTerminal::default();
    app.render(&mut terminal).expect("render");

    assert!(
        terminal.last_lines[4].contains("Are you sure Y/[N] ->"),
        "quit confirm should replace the first-time command row"
    );
    assert!(
        !terminal
            .last_lines
            .last()
            .expect("last line")
            .contains("Are you sure Y/[N] ->"),
        "quit confirm should not fall to the bottom command line"
    );
}
