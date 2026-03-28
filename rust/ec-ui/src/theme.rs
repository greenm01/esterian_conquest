use std::cell::RefCell;
use std::path::PathBuf;

use crate::buffer::{CellStyle, GameColor};

const DEFAULT_THEME_KDL: &str = include_str!("../../ec-game/config/themes/tokyo_night.kdl");
const MATRIX_THEME_KDL: &str = include_str!("../../ec-game/config/themes/matrix.kdl");
const CATPPUCCIN_MOCHA_THEME_KDL: &str =
    include_str!("../../ec-game/config/themes/catppuccin_mocha.kdl");
const DRACULA_THEME_KDL: &str = include_str!("../../ec-game/config/themes/dracula.kdl");
const EVERFOREST_THEME_KDL: &str = include_str!("../../ec-game/config/themes/everforest.kdl");
const GRUVBOX_THEME_KDL: &str = include_str!("../../ec-game/config/themes/gruvbox.kdl");
const KANAGAWA_THEME_KDL: &str = include_str!("../../ec-game/config/themes/kanagawa.kdl");
const NORD_THEME_KDL: &str = include_str!("../../ec-game/config/themes/nord.kdl");
const ONE_DARK_THEME_KDL: &str = include_str!("../../ec-game/config/themes/one_dark.kdl");
const ROSE_PINE_THEME_KDL: &str = include_str!("../../ec-game/config/themes/rose_pine.kdl");
const SOLARIZED_THEME_KDL: &str = include_str!("../../ec-game/config/themes/solarized.kdl");
const MAG16_THEME_KDL: &str = include_str!("../../ec-game/config/themes/mag16.kdl");
const DEFAULT_THEME_KEY: &str = "tokyo_night";
const MONO_THEME_KEY: &str = "mono";

