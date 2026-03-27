use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::startup::StartupAction;
use crate::screen::layout::{
    ScreenGeometry, draw_status_line, draw_table_command_bar_at, draw_title_bar,
    new_playfield_for, standard_table_visible_rows, standard_table_visible_rows_for,
    table_prompt_row_for,
};
use crate::screen::table::{TableColumn, write_table_window_with_cursor};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::{ThemeEntry, ThemeEntryKind, classic};

pub const THEME_PICKER_VISIBLE_ROWS: usize = standard_table_visible_rows(4);

pub fn theme_picker_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 4)
}

const THEME_COLUMNS: [TableColumn<'static>; 3] = [
    TableColumn::center("", 1),
    TableColumn::left("Theme", 22),
    TableColumn::left("Type", 8),
];
const THEME_SELECTION_COL: usize = 1;

pub struct ThemePickerScreen;

impl ThemePickerScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(
        &mut self,
        geometry: ScreenGeometry,
        rows: &[ThemeEntry],
        scroll_offset: usize,
        cursor: usize,
        active_key: Option<&str>,
        input: &str,
        _status: Option<&str>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield_for(geometry);
        draw_title_bar(&mut buffer, 0, "ANSI THEMES:");
        draw_status_line(
            &mut buffer,
            1,
            "",
            "Use arrows or J/K to move. ENTER applies the theme. Q returns.",
        );
        let table_rows = rows
            .iter()
            .map(|row| {
                vec![
                    if active_key == Some(row.key.as_str()) {
                        "*".to_string()
                    } else {
                        String::new()
                    },
                    row.display_name.clone(),
                    match row.kind {
                        ThemeEntryKind::Theme => "Theme".to_string(),
                        ThemeEntryKind::Mono => "Mono".to_string(),
                    },
                ]
            })
            .collect::<Vec<_>>();
        let metrics = write_table_window_with_cursor(
            &mut buffer,
            3,
            &THEME_COLUMNS,
            &table_rows,
            scroll_offset,
            theme_picker_visible_rows(geometry),
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor.min(table_rows.len().saturating_sub(1)))
            },
            THEME_SELECTION_COL,
        );
        let command_row = table_prompt_row_for(geometry, metrics.bottom_row);
        let default_theme = rows
            .get(cursor.min(rows.len().saturating_sub(1)))
            .map(|row| row.display_name.as_str());
        draw_table_command_bar_at(
            &mut buffer,
            command_row,
            "<ARROWS PGUP PGDN ENTER Q>",
            default_theme,
            input,
        );
        Ok(buffer)
    }
}

impl Screen for ThemePickerScreen {
    fn render(
        &mut self,
        _frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render(ScreenGeometry::local_default(), &[], 0, 0, None, "", None)
    }

    fn handle_key(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up => Action::Startup(StartupAction::MoveThemePicker(-1)),
            KeyCode::Down => Action::Startup(StartupAction::MoveThemePicker(1)),
            KeyCode::PageUp => Action::Startup(StartupAction::MoveThemePicker(
                -(THEME_PICKER_VISIBLE_ROWS as isize),
            )),
            KeyCode::PageDown => Action::Startup(StartupAction::MoveThemePicker(
                THEME_PICKER_VISIBLE_ROWS as isize,
            )),
            KeyCode::Backspace => Action::Startup(StartupAction::BackspaceThemePickerInput),
            KeyCode::Char(ch) if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_') => {
                Action::Startup(StartupAction::AppendThemePickerChar(ch))
            }
            KeyCode::Enter => Action::Startup(StartupAction::ApplyThemePickerSelection),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Startup(StartupAction::ExitThemePicker)
            }
            _ => Action::Noop,
        }
    }
}
