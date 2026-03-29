use crossterm::event::{KeyCode, KeyEvent};

use crate::app::Action;
use crate::domains::startup::StartupAction;
use crate::screen::layout::{
    ScreenGeometry, new_playfield_for, standard_table_visible_rows, standard_table_visible_rows_for,
};
use crate::screen::table::{
    HorizontalAlign, LayoutRect, TableColumn, TableFooter, TableWidthMode, VerticalAlign,
    draw_table_footer, draw_table_title, layout_standard_table_block,
    resolve_table_columns_for_widget, write_table_window_with_cursor_at,
};
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame};
use crate::theme::{ThemeEntry, ThemeEntryKind, classic};

pub const THEME_PICKER_VISIBLE_ROWS: usize = standard_table_visible_rows(2);

pub fn theme_picker_visible_rows(geometry: ScreenGeometry) -> usize {
    standard_table_visible_rows_for(geometry, 2)
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
        let visible_rows = theme_picker_visible_rows(geometry);
        let displayed_rows = table_rows
            .len()
            .saturating_sub(scroll_offset)
            .min(visible_rows);
        let scrollable = table_rows.len() > visible_rows;
        let default_theme = rows
            .get(cursor.min(rows.len().saturating_sub(1)))
            .map(|row| row.display_name.as_str());
        let footer = TableFooter::CommandBar {
            hotkeys_markup: "J K ^U ^D <Q>",
            default: default_theme,
            input,
        };
        let columns = resolve_table_columns_for_widget(
            &THEME_COLUMNS,
            &table_rows,
            buffer.width(),
            scrollable,
            TableWidthMode::Compact,
            Some("COLOR THEMES:"),
            Some(footer),
        );
        let layout = layout_standard_table_block(
            LayoutRect::new(0, 0, buffer.width(), buffer.height()),
            &columns,
            displayed_rows,
            Some("COLOR THEMES:"),
            Some(footer),
            scrollable,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        let _ = layout.title_row;
        draw_table_title(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            "COLOR THEMES:",
        );
        let metrics = write_table_window_with_cursor_at(
            &mut buffer,
            layout.table_row,
            layout.table_col,
            &columns,
            &table_rows,
            scroll_offset,
            visible_rows,
            classic::status_value_style(),
            classic::status_value_style(),
            if table_rows.is_empty() {
                None
            } else {
                Some(cursor.min(table_rows.len().saturating_sub(1)))
            },
            THEME_SELECTION_COL,
        );
        draw_table_footer(
            &mut buffer,
            geometry,
            layout.command_col,
            metrics.bottom_row,
            footer,
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
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                Action::Startup(StartupAction::MoveThemePicker(-1))
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                Action::Startup(StartupAction::MoveThemePicker(1))
            }
            KeyCode::PageUp => Action::Startup(StartupAction::MoveThemePicker(
                -(THEME_PICKER_VISIBLE_ROWS as isize),
            )),
            KeyCode::PageDown => Action::Startup(StartupAction::MoveThemePicker(
                THEME_PICKER_VISIBLE_ROWS as isize,
            )),
            KeyCode::Backspace => Action::Startup(StartupAction::BackspaceThemePickerInput),
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                Action::Startup(StartupAction::ExitThemePicker)
            }
            KeyCode::Char(ch) if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_') => {
                Action::Startup(StartupAction::AppendThemePickerChar(ch))
            }
            KeyCode::Enter => Action::Startup(StartupAction::ApplyThemePickerSelection),
            _ => Action::Noop,
        }
    }
}
