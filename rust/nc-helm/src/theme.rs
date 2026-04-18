use std::sync::OnceLock;

use opaline::{OpalineColor, OpalineStyle, Theme};

use crate::{CellStyle, GameColor};

const DEFAULT_THEME_KEY: &str = "tokyo-night";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HelmTheme {
    body: CellStyle,
    label: CellStyle,
    dim: CellStyle,
    accent: CellStyle,
    success: CellStyle,
    warning: CellStyle,
    error: CellStyle,
    root_border: CellStyle,
    root_title: CellStyle,
    panel: CellStyle,
    panel_label: CellStyle,
    panel_dim: CellStyle,
    panel_accent: CellStyle,
    panel_brand: CellStyle,
    panel_warning: CellStyle,
    panel_error: CellStyle,
    panel_border: CellStyle,
    prompt: CellStyle,
    prompt_hotkey: CellStyle,
    prompt_delimiter: CellStyle,
    selected_panel_row: CellStyle,
}

fn active_theme() -> &'static HelmTheme {
    static ACTIVE_THEME: OnceLock<HelmTheme> = OnceLock::new();
    ACTIVE_THEME.get_or_init(load_theme)
}

fn load_theme() -> HelmTheme {
    let theme = opaline::builtins::load_by_name(DEFAULT_THEME_KEY)
        .expect("bundled nc-helm theme should load");
    project_theme(&theme)
}

fn project_theme(theme: &Theme) -> HelmTheme {
    let body_fg = color(theme, "text.primary");
    let body_bg = color(theme, "bg.base");
    let panel_bg = color(theme, "bg.panel");
    let accent = color(theme, "accent.primary");
    let accent_alt = color(theme, "accent.secondary");
    let success = color(theme, "success");
    let warning = color(theme, "warning");
    let error = color(theme, "error");
    let label = color(theme, "text.secondary");
    let dim = color(theme, "text.dim");
    let chrome = color(theme, "border.unfocused");

    HelmTheme {
        body: CellStyle::new(body_fg, body_bg, false),
        label: CellStyle::new(label, body_bg, false),
        dim: style_on_bg(theme.style("dimmed"), dim, body_bg),
        accent: CellStyle::new(accent, body_bg, true),
        success: style_on_bg(theme.style("success_style"), success, body_bg),
        warning: style_on_bg(theme.style("warning_style"), warning, body_bg),
        error: style_on_bg(theme.style("error_style"), error, body_bg),
        root_border: CellStyle::new(chrome, body_bg, false),
        root_title: CellStyle::new(accent, body_bg, true),
        panel: CellStyle::new(body_fg, panel_bg, false),
        panel_label: CellStyle::new(label, panel_bg, false),
        panel_dim: style_on_bg(theme.style("dimmed"), dim, panel_bg),
        panel_accent: CellStyle::new(accent, panel_bg, true),
        panel_brand: CellStyle::new(accent, panel_bg, false),
        panel_warning: style_on_bg(theme.style("warning_style"), warning, panel_bg),
        panel_error: style_on_bg(theme.style("error_style"), error, panel_bg),
        panel_border: CellStyle::new(chrome, panel_bg, false),
        prompt: CellStyle::new(body_fg, body_bg, false),
        prompt_hotkey: CellStyle::new(accent_alt, body_bg, true),
        prompt_delimiter: CellStyle::new(label, body_bg, false),
        selected_panel_row: projected_style(theme.style("selected"), body_fg, panel_bg),
    }
}

fn color(theme: &Theme, token: &str) -> GameColor {
    to_game_color(theme.color(token))
}

fn to_game_color(color: OpalineColor) -> GameColor {
    GameColor::Rgb(color.r, color.g, color.b)
}

fn style_on_bg(style: OpalineStyle, default_fg: GameColor, bg: GameColor) -> CellStyle {
    projected_style(style, default_fg, bg)
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

fn with_bg(style: CellStyle, bg: GameColor) -> CellStyle {
    CellStyle::new(style.fg, bg, style.bold)
}

#[cfg(test)]
pub fn default_theme_key() -> &'static str {
    DEFAULT_THEME_KEY
}

pub fn app_background() -> GameColor {
    active_theme().body.bg
}

pub fn body_style() -> CellStyle {
    active_theme().body
}

pub fn label_style() -> CellStyle {
    active_theme().label
}

pub fn dim_style() -> CellStyle {
    active_theme().dim
}

pub fn accent_style() -> CellStyle {
    active_theme().accent
}

pub fn warning_style() -> CellStyle {
    active_theme().warning
}

pub fn error_style() -> CellStyle {
    active_theme().error
}

pub fn root_border_style() -> CellStyle {
    active_theme().root_border
}

pub fn root_title_style() -> CellStyle {
    active_theme().root_title
}

pub fn panel_style() -> CellStyle {
    active_theme().panel
}

pub fn panel_dim_style() -> CellStyle {
    active_theme().panel_dim
}

pub fn panel_accent_style() -> CellStyle {
    active_theme().panel_accent
}

pub fn panel_brand_style() -> CellStyle {
    active_theme().panel_brand
}

pub fn panel_warning_style() -> CellStyle {
    active_theme().panel_warning
}

pub fn panel_error_style() -> CellStyle {
    active_theme().panel_error
}

pub fn panel_border_style() -> CellStyle {
    active_theme().panel_border
}

pub fn selected_panel_row_style() -> CellStyle {
    active_theme().selected_panel_row
}

#[cfg(test)]
pub fn title_style_on(bg: GameColor) -> CellStyle {
    with_bg(active_theme().root_title, bg)
}

pub fn prompt_style_on(bg: GameColor) -> CellStyle {
    with_bg(active_theme().prompt, bg)
}

pub fn prompt_hotkey_style_on(bg: GameColor) -> CellStyle {
    with_bg(active_theme().prompt_hotkey, bg)
}

pub fn prompt_delimiter_style_on(bg: GameColor) -> CellStyle {
    with_bg(active_theme().prompt_delimiter, bg)
}

pub fn idle_network_color() -> GameColor {
    active_theme().dim.fg
}

pub fn connecting_network_color() -> GameColor {
    active_theme().warning.fg
}

pub fn synced_network_color() -> GameColor {
    active_theme().success.fg
}

pub fn error_network_color() -> GameColor {
    active_theme().error.fg
}

#[cfg(test)]
mod tests {
    use super::{
        app_background, body_style, default_theme_key, panel_style, prompt_hotkey_style_on,
        prompt_style_on, title_style_on,
    };

    #[test]
    fn default_theme_key_is_tokyo_night() {
        assert_eq!(default_theme_key(), "tokyo-night");
    }

    #[test]
    fn body_and_panel_backgrounds_come_from_the_theme() {
        assert_eq!(app_background(), body_style().bg);
        assert_ne!(body_style().bg, panel_style().bg);
    }

    #[test]
    fn command_styles_distinguish_label_and_hotkey() {
        let bg = panel_style().bg;
        assert_ne!(title_style_on(bg).fg, prompt_hotkey_style_on(bg).fg);
        assert_ne!(prompt_style_on(bg).fg, prompt_hotkey_style_on(bg).fg);
    }
}
