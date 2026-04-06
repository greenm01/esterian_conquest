/// Keep `scroll_offset` in sync with `cursor` so the highlighted row is always visible.
pub(crate) fn sync_scroll_to_cursor(scroll_offset: &mut usize, cursor: usize, visible: usize) {
    if cursor < *scroll_offset {
        *scroll_offset = cursor;
    } else if cursor >= *scroll_offset + visible {
        *scroll_offset = cursor + 1 - visible;
    }
}

pub(crate) fn center_scroll_to_cursor(
    scroll_offset: &mut usize,
    cursor: usize,
    visible: usize,
    total: usize,
) {
    if total <= visible {
        *scroll_offset = 0;
        return;
    }
    let half = visible / 2;
    let max_offset = total - visible;
    *scroll_offset = cursor.saturating_sub(half).min(max_offset);
}

pub(crate) fn is_coordinate_input_char(ch: char) -> bool {
    nc_ui::table_selection::is_coordinate_input_char(ch)
}

pub(crate) fn resolve_default_coords_input(input: &str, default: [u8; 2]) -> Option<[u8; 2]> {
    if input.trim().is_empty() {
        Some(default)
    } else {
        crate::screen::parse_planet_coords(input)
    }
}
