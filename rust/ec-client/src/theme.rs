pub mod classic {
    use crate::screen::{CellStyle, RgbColor};

    const BLACK: RgbColor = RgbColor::new(0, 0, 0);
    const DOS_BLUE: RgbColor = RgbColor::new(0, 0, 170);
    const LIGHT_GREY: RgbColor = RgbColor::new(224, 224, 224);
    const BODY_GREY: RgbColor = RgbColor::new(188, 192, 204);
    const YELLOW: RgbColor = RgbColor::new(255, 255, 85);
    const BRIGHT_WHITE: RgbColor = RgbColor::new(255, 255, 255);
    const DOS_RED: RgbColor = RgbColor::new(170, 0, 0);
    const CODEX_MAGENTA: RgbColor = RgbColor::new(215, 135, 255);
    const DOS_GREEN: RgbColor = RgbColor::new(0, 170, 0);
    const AMBER_BG: RgbColor = RgbColor::new(170, 85, 0);
    const SELECTION_FG: RgbColor = RgbColor::new(20, 20, 20);
    const SELECTION_BLUE: RgbColor = RgbColor::new(65, 132, 192);

    // Tokyo Night accent palette.
    const TOKYONIGHT_BLUE: RgbColor = RgbColor::new(122, 162, 247);
    const TOKYONIGHT_PURPLE: RgbColor = RgbColor::new(187, 154, 247);
    const TOKYONIGHT_ORANGE: RgbColor = RgbColor::new(255, 158, 100);
    const TOKYONIGHT_TEAL: RgbColor = RgbColor::new(125, 207, 207);
    const TOKYONIGHT_GOLD: RgbColor = RgbColor::new(224, 175, 104);

    // Stellar spectral colors for logo decoration randomization.
    pub const STAR_COLORS: [RgbColor; 6] = [
        RgbColor::new(180, 200, 255), // hot blue-white
        RgbColor::new(255, 255, 255), // white
        RgbColor::new(255, 255, 200), // yellow-white
        RgbColor::new(255, 230, 130), // yellow / Sol
        RgbColor::new(255, 180, 100), // orange / K-type
        RgbColor::new(255, 140, 100), // red-orange / cool
    ];

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

    pub const fn prompt_notice_action_style() -> CellStyle {
        CellStyle::new(CODEX_MAGENTA, BLACK, true)
    }

    pub const fn bright_style() -> CellStyle {
        CellStyle::new(BRIGHT_WHITE, BLACK, true)
    }

    pub const fn logo_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BLUE, BLACK, true)
    }

    pub const fn intro_accent_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BLUE, BLACK, false)
    }

    pub const fn intro_tribute_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_PURPLE, BLACK, false)
    }

    /// Stardate label ("Stardate: ") and slash separator.
    pub const fn stardate_label_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_ORANGE, BLACK, false)
    }

    /// Stardate week number.
    pub const fn stardate_week_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEAL, BLACK, false)
    }

    /// Stardate year number.
    pub const fn stardate_year_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_GOLD, BLACK, false)
    }

    pub fn star_decoration_style(index: usize) -> CellStyle {
        CellStyle::new(STAR_COLORS[index % STAR_COLORS.len()], BLACK, false)
    }

    pub const fn status_label_style() -> CellStyle {
        CellStyle::new(BODY_GREY, BLACK, false)
    }

    pub const fn status_value_style() -> CellStyle {
        CellStyle::new(BRIGHT_WHITE, BLACK, true)
    }

    pub const fn disabled_row_style() -> CellStyle {
        CellStyle::new(BODY_GREY, BLACK, false)
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
