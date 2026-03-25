use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use crate::screen::{CellStyle, GameColor};

const DEFAULT_THEME_KDL: &str = include_str!("../config/theme.kdl");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnsiMode {
    On,
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Theme {
    body: CellStyle,
    title: CellStyle,
    menu: CellStyle,
    menu_hotkey: CellStyle,
    prompt: CellStyle,
    prompt_hotkey: CellStyle,
    prompt_notice_action: CellStyle,
    bright: CellStyle,
    logo: CellStyle,
    intro_accent: CellStyle,
    intro_tribute: CellStyle,
    stardate_label: CellStyle,
    stardate_week: CellStyle,
    stardate_year: CellStyle,
    error: CellStyle,
    notice: CellStyle,
    status_label: CellStyle,
    status_value: CellStyle,
    table_chrome: CellStyle,
    table_header: CellStyle,
    table_body: CellStyle,
    disabled_row: CellStyle,
    selected: CellStyle,
    alert: CellStyle,
    help_header: CellStyle,
    help_panel: CellStyle,
    map_dot: CellStyle,
    map_crosshair: CellStyle,
    map_center: CellStyle,
    quote: CellStyle,
    quote_author: CellStyle,
    report_header: CellStyle,
    indicator_on: CellStyle,
    indicator_off: CellStyle,
    star_colors: [GameColor; 6],
}

impl Theme {
    fn from_kdl_str(source: &str) -> Result<Self, String> {
        let document: kdl::KdlDocument = source
            .parse()
            .map_err(|err| format!("parse theme.kdl: {err}"))?;

        let require_style = |name: &str| parse_named_style(&document, name);

        Ok(Self {
            body: require_style("body")?,
            title: require_style("title")?,
            menu: require_style("menu")?,
            menu_hotkey: require_style("menu_hotkey")?,
            prompt: require_style("prompt")?,
            prompt_hotkey: require_style("prompt_hotkey")?,
            prompt_notice_action: require_style("prompt_notice_action")?,
            bright: require_style("bright")?,
            logo: require_style("logo")?,
            intro_accent: require_style("intro_accent")?,
            intro_tribute: require_style("intro_tribute")?,
            stardate_label: require_style("stardate_label")?,
            stardate_week: require_style("stardate_week")?,
            stardate_year: require_style("stardate_year")?,
            error: require_style("error")?,
            notice: require_style("notice")?,
            status_label: require_style("status_label")?,
            status_value: require_style("status_value")?,
            table_chrome: require_style("table_chrome")?,
            table_header: require_style("table_header")?,
            table_body: require_style("table_body")?,
            disabled_row: require_style("disabled_row")?,
            selected: require_style("selected")?,
            alert: require_style("alert")?,
            help_header: require_style("help_header")?,
            help_panel: require_style("help_panel")?,
            map_dot: require_style("map_dot")?,
            map_crosshair: require_style("map_crosshair")?,
            map_center: require_style("map_center")?,
            quote: require_style("quote")?,
            quote_author: require_style("quote_author")?,
            report_header: require_style("report_header")?,
            indicator_on: require_style("indicator_on")?,
            indicator_off: require_style("indicator_off")?,
            star_colors: parse_star_colors(&document)?,
        })
    }

    fn bundled_default() -> Self {
        Self::from_kdl_str(DEFAULT_THEME_KDL).expect("bundled theme.kdl should be valid")
    }

    fn monochrome_projection(&self) -> Self {
        let mut theme = self.clone();

        theme.body = mono_dim(theme.body);
        theme.title = mono_bright(theme.title);
        theme.menu = mono_dim(theme.menu);
        theme.menu_hotkey = mono_bright(theme.menu_hotkey);
        theme.prompt = mono_dim(theme.prompt);
        theme.prompt_hotkey = mono_bright(theme.prompt_hotkey);
        theme.prompt_notice_action = mono_bright(theme.prompt_notice_action);
        theme.bright = mono_bright(theme.bright);
        theme.logo = mono_bright(theme.logo);
        theme.intro_accent = mono_bright(theme.intro_accent);
        theme.intro_tribute = mono_bright(theme.intro_tribute);
        theme.stardate_label = mono_bright(theme.stardate_label);
        theme.stardate_week = mono_bright(theme.stardate_week);
        theme.stardate_year = mono_bright(theme.stardate_year);
        theme.error = mono_bright(theme.error);
        theme.notice = mono_bright(theme.notice);
        theme.status_label = mono_dim(theme.status_label);
        theme.status_value = mono_normal(theme.status_value);
        theme.table_chrome = mono_normal(theme.table_chrome);
        theme.table_header = mono_bright(theme.table_header);
        theme.table_body = mono_normal(theme.table_body);
        theme.disabled_row = mono_muted(theme.disabled_row);
        theme.selected = mono_selected(theme.selected);
        theme.alert = mono_bright(theme.alert);
        theme.help_header = mono_bright(theme.help_header);
        theme.help_panel = mono_dim(theme.help_panel);
        theme.map_dot = mono_normal(theme.map_dot);
        theme.map_crosshair = mono_bright(theme.map_crosshair);
        theme.map_center = mono_bright(theme.map_center);
        theme.quote = mono_dim(theme.quote);
        theme.quote_author = mono_normal(theme.quote_author);
        theme.report_header = mono_bright(theme.report_header);
        theme.indicator_on = mono_bright(theme.indicator_on);
        theme.indicator_off = mono_muted(theme.indicator_off);
        theme.star_colors = [GameColor::BrightWhite; 6];

        theme
    }
}

fn mono_dim(style: CellStyle) -> CellStyle {
    CellStyle::new(GameColor::White, GameColor::Black, style.bold)
}

fn mono_muted(style: CellStyle) -> CellStyle {
    CellStyle::new(GameColor::BrightBlack, GameColor::Black, style.bold)
}

fn mono_normal(style: CellStyle) -> CellStyle {
    CellStyle::new(GameColor::White, GameColor::Black, style.bold)
}

fn mono_bright(style: CellStyle) -> CellStyle {
    CellStyle::new(GameColor::White, GameColor::Black, style.bold)
}

fn mono_selected(style: CellStyle) -> CellStyle {
    CellStyle::new(GameColor::Black, GameColor::BrightBlack, style.bold)
}

fn parse_named_style(document: &kdl::KdlDocument, style_name: &str) -> Result<CellStyle, String> {
    let node = document
        .nodes()
        .iter()
        .find(|node| {
            node.name().value() == "style"
                && node.get(0).and_then(|value| value.as_string()) == Some(style_name)
        })
        .ok_or_else(|| format!("missing style {style_name:?}"))?;
    parse_style_node(node)
}

fn parse_style_node(node: &kdl::KdlNode) -> Result<CellStyle, String> {
    let children = node
        .children()
        .ok_or_else(|| format!("style {:?} missing children", node.name().value()))?;
    let child_value = |name: &str| {
        children.nodes().iter().find_map(|child| {
            if child.name().value() == name {
                child.get(0)
            } else {
                None
            }
        })
    };
    let fg = parse_color_value(
        child_value("fg")
            .and_then(|value| value.as_string())
            .ok_or_else(|| format!("style {:?} missing fg", node.name().value()))?,
    )?;
    let bg = parse_color_value(
        child_value("bg")
            .and_then(|value| value.as_string())
            .ok_or_else(|| format!("style {:?} missing bg", node.name().value()))?,
    )?;
    let bold = child_value("bold")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    Ok(CellStyle::new(fg, bg, bold))
}

fn parse_star_colors(document: &kdl::KdlDocument) -> Result<[GameColor; 6], String> {
    let node = document
        .nodes()
        .iter()
        .find(|node| {
            let name = node.name().value();
            name == "star-colors" || name == "star_colors"
        })
        .ok_or_else(|| "missing star_colors".to_string())?;
    let mut colors = [GameColor::BrightBlue; 6];
    for (idx, slot) in colors.iter_mut().enumerate() {
        let value = node
            .get(idx)
            .and_then(|value| value.as_string())
            .ok_or_else(|| format!("star_colors missing entry {idx}"))?;
        *slot = parse_color_value(value)?;
    }
    Ok(colors)
}

fn parse_color_value(value: &str) -> Result<GameColor, String> {
    // Hex RGB: "#RRGGBB"
    if let Some(hex) = value.strip_prefix('#') {
        if hex.len() != 6 {
            return Err(format!(
                "hex color {value:?} must be exactly 6 hex digits (#RRGGBB)"
            ));
        }
        let parse_byte = |s: &str| -> Result<u8, String> {
            u8::from_str_radix(s, 16)
                .map_err(|_| format!("invalid hex byte {s:?} in color {value:?}"))
        };
        let r = parse_byte(&hex[0..2])?;
        let g = parse_byte(&hex[2..4])?;
        let b = parse_byte(&hex[4..6])?;
        return Ok(GameColor::Rgb(r, g, b));
    }

    // 256-color index: "idx:N" or "index:N"
    if let Some(rest) = value
        .strip_prefix("idx:")
        .or_else(|| value.strip_prefix("index:"))
    {
        let idx: u8 = rest
            .parse()
            .map_err(|_| format!("invalid palette index {rest:?} in color {value:?}"))?;
        return Ok(GameColor::Indexed(idx));
    }

    // Named 16-color values (backward compatible).
    match value.replace('-', "_").to_ascii_lowercase().as_str() {
        "black" => Ok(GameColor::Black),
        "red" => Ok(GameColor::Red),
        "green" => Ok(GameColor::Green),
        "yellow" => Ok(GameColor::Yellow),
        "blue" => Ok(GameColor::Blue),
        "magenta" => Ok(GameColor::Magenta),
        "cyan" => Ok(GameColor::Cyan),
        "white" | "grey" | "gray" => Ok(GameColor::White),
        "bright_black" | "dark_grey" | "dark_gray" => Ok(GameColor::BrightBlack),
        "bright_red" => Ok(GameColor::BrightRed),
        "bright_green" => Ok(GameColor::BrightGreen),
        "bright_yellow" => Ok(GameColor::BrightYellow),
        "bright_blue" => Ok(GameColor::BrightBlue),
        "bright_magenta" => Ok(GameColor::BrightMagenta),
        "bright_cyan" => Ok(GameColor::BrightCyan),
        "bright_white" | "light_grey" | "light_gray" => Ok(GameColor::BrightWhite),
        other => Err(format!(
            "unknown color {other:?} (use a named ANSI color, #RRGGBB, or idx:N)"
        )),
    }
}

fn active_theme_lock() -> &'static RwLock<Theme> {
    static ACTIVE_THEME: OnceLock<RwLock<Theme>> = OnceLock::new();
    ACTIVE_THEME.get_or_init(|| RwLock::new(Theme::bundled_default()))
}

