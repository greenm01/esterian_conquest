use std::path::{Path, PathBuf};

pub use ec_ui::theme::classic;
pub use ec_ui::theme::{AnsiMode, ThemeEntry, ThemeEntryKind};

pub fn discover_theme_entries(
    _game_dir: &Path,
) -> Result<Vec<ThemeEntry>, Box<dyn std::error::Error>> {
    let mut entries = ec_ui::theme::bundled_theme_file_names()
        .iter()
        .filter_map(|name| {
            let stem = Path::new(name).file_stem()?.to_str()?;
            let key = ec_ui::theme::normalize_theme_key(stem);
            Some(ThemeEntry {
                key,
                display_name: ec_ui::theme::humanize_theme_name(stem),
                kind: ThemeEntryKind::Theme,
                path: None,
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

pub fn initialize_from_game_dir(
    game_dir: &Path,
    config_theme_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    match resolve_theme_source(game_dir, config_theme_path) {
        ThemeSource::Bundled { key, contents } => {
            let _ = ec_ui::theme::apply_theme_from_kdl(contents, AnsiMode::On, Some(&key)).or_else(
                |_| {
                    ec_ui::theme::apply_theme_from_kdl(
                        ec_ui::theme::bundled_theme_kdl(),
                        AnsiMode::On,
                        Some(ec_ui::theme::default_theme_key()),
                    )
                },
            );
        }
        ThemeSource::File { path, key } => match std::fs::read_to_string(&path) {
            Ok(contents) => {
                let _ = ec_ui::theme::apply_theme_from_kdl(&contents, AnsiMode::On, key.as_deref())
                    .or_else(|_| {
                        ec_ui::theme::apply_theme_from_kdl(
                            ec_ui::theme::bundled_theme_kdl(),
                            AnsiMode::On,
                            Some(ec_ui::theme::default_theme_key()),
                        )
                    });
            }
            Err(_) => ec_ui::theme::apply_default_theme(),
        },
    }
    Ok(())
}

enum ThemeSource {
    Bundled { key: String, contents: &'static str },
    File { path: PathBuf, key: Option<String> },
}

fn resolve_theme_source(game_dir: &Path, config_theme_path: Option<PathBuf>) -> ThemeSource {
    if let Some(rel) = config_theme_path {
        let abs = if rel.is_absolute() {
            rel
        } else {
            game_dir.join(rel)
        };
        if abs.exists() {
            let key = abs
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ec_ui::theme::normalize_theme_key);
            return ThemeSource::File { path: abs, key };
        }
        if let Some((key, contents)) = bundled_theme_for_path(&abs) {
            return ThemeSource::Bundled { key, contents };
        }
    }
    ThemeSource::Bundled {
        key: ec_ui::theme::default_theme_key().to_string(),
        contents: ec_ui::theme::bundled_theme_kdl(),
    }
}

pub fn load_theme_from_path(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    ec_ui::theme::apply_theme_from_kdl(&contents, ansi_mode(), None).map_err(|err| err.into())
}

pub fn apply_theme_entry(entry: &ThemeEntry) -> Result<(), Box<dyn std::error::Error>> {
    match entry.kind {
        ThemeEntryKind::Mono => {
            ec_ui::theme::apply_mono_theme();
            Ok(())
        }
        ThemeEntryKind::Theme => {
            if let Some(path) = entry.path.as_ref() {
                let contents = std::fs::read_to_string(path)?;
                return ec_ui::theme::apply_theme_from_kdl(
                    &contents,
                    AnsiMode::On,
                    Some(&entry.key),
                )
                .map_err(|err| err.into());
            }
            let file_name = format!("{}.kdl", entry.key);
            let contents = ec_ui::theme::bundled_theme_contents(&file_name)
                .ok_or_else(|| format!("unknown bundled theme {:?}", entry.key))?;
            ec_ui::theme::apply_theme_from_kdl(contents, AnsiMode::On, Some(&entry.key))
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

fn bundled_theme_for_path(path: &Path) -> Option<(String, &'static str)> {
    let stem = path.file_stem()?.to_str()?;
    let key = ec_ui::theme::normalize_theme_key(stem);
    let file_name = format!("{key}.kdl");
    let contents = ec_ui::theme::bundled_theme_contents(&file_name)?;
    Some((key, contents))
}
