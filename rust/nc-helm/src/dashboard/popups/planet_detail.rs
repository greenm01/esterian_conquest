//! Planet info popup rendered over the center map pane.

use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::table::TableFooter;

use crate::dashboard::app::state::DashApp;
use crate::dashboard::layout::{self, MapWidgetFrame, dashboard};
use crate::dashboard::modal::Rect;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::dashboard::planet_view::selected_planet_detail;
use crate::dashboard::theme;

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
    let popup = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "PLANET INFO",
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::None,
        app.popup_position_for(crate::dashboard::app::state::ActivePopup::PlanetDetail {
            planet_record_index_1_based: _planet_record_index_1_based,
        }),
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

pub fn popup_rect(
    app: &DashApp,
    map_frame: MapWidgetFrame,
    planet_record_index_1_based: usize,
) -> Rect {
    let Some(detail) = selected_planet_detail(app) else {
        return overlay_popup_rect_for_body_in_parent(
            dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
            "PLANET INFO",
            1,
            1,
            OverlaySizePolicy::default(),
            TableFooter::None,
            app.popup_position_for(crate::dashboard::app::state::ActivePopup::PlanetDetail {
                planet_record_index_1_based,
            }),
        );
    };
    let max_body_width = map_frame.outer.width.saturating_sub(6);
    let lines = popup_lines(&detail.popup_lines, max_body_width);
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        "PLANET INFO",
        body_width,
        lines.len(),
        OverlaySizePolicy::default(),
        TableFooter::None,
        app.popup_position_for(crate::dashboard::app::state::ActivePopup::PlanetDetail {
            planet_record_index_1_based,
        }),
    )
}

pub(crate) fn popup_lines(
    lines: &[crate::dashboard::planet_view::DetailLine],
    max_body_width: usize,
) -> Vec<String> {
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
    use super::popup_lines;
    use crate::dashboard::buffer::PlayfieldBuffer;
    use crate::dashboard::layout::widgets::WidgetRect;
    use crate::dashboard::overlays::frame::draw_overlay_frame_for_body_in_map;
    use crate::dashboard::planet_view::DetailLine;
    use crate::dashboard::table::TableFooter;
    use crate::dashboard::theme;

    #[test]
    fn popup_lines_wrap_long_values_with_aligned_colon_continuations() {
        let lines = popup_lines(
            &[DetailLine {
                label: "Status",
                value: String::from("Regular planet - industry intact"),
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
        let map_frame = crate::dashboard::layout::MapWidgetFrame {
            outer: WidgetRect {
                col: 20,
                row: 5,
                width: 60,
                height: 20,
            },
            map_block: WidgetRect {
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

        let popup = draw_overlay_frame_for_body_in_map(
            &mut buffer,
            map_frame,
            "PLANET INFO",
            30,
            8,
            TableFooter::None,
        );

        assert!(popup.body_col > map_frame.outer.col + 2);
        assert!(popup.body_row > map_frame.outer.row + 1);
        assert!((0..buffer.height()).any(|row| buffer.plain_line(row).contains("┐PLANET INFO┌")));
        for row in 0..buffer.height() {
            assert!(!buffer.plain_line(row).contains("(slap a key)"));
        }
    }
}
