use crate::screen::PlayfieldBuffer;
use crate::screen::layout::{
    ScreenGeometry, draw_command_line_default_input_at_col, draw_command_line_prompt_input_at_col,
    draw_command_line_prompt_text_at_col, draw_command_line_text_at_col,
    draw_dismiss_prompt_at_col, draw_table_command_bar_at_col, draw_table_command_prompt_at_col,
    table_dismiss_prompt_row_for, table_prompt_row_for,
};
use crate::theme::classic;

pub use nc_ui::table::{
    HorizontalAlign, LayoutRect, SplitTableRow, TABLE_TEXT_INSET, TableAlign, TableBlockLayout,
    TableColumn, TableFooter, TableRenderMetrics, TableRenderTheme, TableRowState, TableWidthMode,
    VerticalAlign, centered_table_start_col, fit_table_columns, fit_table_columns_for_widget,
    fit_table_columns_for_widget_with_footer_floor, layout_stacked_table_block,
    layout_standard_table_block, minimum_table_render_width,
    minimum_table_render_width_with_footer_floor, resolve_table_columns,
    resolve_table_columns_for_widget, resolve_table_columns_for_widget_with_footer_floor,
    table_column_start, table_divider, table_footer_scaffold_width, table_footer_width,
    table_render_width, widen_table_columns_to_minimum_render_width, write_split_table,
    write_split_table_at, write_stacked_table_window_with_states,
    write_stacked_table_window_with_states_at, write_table_header, write_table_row,
    write_table_window, write_table_window_with_cursor, write_table_window_with_cursor_at,
    write_table_window_with_states, write_table_window_with_states_at,
};

pub fn draw_table_footer(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    table_col: usize,
    bottom_row: usize,
    footer: TableFooter<'_>,
) -> usize {
    draw_single_table_footer(
        buffer,
        geometry,
        table_col,
        bottom_row,
        table_prompt_row_for(geometry, bottom_row),
        footer,
    )
}

fn draw_single_table_footer(
    buffer: &mut PlayfieldBuffer,
    geometry: ScreenGeometry,
    table_col: usize,
    bottom_row: usize,
    row: usize,
    footer: TableFooter<'_>,
) -> usize {
    match footer {
        TableFooter::Dismiss => {
            let dismiss_row = table_dismiss_prompt_row_for(geometry, bottom_row);
            draw_dismiss_prompt_at_col(buffer, dismiss_row, table_col);
            dismiss_row
        }
        TableFooter::CommandBar {
            hotkeys_markup,
            default,
            input,
        } => {
            draw_table_command_bar_at_col(buffer, row, table_col, hotkeys_markup, default, input);
            row
        }
        TableFooter::LabeledCommandBar {
            label,
            hotkeys_markup,
            default,
            input,
        } => {
            crate::screen::layout::draw_labeled_table_command_bar_at_col(
                buffer,
                row,
                table_col,
                label,
                hotkeys_markup,
                default,
                input,
            );
            row
        }
        TableFooter::CommandText { label, text } => {
            draw_command_line_text_at_col(buffer, row, table_col, label, text);
            row
        }
        TableFooter::CommandPrompt { label, prompt } => {
            draw_command_line_prompt_text_at_col(buffer, row, table_col, label, prompt);
            row
        }
        TableFooter::CommandPromptInput {
            label,
            prompt,
            input,
        } => {
            draw_command_line_prompt_input_at_col(buffer, row, table_col, label, prompt, input);
            row
        }
        TableFooter::CommandInput {
            label,
            prompt,
            default,
            input,
        } => {
            draw_command_line_default_input_at_col(
                buffer, row, table_col, label, prompt, default, input,
            );
            row
        }
        TableFooter::TablePrompt(prompt) => {
            draw_table_command_prompt_at_col(buffer, row, table_col, prompt);
            row
        }
        TableFooter::Stacked { rows, active_row } => {
            if rows.is_empty() {
                return row;
            }
            let active_row = active_row.min(rows.len().saturating_sub(1));
            let last_row = table_prompt_row_for(geometry, bottom_row);
            let first_row = last_row.saturating_sub(rows.len().saturating_sub(1));
            for (idx, footer_row) in rows.iter().enumerate() {
                if idx == active_row {
                    continue;
                }
                draw_single_table_footer(
                    buffer,
                    geometry,
                    table_col,
                    bottom_row,
                    first_row + idx,
                    *footer_row,
                );
            }
            draw_single_table_footer(
                buffer,
                geometry,
                table_col,
                bottom_row,
                first_row + active_row,
                rows[active_row],
            )
        }
    }
}

pub fn draw_table_title_at(
    buffer: &mut PlayfieldBuffer,
    title_row: usize,
    table_col: usize,
    title: &str,
) -> usize {
    nc_ui::table::draw_table_title_at(
        buffer,
        title_row,
        table_col,
        title,
        classic::menu_style(),
        classic::title_style(),
    )
}

pub fn draw_table_title(
    buffer: &mut PlayfieldBuffer,
    table_row: usize,
    table_col: usize,
    title: &str,
) -> usize {
    nc_ui::table::draw_table_title(
        buffer,
        table_row,
        table_col,
        title,
        classic::menu_style(),
        classic::title_style(),
    )
}

pub fn format_empire_id(empire_id: u8) -> String {
    format!("{empire_id:02}")
}

pub fn fleet_number_width(max_fleet_number: u16) -> usize {
    max_fleet_number.max(1).to_string().chars().count()
}

pub fn fleet_id_column_width(max_fleet_number: u16) -> usize {
    "ID".len().max(fleet_number_width(max_fleet_number))
}

pub fn format_fleet_number(fleet_number: u16, max_fleet_number: u16) -> String {
    let width = fleet_number_width(max_fleet_number);
    format!("{fleet_number:0width$}")
}
