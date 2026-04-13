//! Dashboard and lobby theme runtime local to `nc-dash`.

use std::cell::RefCell;

use nc_ui::table::TableRenderTheme;
use nc_ui::{CellStyle, GameColor};
use opaline::{OpalineColor, OpalineStyle};
use ratatui::style::{Color, Modifier, Style};

const DEFAULT_THEME_KEY: &str = "tokyo-night";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeCatalogEntry {
    pub key: String,
    pub display_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CellTheme {
    body: CellStyle,
    title: CellStyle,
    shell_title: CellStyle,
    shell_label: CellStyle,
    menu: CellStyle,
    menu_hotkey: CellStyle,
    menu_featured_label: CellStyle,
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
    empire_colors: [GameColor; 12],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TuiTheme {
    pub body: Style,
    pub panel: Style,
    pub border: Style,
    pub title: Style,
    pub accent: Style,
    pub label: Style,
    pub value: Style,
    pub dim: Style,
    pub selected: Style,
    pub success: Style,
    pub warning: Style,
    pub error: Style,
    pub footer: Style,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DashTheme {
    cell: CellTheme,
    tui: TuiTheme,
}

thread_local! {
    static ACTIVE_THEME: RefCell<DashTheme> = RefCell::new(load_theme(DEFAULT_THEME_KEY));
    static CURRENT_THEME_KEY: RefCell<String> = RefCell::new(DEFAULT_THEME_KEY.to_string());
}

pub fn default_theme_key() -> &'static str {
    DEFAULT_THEME_KEY
}

pub fn normalize_theme_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace('_', "-")
}

#[allow(dead_code)]
pub fn current_theme_key() -> String {
    CURRENT_THEME_KEY.with(|key| key.borrow().clone())
}

pub fn apply_default_theme() {
    apply_theme_key(DEFAULT_THEME_KEY).expect("bundled default theme should load");
}

pub fn apply_theme_key(key: &str) -> Result<(), String> {
    let key = normalize_theme_key(key);
    let loaded = load_theme_by_key(&key)?;
    ACTIVE_THEME.with(|theme| *theme.borrow_mut() = loaded);
    CURRENT_THEME_KEY.with(|current| *current.borrow_mut() = key);
    Ok(())
}

pub fn catalog() -> Vec<ThemeCatalogEntry> {
    let mut entries = opaline::builtins::builtin_names()
        .iter()
        .map(|(key, display_name)| ThemeCatalogEntry {
            key: (*key).to_string(),
            display_name: (*display_name).to_string(),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.display_name.cmp(&right.display_name));
    entries
}

pub fn display_name_for_key(key: &str) -> String {
    let key = normalize_theme_key(key);
    catalog()
        .into_iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.display_name)
        .unwrap_or_else(|| key)
}

pub fn tui_theme() -> TuiTheme {
    ACTIVE_THEME.with(|theme| theme.borrow().tui)
}

fn active_cell_theme() -> CellTheme {
    ACTIVE_THEME.with(|theme| theme.borrow().cell)
}

fn load_theme(key: &str) -> DashTheme {
    load_theme_by_key(key).unwrap_or_else(|_| load_theme_by_key(DEFAULT_THEME_KEY).expect("default theme"))
}

fn load_theme_by_key(key: &str) -> Result<DashTheme, String> {
    let theme = opaline::builtins::load_by_name(key)
        .ok_or_else(|| format!("unknown nc-dash theme {key:?}"))?;
    Ok(project_theme(&theme))
}

fn project_theme(theme: &opaline::Theme) -> DashTheme {
    let body_fg = color(theme, "text.primary");
    let body_bg = color(theme, "bg.base");
    let panel_bg = color(theme, "bg.panel");
    let accent = color(theme, "accent.primary");
    let accent_alt = color(theme, "accent.secondary");
    let accent_tertiary = color(theme, "accent.tertiary");
    let success = color(theme, "success");
    let warning = color(theme, "warning");
    let error = color(theme, "error");
    let info = color(theme, "info");
    let label = color(theme, "text.secondary");
    let dim = color(theme, "text.dim");
    let chrome = color(theme, "border.unfocused");
    let chrome_focus = color(theme, "border.focused");

    let body = CellStyle::new(body_fg, body_bg, false);
    let cell = CellTheme {
        body,
        title: CellStyle::new(accent, body_bg, true),
        shell_title: CellStyle::new(accent, body_bg, true),
        shell_label: CellStyle::new(label, body_bg, false),
        menu: CellStyle::new(body_fg, body_bg, false),
        menu_hotkey: CellStyle::new(accent_alt, body_bg, true),
        menu_featured_label: CellStyle::new(accent_alt, body_bg, true),
        prompt: CellStyle::new(body_fg, body_bg, false),
        prompt_angle_delimiter: CellStyle::new(label, body_bg, false),
        prompt_square_delimiter: CellStyle::new(label, body_bg, false),
        prompt_hotkey: CellStyle::new(accent_alt, body_bg, true),
        prompt_notice_action: CellStyle::new(info, body_bg, false),
        bright: CellStyle::new(body_fg, body_bg, true),
        logo: CellStyle::new(accent, body_bg, true),
        intro_accent: CellStyle::new(accent_tertiary, body_bg, true),
        intro_tribute: CellStyle::new(label, body_bg, false),
        stardate_label: CellStyle::new(label, body_bg, false),
        stardate_week: CellStyle::new(body_fg, body_bg, false),
        stardate_year: CellStyle::new(accent, body_bg, true),
        error: style_on_body(theme.style("error_style"), error, body_bg),
        notice: style_on_body(theme.style("info_style"), info, body_bg),
        status_label: CellStyle::new(label, body_bg, false),
        status_value: CellStyle::new(body_fg, body_bg, false),
        table_chrome: CellStyle::new(chrome, body_bg, false),
        table_header: CellStyle::new(accent, body_bg, true),
        table_body: CellStyle::new(body_fg, body_bg, false),
        disabled_row: style_on_body(theme.style("dimmed"), dim, body_bg),
        selected: projected_style(theme.style("selected"), body_fg, panel_bg),
        alert: style_on_body(theme.style("warning_style"), warning, body_bg),
        help_header: CellStyle::new(accent, body_bg, true),
        help_panel: CellStyle::new(body_fg, body_bg, false),
        map_dot: CellStyle::new(success, body_bg, false),
        map_crosshair: CellStyle::new(chrome_focus, body_bg, true),
        map_center: CellStyle::new(accent_alt, body_bg, true),
        quote: CellStyle::new(dim, body_bg, false),
        quote_author: CellStyle::new(label, body_bg, false),
        report_header: CellStyle::new(accent, body_bg, true),
        indicator_on: CellStyle::new(accent_alt, body_bg, true),
        indicator_off: CellStyle::new(dim, body_bg, false),
        star_colors: [
            accent,
            accent_alt,
            accent_tertiary,
            warning,
            info,
            label,
        ],
        empire_colors: [
            accent,
            accent_alt,
            accent_tertiary,
            color(theme, "accent.deep"),
            success,
            error,
            warning,
            info,
            body_fg,
            label,
            chrome_focus,
            chrome,
        ],
    };

    let tui = TuiTheme {
        body: ratatui_style(body),
        panel: Style::default()
            .fg(to_tui_color(body_fg))
            .bg(to_tui_color(panel_bg)),
        border: Style::default()
            .fg(to_tui_color(chrome))
            .bg(to_tui_color(body_bg)),
        title: Style::default()
            .fg(to_tui_color(accent))
            .bg(to_tui_color(body_bg))
            .add_modifier(Modifier::BOLD),
        accent: Style::default()
            .fg(to_tui_color(accent_alt))
            .bg(to_tui_color(body_bg))
            .add_modifier(Modifier::BOLD),
        label: Style::default()
            .fg(to_tui_color(label))
            .bg(to_tui_color(body_bg)),
        value: Style::default()
            .fg(to_tui_color(body_fg))
            .bg(to_tui_color(body_bg)),
        dim: Style::default()
            .fg(to_tui_color(dim))
            .bg(to_tui_color(body_bg)),
        selected: ratatui_style(cell.selected),
        success: Style::default()
            .fg(to_tui_color(success))
            .bg(to_tui_color(body_bg)),
        warning: Style::default()
            .fg(to_tui_color(warning))
            .bg(to_tui_color(body_bg))
            .add_modifier(Modifier::BOLD),
        error: Style::default()
            .fg(to_tui_color(error))
            .bg(to_tui_color(body_bg))
            .add_modifier(Modifier::BOLD),
        footer: Style::default()
            .fg(to_tui_color(label))
            .bg(to_tui_color(body_bg)),
    };

    DashTheme { cell, tui }
}

fn color(theme: &opaline::Theme, token: &str) -> GameColor {
    to_game_color(theme.color(token))
}

fn style_on_body(style: OpalineStyle, default_fg: GameColor, body_bg: GameColor) -> CellStyle {
    projected_style(style, default_fg, body_bg)
}

fn projected_style(style: OpalineStyle, default_fg: GameColor, default_bg: GameColor) -> CellStyle {
    let mut fg = style.fg.map(to_game_color).unwrap_or(default_fg);
    let mut bg = style.bg.map(to_game_color).unwrap_or(default_bg);
    if style.reversed {
        std::mem::swap(&mut fg, &mut bg);
    }
    if style.hidden {
        fg = bg;
    }
    CellStyle::new(fg, bg, style.bold)
}

fn ratatui_style(style: CellStyle) -> Style {
    let mut out = Style::default()
        .fg(to_tui_color(style.fg))
        .bg(to_tui_color(style.bg));
    if style.bold {
        out = out.add_modifier(Modifier::BOLD);
    }
    out
}

fn to_game_color(color: OpalineColor) -> GameColor {
    GameColor::Rgb(color.r, color.g, color.b)
}

pub fn to_tui_color(color: GameColor) -> Color {
    match color {
        GameColor::Black => Color::Black,
        GameColor::Red => Color::Red,
        GameColor::Green => Color::Green,
        GameColor::Yellow => Color::Yellow,
        GameColor::Blue => Color::Blue,
        GameColor::Magenta => Color::Magenta,
        GameColor::Cyan => Color::Cyan,
        GameColor::White => Color::Gray,
        GameColor::BrightBlack => Color::DarkGray,
        GameColor::BrightRed => Color::LightRed,
        GameColor::BrightGreen => Color::LightGreen,
        GameColor::BrightYellow => Color::LightYellow,
        GameColor::BrightBlue => Color::LightBlue,
        GameColor::BrightMagenta => Color::LightMagenta,
        GameColor::BrightCyan => Color::LightCyan,
        GameColor::BrightWhite => Color::White,
        GameColor::Indexed(index) => Color::Indexed(index),
        GameColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

pub fn from_tui_color(color: Color, fallback: GameColor) -> GameColor {
    match color {
        Color::Reset => fallback,
        Color::Black => GameColor::Black,
        Color::Red => GameColor::Red,
        Color::Green => GameColor::Green,
        Color::Yellow => GameColor::Yellow,
        Color::Blue => GameColor::Blue,
        Color::Magenta => GameColor::Magenta,
        Color::Cyan => GameColor::Cyan,
        Color::Gray => GameColor::White,
        Color::DarkGray => GameColor::BrightBlack,
        Color::LightRed => GameColor::BrightRed,
        Color::LightGreen => GameColor::BrightGreen,
        Color::LightYellow => GameColor::BrightYellow,
        Color::LightBlue => GameColor::BrightBlue,
        Color::LightMagenta => GameColor::BrightMagenta,
        Color::LightCyan => GameColor::BrightCyan,
        Color::White => GameColor::BrightWhite,
        Color::Indexed(index) => GameColor::Indexed(index),
        Color::Rgb(r, g, b) => GameColor::Rgb(r, g, b),
    }
}

fn on_body(style: CellStyle) -> CellStyle {
    CellStyle::new(style.fg, body_style().bg, style.bold)
}

pub fn body_style() -> CellStyle {
    active_cell_theme().body
}

pub fn shell_title_style() -> CellStyle {
    active_cell_theme().shell_title
}

pub fn shell_label_style() -> CellStyle {
    active_cell_theme().shell_label
}

pub fn prompt_style() -> CellStyle {
    active_cell_theme().prompt
}

pub fn prompt_hotkey_style() -> CellStyle {
    active_cell_theme().prompt_hotkey
}

pub fn prompt_notice_action_style() -> CellStyle {
    active_cell_theme().prompt_notice_action
}

pub fn table_body_style() -> CellStyle {
    active_cell_theme().table_body
}

pub fn table_header_style() -> CellStyle {
    active_cell_theme().table_header
}

pub fn table_chrome_style() -> CellStyle {
    active_cell_theme().table_chrome
}

pub fn status_label_style() -> CellStyle {
    active_cell_theme().status_label
}

pub fn logo_style() -> CellStyle {
    active_cell_theme().logo
}

#[allow(dead_code)]
pub fn star_decoration_style(index: usize) -> CellStyle {
    let theme = active_cell_theme();
    CellStyle::new(
        theme.star_colors[index % theme.star_colors.len()],
        theme.body.bg,
        false,
    )
}

pub fn empire_slot_style(slot: u8) -> CellStyle {
    let theme = active_cell_theme();
    let idx = slot.saturating_sub(1) as usize % theme.empire_colors.len();
    CellStyle::new(theme.empire_colors[idx], theme.body.bg, false)
}

pub fn empire_slot_style_on(slot: u8, bg: GameColor, bold: bool) -> CellStyle {
    let theme = active_cell_theme();
    let idx = slot.saturating_sub(1) as usize % theme.empire_colors.len();
    CellStyle::new(theme.empire_colors[idx], bg, bold)
}

pub fn border_style() -> CellStyle {
    active_cell_theme().table_chrome
}

pub fn header_style() -> CellStyle {
    active_cell_theme().shell_label
}

pub fn title_style() -> CellStyle {
    active_cell_theme().title
}

pub fn section_title_style() -> CellStyle {
    active_cell_theme().table_header
}

pub fn label_style() -> CellStyle {
    active_cell_theme().status_label
}

pub fn value_style() -> CellStyle {
    active_cell_theme().status_value
}

pub fn alert_style() -> CellStyle {
    on_body(active_cell_theme().alert)
}

pub fn error_style() -> CellStyle {
    on_body(active_cell_theme().error)
}

pub fn dim_style() -> CellStyle {
    active_cell_theme().disabled_row
}

pub fn dim_style_on(bg: GameColor, bold: bool) -> CellStyle {
    let style = dim_style();
    CellStyle::new(style.fg, bg, bold)
}

pub fn enemy_style() -> CellStyle {
    on_body(active_cell_theme().error)
}

pub fn friendly_style() -> CellStyle {
    on_body(active_cell_theme().notice)
}

pub fn icd_style() -> CellStyle {
    on_body(active_cell_theme().alert)
}

pub fn map_crosshair_style() -> CellStyle {
    active_cell_theme().map_crosshair
}

pub fn map_center_style() -> CellStyle {
    active_cell_theme().map_center
}

#[cfg(test)]
pub fn map_fleet_marker_style() -> CellStyle {
    on_body(active_cell_theme().map_dot)
}

pub fn map_fleet_marker_style_on(bg: GameColor, bold: bool) -> CellStyle {
    let style = active_cell_theme().map_dot;
    CellStyle::new(style.fg, bg, bold)
}

pub fn table_theme() -> TableRenderTheme {
    let theme = active_cell_theme();
    TableRenderTheme {
        header_style: theme.table_header,
        chrome_style: theme.table_chrome,
        body_style: theme.table_body,
        disabled_row_style: theme.disabled_row,
        selected_style: theme.selected,
        scroll_track_style: theme.body,
        scrollbar_thumb_style: theme.indicator_on,
    }
}

#[allow(dead_code)]
pub mod classic {
    use nc_ui::{CellStyle, GameColor};

    use super::active_cell_theme;

    pub fn body_style() -> CellStyle {
        active_cell_theme().body
    }

    pub fn title_style() -> CellStyle {
        active_cell_theme().title
    }

    pub fn shell_title_style() -> CellStyle {
        active_cell_theme().shell_title
    }

    pub fn shell_label_style() -> CellStyle {
        active_cell_theme().shell_label
    }

    pub fn menu_style() -> CellStyle {
        active_cell_theme().menu
    }

    pub fn menu_hotkey_style() -> CellStyle {
        active_cell_theme().menu_hotkey
    }

    pub fn menu_featured_label_style() -> CellStyle {
        active_cell_theme().menu_featured_label
    }

    pub fn prompt_style() -> CellStyle {
        active_cell_theme().prompt
    }

    pub fn prompt_angle_delimiter_style() -> CellStyle {
        active_cell_theme().prompt_angle_delimiter
    }

    pub fn prompt_square_delimiter_style() -> CellStyle {
        active_cell_theme().prompt_square_delimiter
    }

    pub fn prompt_hotkey_style() -> CellStyle {
        active_cell_theme().prompt_hotkey
    }

    pub fn prompt_notice_action_style() -> CellStyle {
        active_cell_theme().prompt_notice_action
    }

    pub fn bright_style() -> CellStyle {
        active_cell_theme().bright
    }

    pub fn logo_style() -> CellStyle {
        active_cell_theme().logo
    }

    pub fn intro_accent_style() -> CellStyle {
        active_cell_theme().intro_accent
    }

    pub fn intro_tribute_style() -> CellStyle {
        active_cell_theme().intro_tribute
    }

    pub fn stardate_label_style() -> CellStyle {
        active_cell_theme().stardate_label
    }

    pub fn stardate_week_style() -> CellStyle {
        active_cell_theme().stardate_week
    }

    pub fn stardate_year_style() -> CellStyle {
        active_cell_theme().stardate_year
    }

    pub fn star_decoration_style(index: usize) -> CellStyle {
        let theme = active_cell_theme();
        CellStyle::new(
            theme.star_colors[index % theme.star_colors.len()],
            theme.body.bg,
            false,
        )
    }

    pub fn empire_slot_color(slot: u8) -> GameColor {
        let theme = active_cell_theme();
        let idx = slot.saturating_sub(1) as usize % theme.empire_colors.len();
        theme.empire_colors[idx]
    }

    pub fn empire_slot_style(slot: u8) -> CellStyle {
        let theme = active_cell_theme();
        CellStyle::new(empire_slot_color(slot), theme.body.bg, false)
    }

    pub fn empire_slot_style_on(slot: u8, bg: GameColor, bold: bool) -> CellStyle {
        CellStyle::new(empire_slot_color(slot), bg, bold)
    }

    pub fn status_label_style() -> CellStyle {
        active_cell_theme().status_label
    }

    pub fn notice_style() -> CellStyle {
        active_cell_theme().notice
    }

    pub fn error_style() -> CellStyle {
        active_cell_theme().error
    }

    pub fn status_value_style() -> CellStyle {
        active_cell_theme().status_value
    }

    pub fn table_chrome_style() -> CellStyle {
        active_cell_theme().table_chrome
    }

    pub fn table_header_style() -> CellStyle {
        active_cell_theme().table_header
    }

    pub fn table_body_style() -> CellStyle {
        active_cell_theme().table_body
    }

    pub fn disabled_row_style() -> CellStyle {
        active_cell_theme().disabled_row
    }

    pub fn selected_row_style() -> CellStyle {
        active_cell_theme().selected
    }

    pub fn scrollbar_thumb_style() -> CellStyle {
        active_cell_theme().indicator_on
    }

    pub fn alert_style() -> CellStyle {
        active_cell_theme().alert
    }

    pub fn help_header_style() -> CellStyle {
        active_cell_theme().help_header
    }

    pub fn help_panel_style() -> CellStyle {
        active_cell_theme().help_panel
    }

    pub fn map_dot_style() -> CellStyle {
        active_cell_theme().map_dot
    }

    pub fn map_crosshair_style() -> CellStyle {
        active_cell_theme().map_crosshair
    }

    pub fn map_center_style() -> CellStyle {
        active_cell_theme().map_center
    }

    pub fn quote_style() -> CellStyle {
        active_cell_theme().quote
    }

    pub fn quote_author_style() -> CellStyle {
        active_cell_theme().quote_author
    }

    pub fn report_header_style() -> CellStyle {
        active_cell_theme().report_header
    }

    pub fn indicator_on_style() -> CellStyle {
        active_cell_theme().indicator_on
    }

    pub fn indicator_off_style() -> CellStyle {
        active_cell_theme().indicator_off
    }

    pub fn app_background() -> GameColor {
        active_cell_theme().body.bg
    }

    pub fn terminal_foreground() -> GameColor {
        active_cell_theme().body.fg
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_default_theme, apply_theme_key, catalog, current_theme_key};
    use nc_ui::GameColor;

    #[test]
    fn default_theme_key_is_tokyo_night() {
        apply_default_theme();
        assert_eq!(current_theme_key(), "tokyo-night");
    }

    #[test]
    fn legacy_theme_ids_normalize() {
        apply_theme_key("rose_pine").expect("theme loads");
        assert_eq!(current_theme_key(), "rose-pine");
    }

    #[test]
    fn catalog_contains_tokyo_night() {
        assert!(catalog().iter().any(|entry| entry.key == "tokyo-night"));
    }

    #[test]
    fn body_style_uses_active_theme() {
        apply_default_theme();
        assert_eq!(super::current_theme_key(), "tokyo-night");
        assert_ne!(super::body_style().bg, GameColor::Black);
    }
}