const BUNDLED_THEME_FILES: &[(&str, &str)] = &[
    ("catppuccin_mocha.kdl", CATPPUCCIN_MOCHA_THEME_KDL),
    ("mag16.kdl", MAG16_THEME_KDL),
    ("dracula.kdl", DRACULA_THEME_KDL),
    ("everforest.kdl", EVERFOREST_THEME_KDL),
    ("gruvbox.kdl", GRUVBOX_THEME_KDL),
    ("kanagawa.kdl", KANAGAWA_THEME_KDL),
    ("matrix.kdl", MATRIX_THEME_KDL),
    ("nord.kdl", NORD_THEME_KDL),
    ("one_dark.kdl", ONE_DARK_THEME_KDL),
    ("rose_pine.kdl", ROSE_PINE_THEME_KDL),
    ("solarized.kdl", SOLARIZED_THEME_KDL),
    ("tokyo_night.kdl", DEFAULT_THEME_KDL),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnsiMode {
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeEntryKind {
    Theme,
    Mono,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeEntry {
    pub key: String,
    pub display_name: String,
    pub kind: ThemeEntryKind,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Theme {
    body: CellStyle,
    title: CellStyle,
    shell_title: CellStyle,
    shell_label: CellStyle,
    menu: CellStyle,
    menu_hotkey: CellStyle,
    prompt: CellStyle,
    prompt_angle_delimiter: CellStyle,
    prompt_square_delimiter: CellStyle,
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
        let optional_style =
            |name: &str, fallback: &str| parse_named_style_or(&document, name, fallback);

        Ok(Self {
            body: require_style("body")?,
            title: require_style("title")?,
            shell_title: require_style("shell_title")?,
            shell_label: require_style("shell_label")?,
            menu: require_style("menu")?,
            menu_hotkey: require_style("menu_hotkey")?,
            prompt: require_style("prompt")?,
            prompt_angle_delimiter: optional_style("prompt_angle_delimiter", "prompt")?,
            prompt_square_delimiter: optional_style("prompt_square_delimiter", "prompt")?,
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
        theme.shell_title = mono_selected(theme.shell_title);
        theme.shell_label = mono_bright(theme.shell_label);
        theme.menu = mono_dim(theme.menu);
        theme.menu_hotkey = mono_bright(theme.menu_hotkey);
        theme.prompt = mono_dim(theme.prompt);
        theme.prompt_angle_delimiter = mono_dim(theme.prompt_angle_delimiter);
        theme.prompt_square_delimiter = mono_dim(theme.prompt_square_delimiter);
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

fn parse_named_style_or(
    document: &kdl::KdlDocument,
    style_name: &str,
    fallback_name: &str,
) -> Result<CellStyle, String> {
    parse_named_style(document, style_name).or_else(|_| parse_named_style(document, fallback_name))
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

    if let Some(rest) = value
        .strip_prefix("idx:")
        .or_else(|| value.strip_prefix("index:"))
    {
        let idx: u8 = rest
            .parse()
            .map_err(|_| format!("invalid palette index {rest:?} in color {value:?}"))?;
        return Ok(GameColor::Indexed(idx));
    }

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

thread_local! {
    static ACTIVE_THEME: RefCell<Theme> = RefCell::new(Theme::bundled_default());
    static BASE_THEME: RefCell<Theme> = RefCell::new(Theme::bundled_default());
    static ANSI_MODE: RefCell<AnsiMode> = const { RefCell::new(AnsiMode::On) };
    static CURRENT_THEME_KEY: RefCell<Option<String>> =
        RefCell::new(Some(DEFAULT_THEME_KEY.to_string()));
}

fn active_theme() -> Theme {
    ACTIVE_THEME.with(|theme| theme.borrow().clone())
}

fn set_active_theme(theme: Theme) {
    ACTIVE_THEME.with(|active| *active.borrow_mut() = theme);
}

fn base_theme() -> Theme {
    BASE_THEME.with(|theme| theme.borrow().clone())
}

fn set_theme_state(theme: Theme, ansi_mode: AnsiMode, theme_key: Option<&str>) {
    BASE_THEME.with(|base| *base.borrow_mut() = theme.clone());
    ANSI_MODE.with(|mode| *mode.borrow_mut() = ansi_mode);
    CURRENT_THEME_KEY.with(|key| {
        *key.borrow_mut() = theme_key.map(|value| value.to_string());
    });
    let active = if ansi_mode == AnsiMode::On {
        theme
    } else {
        theme.monochrome_projection()
    };
    set_active_theme(active);
}

pub fn apply_theme_from_kdl(
    source: &str,
    ansi_mode: AnsiMode,
    theme_key: Option<&str>,
) -> Result<(), String> {
    let theme = Theme::from_kdl_str(source)?;
    set_theme_state(theme, ansi_mode, theme_key);
    Ok(())
}

pub fn apply_default_theme() {
    set_theme_state(
        Theme::bundled_default(),
        AnsiMode::On,
        Some(DEFAULT_THEME_KEY),
    );
}

pub fn apply_mono_theme() {
    set_theme_state(
        Theme::bundled_default(),
        AnsiMode::Off,
        Some(MONO_THEME_KEY),
    );
}

pub fn bundled_theme_kdl() -> &'static str {
    DEFAULT_THEME_KDL
}

pub fn bundled_theme_file_names() -> &'static [&'static str] {
    &[
        "catppuccin_mocha.kdl",
        "mag16.kdl",
        "dracula.kdl",
        "everforest.kdl",
        "gruvbox.kdl",
        "kanagawa.kdl",
        "matrix.kdl",
        "nord.kdl",
        "one_dark.kdl",
        "rose_pine.kdl",
        "solarized.kdl",
        "tokyo_night.kdl",
    ]
}

pub fn bundled_theme_files() -> &'static [(&'static str, &'static str)] {
    BUNDLED_THEME_FILES
}

pub fn bundled_theme_contents(file_name: &str) -> Option<&'static str> {
    BUNDLED_THEME_FILES
        .iter()
        .find_map(|(name, contents)| (*name == file_name).then_some(*contents))
}

pub fn current_theme_key() -> Option<String> {
    CURRENT_THEME_KEY.with(|key| key.borrow().clone())
}

pub fn default_theme_key() -> &'static str {
    DEFAULT_THEME_KEY
}

pub fn default_theme_display_name() -> String {
    humanize_theme_name(DEFAULT_THEME_KEY)
}

pub fn ansi_mode() -> AnsiMode {
    ANSI_MODE.with(|mode| *mode.borrow())
}

pub fn ansi_enabled() -> bool {
    ansi_mode() == AnsiMode::On
}

pub fn toggle_ansi_mode() -> Result<AnsiMode, String> {
    let next_mode = if ansi_enabled() {
        AnsiMode::Off
    } else {
        AnsiMode::On
    };
    let current_key = current_theme_key();
    set_theme_state(base_theme(), next_mode, current_key.as_deref());
    Ok(next_mode)
}

pub fn normalize_theme_key(stem: &str) -> String {
    stem.trim().to_ascii_lowercase()
}

pub fn humanize_theme_name(stem: &str) -> String {
    stem.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.extend(first.to_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub mod classic {
    use crate::buffer::{CellStyle, GameColor};

    use super::active_theme;

    pub fn body_style() -> CellStyle {
        active_theme().body
    }

    pub fn title_style() -> CellStyle {
        active_theme().title
    }

    pub fn shell_title_style() -> CellStyle {
        active_theme().shell_title
    }

    pub fn shell_label_style() -> CellStyle {
        active_theme().shell_label
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

    pub fn prompt_angle_delimiter_style() -> CellStyle {
        active_theme().prompt_angle_delimiter
    }

    pub fn prompt_square_delimiter_style() -> CellStyle {
        active_theme().prompt_square_delimiter
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

    pub fn scrollbar_thumb_style() -> CellStyle {
        active_theme().indicator_on
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

#[cfg(test)]
mod tests {
    use super::{
        AnsiMode, apply_default_theme, apply_mono_theme, apply_theme_from_kdl, current_theme_key,
    };

    #[test]
    fn bundled_theme_is_valid_kdl() {
        apply_default_theme();
        assert_eq!(current_theme_key().as_deref(), Some("tokyo_night"));
    }

    #[test]
    fn mono_theme_sets_theme_key() {
        apply_mono_theme();
        assert_eq!(current_theme_key().as_deref(), Some("mono"));
    }

    #[test]
    fn invalid_theme_is_rejected() {
        let err = apply_theme_from_kdl("style body", AnsiMode::On, Some("bad"))
            .expect_err("invalid theme should fail");
        assert!(!err.is_empty());
    }
}