fn base_theme_lock() -> &'static RwLock<Theme> {
    static BASE_THEME: OnceLock<RwLock<Theme>> = OnceLock::new();
    BASE_THEME.get_or_init(|| RwLock::new(Theme::bundled_default()))
}

fn ansi_mode_lock() -> &'static RwLock<AnsiMode> {
    static ANSI_MODE: OnceLock<RwLock<AnsiMode>> = OnceLock::new();
    ANSI_MODE.get_or_init(|| RwLock::new(AnsiMode::On))
}

fn active_theme() -> Theme {
    active_theme_lock()
        .read()
        .expect("theme lock poisoned")
        .clone()
}

fn set_active_theme(theme: Theme) {
    *active_theme_lock().write().expect("theme lock poisoned") = theme;
}

fn base_theme() -> Theme {
    base_theme_lock()
        .read()
        .expect("theme lock poisoned")
        .clone()
}

fn set_theme_state(theme: Theme, ansi_mode: AnsiMode) {
    *base_theme_lock().write().expect("theme lock poisoned") = theme.clone();
    *ansi_mode_lock().write().expect("theme lock poisoned") = ansi_mode;
    let active = if ansi_mode == AnsiMode::On {
        theme
    } else {
        theme.monochrome_projection()
    };
    set_active_theme(active);
}

