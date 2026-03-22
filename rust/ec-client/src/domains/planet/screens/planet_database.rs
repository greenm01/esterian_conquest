use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::planet::PlanetAction;
use crate::screen::layout::{
    draw_command_line_default_input, draw_command_line_text, draw_command_prompt, draw_status_line,
    draw_title_bar, new_playfield,
};
use crate::screen::table::table_divider;
use crate::screen::{
    CommandMenu, PlayfieldBuffer, command_menu_label, format_sector_coords,
    format_sector_coords_padded,
};
use crate::theme::classic;

pub const PLANET_DATABASE_VISIBLE_ROWS: usize = 14;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDatabaseRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub name_label: String,
    pub owner_label: String,
    pub max_prod_label: String,
    pub year_seen_label: String,
    pub armies_label: String,
    pub batteries_label: String,
    pub current_prod_label: String,
    pub stored_points_label: String,
    pub year_scout_label: String,
    pub intel_label: String,
}

pub struct PlanetDatabaseScreen;

// Column widths for the 11-column layout (71 chars data + spaces + scroll indicator = 80).
// Coord(7) Planet(14) Own(3) Prod(4) Seen(5) ARs(3) GBs(3) Prod(4) Points(6) Scout(5) Intel(7)
use crate::screen::table::TableColumn;

const DATABASE_COLUMNS: [TableColumn<'static>; 11] = [
    TableColumn::left("Coord", 7),
    TableColumn::left("Planet", 14),
    TableColumn::right("Own", 3),
    TableColumn::right("Prod", 4),
    TableColumn::right("Seen", 5),
    TableColumn::right("ARs", 3),
    TableColumn::right("GBs", 3),
    TableColumn::right("Prod", 4),
    TableColumn::right("Points", 6),
    TableColumn::right("Scout", 5),
    TableColumn::left("Intel", 7),
];

impl PlanetDatabaseScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_list(
        &mut self,
        rows: &[PlanetDatabaseRow],
        scroll_offset: usize,
        cursor: usize,
        default_coords: [u8; 2],
        input: &str,
        status: Option<&str>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "TOTAL PLANET DATABASE:");

        // Two-line stacked header (rows 1-2), matching the original DOS layout.
        //
        // Row 1 (top labels):
        //   Coord(7) Planet(14) Own(3) | Max(4) Year(5)  | ARs(3) GBs(3) | Curr(4)  Stored(6) Year(5) Intel(7)
        // Row 2 (bottom labels):
        //   Coord    Planet      Own     Prod    Seen       ARs     GBs     Prod     Points     Scout   Intel
        let header_style = classic::status_value_style();
        buffer.write_text(
            1,
            0,
            //  7       14       3    4     5    3   3    4      6     5      7
            "                                Max  Year             Curr Stored  Year        ",
            header_style,
        );
        buffer.write_text(
            2,
            0,
            "Coord   Planet         Own Prod  Seen  ARs GBs Prod Points Scout Intel  ",
            header_style,
        );

        // Divider (row 3).
        buffer.write_text(
            3,
            0,
            &table_divider(&DATABASE_COLUMNS),
            classic::menu_style(),
        );

        // Data rows (row 4+, up to PLANET_DATABASE_VISIBLE_ROWS).
        for (idx, row) in rows
            .iter()
            .skip(scroll_offset)
            .take(PLANET_DATABASE_VISIBLE_ROWS)
            .enumerate()
        {
            let abs_idx = scroll_offset + idx;
            let style = if Some(abs_idx) == if rows.is_empty() { None } else { Some(cursor) } {
                classic::selected_row_style()
            } else {
                classic::status_value_style()
            };
            let line = format_data_row(row);
            buffer.write_text(4 + idx, 0, &line, style);
        }

        // Scroll indicator (right edge of data area).
        write_database_scroll_indicator(
            &mut buffer,
            4,
            PLANET_DATABASE_VISIBLE_ROWS,
            rows.len(),
            scroll_offset,
        );

        if rows.is_empty() {
            draw_command_line_text(
                &mut buffer,
                command_menu_label(menu),
                "No planets are in your database. Q quits.",
            );
        } else if let Some(status) = status {
            draw_command_line_text(&mut buffer, command_menu_label(menu), status);
        } else {
            draw_command_line_default_input(
                &mut buffer,
                command_menu_label(menu),
                "",
                &format!("{},{}", default_coords[0], default_coords[1]),
                input,
            );
        }
        Ok(buffer)
    }

    pub fn render_detail(
        &mut self,
        row: &PlanetDatabaseRow,
        selected_index: usize,
        total: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(
            &mut buffer,
            0,
            &format!("TOTAL PLANET DATABASE {}/{}:", selected_index + 1, total),
        );
        draw_status_line(
            &mut buffer,
            2,
            "Coordinates: ",
            &format_sector_coords(row.coords),
        );
        draw_status_line(&mut buffer, 3, "Planet Name: ", &row.name_label);
        draw_status_line(&mut buffer, 4, "Known Owner: ", &row.owner_label);
        draw_status_line(
            &mut buffer,
            6,
            "Potential Production: ",
            &row.max_prod_label,
        );
        draw_status_line(&mut buffer, 7, "Armies: ", &row.armies_label);
        draw_status_line(&mut buffer, 8, "Ground Batteries: ", &row.batteries_label);
        draw_status_line(&mut buffer, 10, "Last Intel: ", &row.year_seen_label);
        draw_status_line(&mut buffer, 11, "Known Intel: ", &row.intel_label);
        buffer.write_text(
            13,
            0,
            "Use arrows or HJKL to browse other known planets in the database.",
            classic::body_style(),
        );
        draw_command_prompt(&mut buffer, 19, "PLANET DATABASE", "ARROWS H J K L Q");
        Ok(buffer)
    }

    pub fn handle_list_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Planet(PlanetAction::MoveDatabaseList(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Planet(PlanetAction::MoveDatabaseList(1))
            }
            KeyCode::PageUp => Action::Planet(PlanetAction::MoveDatabaseList(-8)),
            KeyCode::PageDown => Action::Planet(PlanetAction::MoveDatabaseList(8)),
            KeyCode::Char(ch) if ch.is_ascii_digit() || ch == ',' || ch == ' ' => {
                Action::Planet(PlanetAction::AppendDatabaseChar(ch))
            }
            KeyCode::Backspace => Action::Planet(PlanetAction::BackspaceDatabaseInput),
            KeyCode::Enter => Action::Planet(PlanetAction::SubmitDatabaseLookup),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Action::ReturnToCommandMenu,
            _ => Action::Noop,
        }
    }

    pub fn handle_detail_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Left => Action::Planet(PlanetAction::MoveDatabaseDetail(-1)),
            KeyCode::Down | KeyCode::Right => Action::Planet(PlanetAction::MoveDatabaseDetail(1)),
            KeyCode::Home => Action::Planet(PlanetAction::MoveDatabaseDetail(i8::MIN)),
            KeyCode::End => Action::Planet(PlanetAction::MoveDatabaseDetail(i8::MAX)),
            KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('h') | KeyCode::Char('H') => {
                Action::Planet(PlanetAction::MoveDatabaseDetail(-1))
            }
            KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('l') | KeyCode::Char('L') => {
                Action::Planet(PlanetAction::MoveDatabaseDetail(1))
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Planet(PlanetAction::OpenDatabase)
            }
            _ => Action::Noop,
        }
    }
}

