pub mod classic {
    use crate::screen::{CellStyle, RgbColor};

    const TOKYONIGHT_BG: RgbColor = RgbColor::new(26, 27, 38);
    const TOKYONIGHT_CHROME: RgbColor = RgbColor::new(86, 95, 137);
    const TOKYONIGHT_BODY: RgbColor = RgbColor::new(169, 177, 214);
    const TOKYONIGHT_TEXT: RgbColor = RgbColor::new(192, 202, 245);
    const TOKYONIGHT_YELLOW: RgbColor = RgbColor::new(224, 175, 104);
    const TOKYONIGHT_RED: RgbColor = RgbColor::new(247, 118, 142);
    const TOKYONIGHT_GREEN: RgbColor = RgbColor::new(158, 206, 106);
    const CODEX_MAGENTA: RgbColor = RgbColor::new(215, 135, 255);
    const SELECTION_FG: RgbColor = TOKYONIGHT_BG;
    const SELECTION_BLUE: RgbColor = RgbColor::new(122, 162, 247);

    // Tokyo Night accent palette.
    const TOKYONIGHT_BLUE: RgbColor = RgbColor::new(122, 162, 247);
    const TOKYONIGHT_PURPLE: RgbColor = RgbColor::new(187, 154, 247);
    const TOKYONIGHT_ORANGE: RgbColor = RgbColor::new(255, 158, 100);
    const TOKYONIGHT_TEAL: RgbColor = RgbColor::new(125, 207, 207);
    const TOKYONIGHT_GOLD: RgbColor = TOKYONIGHT_YELLOW;

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
        CellStyle::new(TOKYONIGHT_BODY, TOKYONIGHT_BG, false)
    }

    pub const fn title_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEXT, TOKYONIGHT_BG, true)
    }

    pub const fn menu_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BODY, TOKYONIGHT_BG, false)
    }

    pub const fn menu_hotkey_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_YELLOW, TOKYONIGHT_BG, true)
    }

    pub const fn prompt_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BODY, TOKYONIGHT_BG, false)
    }

    pub const fn prompt_hotkey_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_YELLOW, TOKYONIGHT_BG, true)
    }

    pub const fn prompt_notice_action_style() -> CellStyle {
        CellStyle::new(CODEX_MAGENTA, TOKYONIGHT_BG, true)
    }

    pub const fn bright_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEXT, TOKYONIGHT_BG, true)
    }

    pub const fn logo_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BLUE, TOKYONIGHT_BG, true)
    }

    pub const fn intro_accent_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BLUE, TOKYONIGHT_BG, false)
    }

    pub const fn intro_tribute_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_PURPLE, TOKYONIGHT_BG, false)
    }

    /// Stardate label ("Stardate: ") and slash separator.
    pub const fn stardate_label_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_ORANGE, TOKYONIGHT_BG, false)
    }

    /// Stardate week number.
    pub const fn stardate_week_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEAL, TOKYONIGHT_BG, false)
    }

    /// Stardate year number.
    pub const fn stardate_year_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_GOLD, TOKYONIGHT_BG, false)
    }

    pub fn star_decoration_style(index: usize) -> CellStyle {
        CellStyle::new(STAR_COLORS[index % STAR_COLORS.len()], TOKYONIGHT_BG, false)
    }

    pub const fn status_label_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BODY, TOKYONIGHT_BG, false)
    }

    pub const fn status_value_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEXT, TOKYONIGHT_BG, false)
    }

    pub const fn table_chrome_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_CHROME, TOKYONIGHT_BG, false)
    }

    pub const fn table_header_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BODY, TOKYONIGHT_BG, false)
    }

    pub const fn table_body_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEXT, TOKYONIGHT_BG, false)
    }

    pub const fn disabled_row_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_CHROME, TOKYONIGHT_BG, false)
    }

    /// Highlight style for the selected row in a navigable table.
    /// Override this function when adding new themes.
    pub const fn selected_row_style() -> CellStyle {
        CellStyle::new(SELECTION_FG, SELECTION_BLUE, false)
    }

    pub const fn alert_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEXT, TOKYONIGHT_RED, true)
    }

    pub const fn help_header_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEXT, TOKYONIGHT_BG, true)
    }

    pub const fn help_panel_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_BODY, TOKYONIGHT_BG, false)
    }

    pub const fn map_dot_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_GREEN, TOKYONIGHT_BG, false)
    }

    pub const fn map_crosshair_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_RED, TOKYONIGHT_BG, true)
    }

    pub const fn map_center_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEXT, TOKYONIGHT_BG, true)
    }

    // Quote display styles — understated so they don't compete with the menu.
    const QUOTE_GREY: RgbColor = RgbColor::new(158, 162, 174);
    const QUOTE_AUTHOR_GREY: RgbColor = RgbColor::new(178, 182, 194);

    /// Dim body grey for quote text — slightly muted relative to BODY_GREY.
    pub const fn quote_style() -> CellStyle {
        CellStyle::new(QUOTE_GREY, TOKYONIGHT_BG, false)
    }

    /// Slightly brighter grey for the quote author attribution line.
    pub const fn quote_author_style() -> CellStyle {
        CellStyle::new(QUOTE_AUTHOR_GREY, TOKYONIGHT_BG, false)
    }

    /// Report header lines ("From your Xth Fleet...") — teal accent.
    pub const fn report_header_style() -> CellStyle {
        CellStyle::new(TOKYONIGHT_TEAL, TOKYONIGHT_BG, false)
    }

    pub const fn app_background() -> RgbColor {
        TOKYONIGHT_BG
    }

    pub const fn terminal_foreground() -> RgbColor {
        TOKYONIGHT_BODY
    }
}