pub fn bundled_theme_kdl() -> &'static str {
    DEFAULT_THEME_KDL
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
    let theme_file = resolve_game_dir_theme(game_dir, config_theme_path)?;
    let theme = match fs::read_to_string(&theme_file) {
        Ok(contents) => Theme::from_kdl_str(&contents).unwrap_or_else(|_| Theme::bundled_default()),
        Err(_) => Theme::bundled_default(),
    };
    set_theme_state(theme, AnsiMode::On);
    Ok(())
}

/// Resolve (and if necessary bootstrap) the theme file path for a game
/// directory without loading or applying the theme.
fn resolve_game_dir_theme(
    game_dir: &Path,
    config_theme_path: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // 1. Explicit path from config.kdl
    if let Some(rel) = config_theme_path {
        let abs = if rel.is_absolute() {
            rel
        } else {
            game_dir.join(rel)
        };
        return Ok(abs);
    }

    // 2. Direct theme.kdl in game dir
    let theme_file = game_dir.join("theme.kdl");
    if !theme_file.exists() {
        // 3. Bootstrap bundled default
        fs::write(&theme_file, DEFAULT_THEME_KDL)?;
    }
    Ok(theme_file)
}

pub fn load_theme_from_path(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let theme = Theme::from_kdl_str(&contents).map_err(|err| err.to_string())?;
    set_theme_state(theme, ansi_mode());
    Ok(())
}

