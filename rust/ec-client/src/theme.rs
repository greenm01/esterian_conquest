pub mod classic {
    use crate::screen::{CellStyle, RgbColor};

    const BLACK: RgbColor = RgbColor::new(0, 0, 0);
    const DOS_BLUE: RgbColor = RgbColor::new(0, 0, 170);
    const LIGHT_GREY: RgbColor = RgbColor::new(224, 224, 224);
    const BODY_GREY: RgbColor = RgbColor::new(192, 192, 192);
    const YELLOW: RgbColor = RgbColor::new(255, 255, 85);
    const BRIGHT_WHITE: RgbColor = RgbColor::new(255, 255, 255);
    const DOS_RED: RgbColor = RgbColor::new(170, 0, 0);
    const DOS_GREEN: RgbColor = RgbColor::new(0, 170, 0);
    const AMBER_BG: RgbColor = RgbColor::new(170, 85, 0);
    const SELECTION_FG: RgbColor = RgbColor::new(20, 20, 20);
    const SELECTION_BLUE: RgbColor = RgbColor::new(65, 132, 192);

    pub const fn body_style() -> CellStyle {
        CellStyle::new(BODY_GREY, BLACK, false)
    }

    pub const fn title_style() -> CellStyle {
        CellStyle::new(BLACK, LIGHT_GREY, false)
    }

    pub const fn menu_style() -> CellStyle {
        CellStyle::new(LIGHT_GREY, DOS_BLUE, false)
    }

    pub const fn menu_hotkey_style() -> CellStyle {
        CellStyle::new(YELLOW, DOS_BLUE, true)
    }

    pub const fn prompt_style() -> CellStyle {
        CellStyle::new(BODY_GREY, BLACK, false)
    }

    pub const fn prompt_hotkey_style() -> CellStyle {
        CellStyle::new(YELLOW, BLACK, true)
    }

    pub const fn bright_style() -> CellStyle {
        CellStyle::new(BRIGHT_WHITE, BLACK, true)
    }

    pub const fn logo_style() -> CellStyle {
        CellStyle::new(YELLOW, BLACK, true)
    }

    pub const fn status_label_style() -> CellStyle {
        CellStyle::new(BODY_GREY, BLACK, false)
    }

    pub const fn status_value_style() -> CellStyle {
        CellStyle::new(BRIGHT_WHITE, BLACK, true)
    }

    /// Highlight style for the selected row in a navigable table.
    /// Override this function when adding new themes.
    pub const fn selected_row_style() -> CellStyle {
        CellStyle::new(SELECTION_FG, SELECTION_BLUE, false)
    }

    pub const fn alert_style() -> CellStyle {
        CellStyle::new(BRIGHT_WHITE, DOS_RED, true)
    }

    pub const fn help_header_style() -> CellStyle {
        CellStyle::new(BLACK, LIGHT_GREY, false)
    }

    pub const fn help_panel_style() -> CellStyle {
        CellStyle::new(YELLOW, AMBER_BG, false)
    }

    pub const fn map_dot_style() -> CellStyle {
        CellStyle::new(DOS_GREEN, BLACK, false)
    }

    pub const fn map_crosshair_style() -> CellStyle {
        CellStyle::new(DOS_RED, BLACK, true)
    }

    pub const fn map_center_style() -> CellStyle {
        CellStyle::new(BRIGHT_WHITE, BLACK, true)
    }
}
