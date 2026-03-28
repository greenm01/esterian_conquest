use std::fs;
use std::path::{Path, PathBuf};

pub use ec_ui::theme::classic;
pub use ec_ui::theme::{AnsiMode, ThemeEntry, ThemeEntryKind};

pub fn ensure_bundled_themes_in_game_dir(
    game_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let themes_dir = game_dir.join("themes");
    fs::create_dir_all(&themes_dir)?;
    for (name, contents) in ec_ui::theme::bundled_theme_files() {
        let path = themes_dir.join(name);
        fs::write(path, contents)?;
    }
    Ok(())
}

pub fn discover_theme_entries(
    game_dir: &Path,
) -> Result<Vec<ThemeEntry>, Box<dyn std::error::Error>> {
    ensure_bundled_themes_in_game_dir(game_dir)?;
    let themes_dir = game_dir.join("themes");
    let mut entries = fs::read_dir(&themes_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("kdl"))
                .unwrap_or(false)
        })
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?;
            let key = ec_ui::theme::normalize_theme_key(stem);
            Some(ThemeEntry {
                key,
                display_name: ec_ui::theme::humanize_theme_name(stem),
                kind: ThemeEntryKind::Theme,
                path: Some(path),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.key.cmp(&right.key));
    entries.dedup_by(|left, right| left.key == right.key);
    entries.push(ThemeEntry {
        key: "mono".to_string(),
        display_name: "Mono".to_string(),
        kind: ThemeEntryKind::Mono,
        path: None,
    });
    Ok(entries)
}

/// Initialise the theme for a game directory.
///
/// `config_theme_path` is the value of the `theme` directive from
/// `config.kdl`, already resolved by the caller (pass `None` to skip).
///
/// Resolution order:
/// 1. `config_theme_path` — if `Some`, use it (relative paths are joined to
///    `game_dir`).
/// 2. `<game_dir>/theme.kdl` — direct theme file next to `ecgame.db`.
/// 3. Bootstrap: write the bundled default `theme.kdl` into `game_dir` and
///    use it.
///
/// On parse error the bundled default is used silently, so a corrupted user
/// theme never prevents the client from starting.
pub fn initialize_from_game_dir(
    game_dir: &Path,
    config_theme_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_bundled_themes_in_game_dir(game_dir)?;
    let theme_file = resolve_game_dir_theme(game_dir, config_theme_path)?;
    let theme_key = theme_key_for_path(game_dir, &theme_file);
    match fs::read_to_string(&theme_file) {
        Ok(contents) => {
            let _ =
                ec_ui::theme::apply_theme_from_kdl(&contents, AnsiMode::On, theme_key.as_deref())
                    .or_else(|_| {
                        ec_ui::theme::apply_theme_from_kdl(
                            ec_ui::theme::bundled_theme_kdl(),
                            AnsiMode::On,
                            Some(ec_ui::theme::default_theme_key()),
                        )
                    });
        }
        Err(_) => ec_ui::theme::apply_default_theme(),
    }
    Ok(())
}

/// Resolve (and if necessary bootstrap) the theme file path for a game
/// directory without loading or applying the theme.
fn resolve_game_dir_theme(
    game_dir: &Path,
    config_theme_path: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    ensure_bundled_themes_in_game_dir(game_dir)?;
    // 1. Explicit path from config.kdl
    if let Some(rel) = config_theme_path {
        let abs = if rel.is_absolute() {
            rel
        } else {
            game_dir.join(rel)
        };
        return Ok(abs);
    }

    Ok(game_dir.join("themes").join("tokyo_night.kdl"))
}

pub fn load_theme_from_path(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    ec_ui::theme::apply_theme_from_kdl(&contents, ansi_mode(), None).map_err(|err| err.into())
}

pub fn apply_theme_entry(entry: &ThemeEntry) -> Result<(), Box<dyn std::error::Error>> {
    match entry.kind {
        ThemeEntryKind::Mono => {
            ec_ui::theme::apply_mono_theme();
            Ok(())
        }
        ThemeEntryKind::Theme => {
            let path = entry
                .path
                .as_ref()
                .ok_or_else(|| format!("theme {:?} is missing a file path", entry.key))?;
            let contents = fs::read_to_string(path)?;
            ec_ui::theme::apply_theme_from_kdl(&contents, AnsiMode::On, Some(&entry.key))
                .map_err(|err| err.into())
        }
    }
}

pub fn apply_default_theme() {
    ec_ui::theme::apply_default_theme();
}

pub fn ansi_mode() -> AnsiMode {
    ec_ui::theme::ansi_mode()
}

pub fn ansi_enabled() -> bool {
    ec_ui::theme::ansi_enabled()
}

pub fn toggle_ansi_mode() -> Result<AnsiMode, Box<dyn std::error::Error>> {
    ec_ui::theme::toggle_ansi_mode().map_err(|err| err.into())
}

pub fn bundled_theme_kdl() -> &'static str {
    ec_ui::theme::bundled_theme_kdl()
}

pub fn bundled_theme_file_names() -> &'static [&'static str] {
    ec_ui::theme::bundled_theme_file_names()
}

pub fn current_theme_key() -> Option<String> {
    ec_ui::theme::current_theme_key()
}

pub fn default_theme_key() -> &'static str {
    ec_ui::theme::default_theme_key()
}

pub fn default_theme_display_name() -> String {
    ec_ui::theme::default_theme_display_name()
}

fn theme_key_for_path(game_dir: &Path, path: &Path) -> Option<String> {
    let themes_dir = game_dir.join("themes");
    let relative = path.strip_prefix(&themes_dir).ok()?;
    if relative.components().count() != 1 {
        return None;
    }
    let stem = relative.file_stem()?.to_str()?;
    Some(ec_ui::theme::normalize_theme_key(stem))
}