pub fn ansi_mode() -> AnsiMode {
    *ansi_mode_lock().read().expect("theme lock poisoned")
}

pub fn ansi_enabled() -> bool {
    ansi_mode() == AnsiMode::On
}

pub fn toggle_ansi_mode() -> Result<AnsiMode, Box<dyn std::error::Error>> {
    let next_mode = if ansi_enabled() {
        AnsiMode::Off
    } else {
        AnsiMode::On
    };
    set_theme_state(base_theme(), next_mode);
    Ok(next_mode)
}

pub mod classic {
    use crate::screen::{CellStyle, GameColor};

    use super::active_theme;

    pub fn body_style() -> CellStyle {
        active_theme().body
    }

    pub fn title_style() -> CellStyle {
        active_theme().title
    }

    pub fn menu_style() -> CellStyle {
        active_theme().menu
    }

    pub fn menu_hotkey_style() -> CellStyle {
        active_theme().menu_hotkey
    }

    pub fn prompt_style() -> CellStyle {
        active_theme().prompt
    }

    pub fn prompt_hotkey_style() -> CellStyle {
        active_theme().prompt_hotkey
    }

    pub fn prompt_notice_action_style() -> CellStyle {
        active_theme().prompt_notice_action
    }

    pub fn bright_style() -> CellStyle {
        active_theme().bright
    }

    pub fn logo_style() -> CellStyle {
        active_theme().logo
    }

    pub fn intro_accent_style() -> CellStyle {
        active_theme().intro_accent
    }

    pub fn intro_tribute_style() -> CellStyle {
        active_theme().intro_tribute
    }

    pub fn stardate_label_style() -> CellStyle {
        active_theme().stardate_label
    }

    pub fn stardate_week_style() -> CellStyle {
        active_theme().stardate_week
    }

    pub fn stardate_year_style() -> CellStyle {
        active_theme().stardate_year
    }

    pub fn star_decoration_style(index: usize) -> CellStyle {
        let theme = active_theme();
        CellStyle::new(
            theme.star_colors[index % theme.star_colors.len()],
            theme.body.bg,
            false,
        )
    }

    pub fn status_label_style() -> CellStyle {
        active_theme().status_label
    }

    pub fn notice_style() -> CellStyle {
        active_theme().notice
    }

    pub fn error_style() -> CellStyle {
        active_theme().error
    }

    pub fn status_value_style() -> CellStyle {
        active_theme().status_value
    }

    pub fn table_chrome_style() -> CellStyle {
        active_theme().table_chrome
    }

    pub fn table_header_style() -> CellStyle {
        active_theme().table_header
    }

    pub fn table_body_style() -> CellStyle {
        active_theme().table_body
    }

    pub fn disabled_row_style() -> CellStyle {
        active_theme().disabled_row
    }

    pub fn selected_row_style() -> CellStyle {
        active_theme().selected
    }

    pub fn alert_style() -> CellStyle {
        active_theme().alert
    }

    pub fn help_header_style() -> CellStyle {
        active_theme().help_header
    }

    pub fn help_panel_style() -> CellStyle {
        active_theme().help_panel
    }

    pub fn map_dot_style() -> CellStyle {
        active_theme().map_dot
    }

    pub fn map_crosshair_style() -> CellStyle {
        active_theme().map_crosshair
    }

    pub fn map_center_style() -> CellStyle {
        active_theme().map_center
    }

    pub fn quote_style() -> CellStyle {
        active_theme().quote
    }

    pub fn quote_author_style() -> CellStyle {
        active_theme().quote_author
    }

    pub fn report_header_style() -> CellStyle {
        active_theme().report_header
    }

    pub fn indicator_on_style() -> CellStyle {
        active_theme().indicator_on
    }

    pub fn indicator_off_style() -> CellStyle {
        active_theme().indicator_off
    }

    pub fn app_background() -> GameColor {
        active_theme().body.bg
    }

    pub fn terminal_foreground() -> GameColor {
        active_theme().body.fg
    }
}
