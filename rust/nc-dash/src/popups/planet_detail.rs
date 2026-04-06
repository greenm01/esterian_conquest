//! Planet info popup rendered over the center map pane.

use nc_ui::PlayfieldBuffer;
use nc_ui::modal::{Rect, centered_rect, draw_box};
use nc_ui::table::{TableFooter, draw_table_footer_in_span, table_footer_scaffold_width};

use crate::app::state::DashApp;
use crate::layout::{self, MapWidgetFrame};
use crate::planet_view::selected_planet_detail;
use crate::theme;

pub fn draw(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: MapWidgetFrame,
    _planet_record_index_1_based: usize,
) {
    let Some(detail) = selected_planet_detail(app) else {
        return;
    };

    let lines = popup_lines(&detail.popup_lines);
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let footer = TableFooter::CommandPrompt {
        label: "COMMAND",
        prompt: "Enter, Esc, or <Q> to close ->",
    };
    let popup = draw_center_map_popup_frame(
        buf,
        map_frame,
        "INFO ABOUT A PLANET:",
        body_width,
        lines.len(),
        footer,
    );
    for (idx, line) in lines.into_iter().enumerate().take(popup.body_height) {
        layout::write_clipped(
            buf,
            popup.body_row + idx,
            popup.body_col,
            popup.body_width,
            &line,
            theme::value_style(),
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct PopupFrame {
    body_col: usize,
    body_row: usize,
    body_width: usize,
    body_height: usize,
}

fn draw_center_map_popup_frame(
    buf: &mut PlayfieldBuffer,
    map_frame: MapWidgetFrame,
    title: &str,
    body_width: usize,
    body_height: usize,
    footer: TableFooter<'_>,
) -> PopupFrame {
    let parent = Rect::new(
        (map_frame.outer.col + 1) as u16,
        (map_frame.outer.row + 1) as u16,
        map_frame.outer.width.saturating_sub(2) as u16,
        map_frame.outer.height.saturating_sub(2) as u16,
    );
    let preferred_width =
        (body_width.max(table_footer_scaffold_width(footer)) + 4).max(title.chars().count() + 6);
    let preferred_height = (body_height + 4) as u16;
    let popup = centered_rect(preferred_width as u16, preferred_height, parent);
    draw_box(
        buf,
        popup,
        title,
        theme::border_style(),
        theme::title_style(),
    );
    buf.fill_rect(
        popup.y as usize + 1,
        popup.x as usize + 1,
        popup.width.saturating_sub(2) as usize,
        popup.height.saturating_sub(2) as usize,
        theme::body_style(),
    );

    let inner_left = popup.x as usize + 1;
    let inner_right = popup.x as usize + popup.width as usize - 2;
    let footer_row = popup.y as usize + popup.height as usize - 2;
    let divider_row = footer_row.saturating_sub(1);
    for col in inner_left..=inner_right {
        buf.set_cell(divider_row, col, '─', theme::border_style());
    }
    buf.set_cell(
        divider_row,
        inner_left.saturating_sub(1),
        '├',
        theme::border_style(),
    );
    buf.set_cell(divider_row, inner_right + 1, '┤', theme::border_style());
    draw_table_footer_in_span(
        buf,
        footer_row,
        popup.x as usize + 2,
        popup.width.saturating_sub(4) as usize,
        footer,
    );

    PopupFrame {
        body_col: popup.x as usize + 2,
        body_row: popup.y as usize + 1,
        body_width: popup.width.saturating_sub(4) as usize,
        body_height: divider_row.saturating_sub(popup.y as usize + 1),
    }
}

fn popup_lines(lines: &[crate::planet_view::DetailLine]) -> Vec<String> {
    let label_width = lines
        .iter()
        .map(|line| line.label.chars().count())
        .max()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| format!("{:<label_width$} : {}", line.label, line.value))
        .collect()
}