/// Format a single data row in the 11-column layout.
///
/// Coord(7) Planet(14) Own(3) Prod(4) Seen(5) ARs(3) GBs(3) Prod(4) Points(6) Scout(5) Intel(7)
fn format_data_row(row: &PlanetDatabaseRow) -> String {
    format!(
        "{:<7} {:<14} {:>3} {:>4} {:>5} {:>3} {:>3} {:>4} {:>6} {:>5} {:<7}",
        format_sector_coords_padded(row.coords),
        truncate(&row.name_label, 14),
        truncate(&row.owner_label, 3),
        truncate(&row.max_prod_label, 4),
        truncate(&row.year_seen_label, 5),
        truncate(&row.armies_label, 3),
        truncate(&row.batteries_label, 3),
        truncate(&row.current_prod_label, 4),
        truncate(&row.stored_points_label, 6),
        truncate(&row.year_scout_label, 5),
        truncate(&row.intel_label, 7),
    )
}

fn truncate(value: &str, max_width: usize) -> String {
    value.chars().take(max_width).collect::<String>()
}

fn write_database_scroll_indicator(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    visible_rows: usize,
    total_rows: usize,
    scroll_offset: usize,
) {
    if total_rows <= visible_rows || visible_rows == 0 || buffer.width() == 0 {
        return;
    }
    let displayed_rows = usize::min(visible_rows, total_rows.saturating_sub(scroll_offset));
    if displayed_rows < 3 {
        return;
    }
    let col = buffer.width() - 1;
    let last_row = start_row + displayed_rows - 1;
    let style = classic::status_value_style();
    let track_style = classic::menu_style();

    buffer.write_text(start_row, col, "^", style);
    buffer.write_text(last_row, col, "v", style);

    let track_top = start_row + 1;
    let track_bottom = last_row.saturating_sub(1);
    for r in track_top..=track_bottom {
        buffer.write_text(r, col, "|", track_style);
    }

    let max_offset = total_rows.saturating_sub(visible_rows);
    let thumb_span = track_bottom.saturating_sub(track_top);
    let thumb_row = if max_offset == 0 || thumb_span == 0 {
        track_top
    } else {
        track_top + (scroll_offset * thumb_span) / max_offset
    };
    buffer.write_text(thumb_row, col, "#", style);
}
