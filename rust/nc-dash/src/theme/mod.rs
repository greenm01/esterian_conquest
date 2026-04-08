//! Dashboard color theme — delegates to nc-ui theme system.

use nc_ui::table::TableRenderTheme;
use nc_ui::theme::classic;
use nc_ui::{CellStyle, GameColor};

fn on_body(style: CellStyle) -> CellStyle {
    CellStyle::new(style.fg, body_style().bg, style.bold)
}

pub fn body_style() -> CellStyle {
    classic::body_style()
}

pub fn empire_slot_style(slot: u8) -> CellStyle {
    classic::empire_slot_style(slot)
}

pub fn empire_slot_style_on(slot: u8, bg: GameColor, bold: bool) -> CellStyle {
    classic::empire_slot_style_on(slot, bg, bold)
}

pub fn border_style() -> CellStyle {
    classic::table_chrome_style()
}

pub fn header_style() -> CellStyle {
    classic::shell_label_style()
}

pub fn title_style() -> CellStyle {
    classic::shell_title_style()
}

pub fn section_title_style() -> CellStyle {
    classic::table_header_style()
}

pub fn label_style() -> CellStyle {
    classic::status_label_style()
}

pub fn value_style() -> CellStyle {
    classic::status_value_style()
}

pub fn alert_style() -> CellStyle {
    on_body(classic::alert_style())
}

pub fn error_style() -> CellStyle {
    on_body(classic::error_style())
}

pub fn dim_style() -> CellStyle {
    classic::disabled_row_style()
}

pub fn enemy_style() -> CellStyle {
    on_body(classic::error_style())
}

pub fn friendly_style() -> CellStyle {
    on_body(classic::notice_style())
}

pub fn icd_style() -> CellStyle {
    on_body(classic::alert_style())
}

pub fn map_crosshair_style() -> CellStyle {
    classic::map_crosshair_style()
}

pub fn map_center_style() -> CellStyle {
    classic::map_center_style()
}

pub fn map_fleet_marker_style() -> CellStyle {
    on_body(classic::map_dot_style())
}

pub fn map_fleet_marker_style_on(bg: GameColor, bold: bool) -> CellStyle {
    let style = classic::map_dot_style();
    CellStyle::new(style.fg, bg, bold)
}

pub fn table_theme() -> TableRenderTheme {
    TableRenderTheme::classic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_default_styles_use_active_theme_background() {
        let expected_bg = body_style().bg;
        let styles = [
            border_style(),
            header_style(),
            classic::prompt_style(),
            title_style(),
            section_title_style(),
            label_style(),
            value_style(),
            alert_style(),
            dim_style(),
            enemy_style(),
            friendly_style(),
            icd_style(),
            map_fleet_marker_style(),
        ];

        for style in styles {
            assert_eq!(style.bg, expected_bg);
        }
    }

    #[test]
    fn fleet_marker_style_uses_map_dot_green_source() {
        let map_dot = classic::map_dot_style();
        let fleet_marker = map_fleet_marker_style();

        assert_eq!(fleet_marker.fg, map_dot.fg);
        assert_eq!(fleet_marker.bg, body_style().bg);
    }
}
