//! Planet info popup rendered over the center map pane.

use nc_ui::PlayfieldBuffer;
use nc_ui::modal::{ModalTheme, Rect, draw_modal_frame_in_parent};
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

    let max_body_width = map_frame.outer.width.saturating_sub(6);
    let lines = popup_lines(&detail.popup_lines, max_body_width);
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let footer = TableFooter::Dismiss;
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
    let popup = draw_modal_frame_in_parent(
        buf,
        title,
        preferred_width,
        preferred_height,
        parent,
        ModalTheme {
            body_style: theme::body_style(),
            pad_style: theme::dim_style(),
            chrome_style: theme::border_style(),
            title_style: theme::title_style(),
        },
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

fn popup_lines(lines: &[crate::planet_view::DetailLine], max_body_width: usize) -> Vec<String> {
    let label_width = layout::label_value_width(lines.iter().map(|line| line.label));
    let prefix_width = label_width + " : ".chars().count();
    let value_width = max_body_width.saturating_sub(prefix_width).max(1);
    let continuation_label = " ".repeat(label_width);
    let mut rendered = Vec::new();

    for line in lines {
        let wrapped = wrap_value(&line.value, value_width);
        if wrapped.is_empty() {
            rendered.push(layout::format_label_value(line.label, label_width, ""));
            continue;
        }
        for (idx, segment) in wrapped.into_iter().enumerate() {
            let label = if idx == 0 {
                line.label
            } else {
                continuation_label.as_str()
            };
            rendered.push(layout::format_label_value(label, label_width, &segment));
        }
    }

    rendered
}

fn wrap_value(value: &str, width: usize) -> Vec<String> {
    if value.is_empty() {
        return vec![String::new()];
    }
    if width == 0 {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in value.split_whitespace() {
        let word_width = word.chars().count();
        if current.is_empty() {
            if word_width <= width {
                current.push_str(word);
            } else {
                lines.extend(chunk_word(word, width));
            }
            continue;
        }

        let candidate_width = current.chars().count() + 1 + word_width;
        if candidate_width <= width {
            current.push(' ');
            current.push_str(word);
            continue;
        }

        lines.push(current);
        current = String::new();
        if word_width <= width {
            current.push_str(word);
        } else {
            let mut chunks = chunk_word(word, width);
            current = chunks.pop().unwrap_or_default();
            lines.extend(chunks);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

fn chunk_word(word: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if current.chars().count() == width {
            chunks.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::{draw_center_map_popup_frame, popup_lines};
    use crate::layout::widgets::WidgetRect;
    use crate::theme;
    use crate::planet_view::DetailLine;
    use nc_ui::PlayfieldBuffer;
    use nc_ui::table::TableFooter;

    #[test]
    fn popup_lines_wrap_long_values_with_aligned_colon_continuations() {
        let lines = popup_lines(
            &[DetailLine {
                label: "Status",
                value: String::from("Regular planet - factories fully functional"),
            }],
            36,
        );

        assert!(lines.len() > 1);
        assert_eq!(lines[0].find(" : "), lines[1].find(" : "));
        assert!(lines[0].contains("Status"));
        assert!(lines[1].starts_with("       : "));
    }

    #[test]
    fn center_map_popup_keeps_visible_padding_inside_map_frame() {
        let mut buffer = PlayfieldBuffer::new(120, 40, theme::body_style());
        let map_frame = crate::layout::MapWidgetFrame {
            outer: WidgetRect {
                col: 20,
                row: 5,
                width: 60,
                height: 20,
            },
            axis_row: 6,
            grid: WidgetRect {
                col: 22,
                row: 7,
                width: 57,
                height: 18,
            },
            bottom_pad_row: 25,
            row_label_cols: 3,
            cell_width: 3,
        };

        let popup = draw_center_map_popup_frame(
            &mut buffer,
            map_frame,
            "INFO ABOUT A PLANET:",
            30,
            8,
            TableFooter::Dismiss,
        );

        assert!(popup.body_col > map_frame.outer.col + 2);
        assert!(popup.body_row > map_frame.outer.row + 1);
    }
}
