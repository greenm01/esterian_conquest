//! Dashboard color theme — delegates to nc-ui theme system.

use nc_ui::table::TableRenderTheme;
use nc_ui::{CellStyle, GameColor};

pub fn body_style() -> CellStyle {
    nc_ui::theme::classic::body_style()
}

pub fn empire_slot_color(slot: u8) -> GameColor {
    nc_ui::theme::classic::empire_slot_color(slot)
}

pub fn empire_slot_style(slot: u8) -> CellStyle {
    nc_ui::theme::classic::empire_slot_style(slot)
}

pub fn empire_slot_style_on(slot: u8, bg: GameColor, bold: bool) -> CellStyle {
    nc_ui::theme::classic::empire_slot_style_on(slot, bg, bold)
}

pub fn border_style() -> CellStyle {
    CellStyle::new(GameColor::BrightBlack, GameColor::Black, false)
}

pub fn header_style() -> CellStyle {
    CellStyle::new(GameColor::BrightCyan, GameColor::Black, false)
}

pub fn footer_style() -> CellStyle {
    CellStyle::new(GameColor::Cyan, GameColor::Black, false)
}

pub fn title_style() -> CellStyle {
    CellStyle::new(GameColor::BrightWhite, GameColor::Black, true)
}

pub fn section_title_style() -> CellStyle {
    CellStyle::new(GameColor::BrightCyan, GameColor::Black, false)
}

pub fn label_style() -> CellStyle {
    CellStyle::new(GameColor::White, GameColor::Black, false)
}

pub fn value_style() -> CellStyle {
    CellStyle::new(GameColor::BrightWhite, GameColor::Black, false)
}

pub fn alert_style() -> CellStyle {
    CellStyle::new(GameColor::BrightYellow, GameColor::Black, true)
}

pub fn dim_style() -> CellStyle {
    CellStyle::new(GameColor::BrightBlack, GameColor::Black, false)
}

pub fn enemy_style() -> CellStyle {
    CellStyle::new(GameColor::Red, GameColor::Black, false)
}

pub fn friendly_style() -> CellStyle {
    CellStyle::new(GameColor::Green, GameColor::Black, false)
}

pub fn icd_style() -> CellStyle {
    CellStyle::new(GameColor::Yellow, GameColor::Black, false)
}

pub fn table_theme() -> TableRenderTheme {
    TableRenderTheme {
        header_style: section_title_style(),
        chrome_style: border_style(),
        body_style: value_style(),
        disabled_row_style: dim_style(),
        selected_style: alert_style(),
        scroll_track_style: dim_style(),
        scrollbar_thumb_style: title_style(),
    }
}
