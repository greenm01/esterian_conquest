//! Right panel: condensed detail for the currently selected map sector.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::planet_view::{
    DetailLine, preferred_sector_detail_body_width, projected_sector_details,
    selected_planet_detail, widget_label_for_width,
};
use crate::theme;

pub(crate) const TITLE: &str = "SECTOR DETAIL";
const PREFERRED_BODY_WIDTH_CAP: usize = 24;
pub(crate) const MAX_BODY_ROWS: usize = 16;
pub(crate) const MIN_BODY_ROWS: usize = 8;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, TITLE, theme::section_title_style());

    let Some(detail) = selected_planet_detail(app) else {
        layout::write_panel_body_line(buf, frame, 0, "empty sector", theme::dim_style());
        return;
    };

    for (row_idx, line) in
        rendered_widget_lines(&detail.widget_fields, frame.body.width, frame.body.height)
            .into_iter()
            .enumerate()
    {
        layout::write_panel_body_line(buf, frame, row_idx, &line, theme::value_style());
    }
}

pub(crate) fn preferred_body_width(app: &DashApp) -> usize {
    preferred_sector_detail_body_width(app)
        .max("empty sector".chars().count())
        .min(PREFERRED_BODY_WIDTH_CAP)
}

pub(crate) fn preferred_body_rows(app: &DashApp) -> usize {
    projected_sector_details(app)
        .into_iter()
        .map(|detail| {
            rendered_widget_lines(
                &detail.widget_fields,
                preferred_body_width(app),
                MAX_BODY_ROWS,
            )
            .len()
        })
        .max()
        .unwrap_or(1)
        .clamp(MIN_BODY_ROWS, MAX_BODY_ROWS)
}

fn rendered_widget_lines(rows: &[DetailLine], body_width: usize, max_rows: usize) -> Vec<String> {
    if max_rows == 0 {
        return Vec::new();
    }

    let mut indexed = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            (
                row_priority(row.label),
                idx,
                render_widget_field_lines(row, body_width),
            )
        })
        .collect::<Vec<_>>();
    indexed.sort_by_key(|(priority, idx, _)| (*priority, *idx));

    let mut used_rows = 0usize;
    let mut kept = indexed
        .into_iter()
        .filter_map(|(_, idx, lines)| {
            if used_rows + lines.len() > max_rows {
                return None;
            }
            used_rows += lines.len();
            Some((idx, lines))
        })
        .collect::<Vec<_>>();
    kept.sort_by_key(|(idx, _)| *idx);
    kept.into_iter().flat_map(|(_, lines)| lines).collect()
}

fn render_widget_field_lines(field: &DetailLine, body_width: usize) -> Vec<String> {
    let label = widget_label_for_width(field, body_width);
    let prefix = format!("{label}: ");

    if !field_is_wrappable(field.label) {
        return vec![format!("{prefix}{}", field.value)];
    }

    wrap_field_value_lines(&prefix, &field.value, body_width)
}

fn wrap_field_value_lines(prefix: &str, value: &str, body_width: usize) -> Vec<String> {
    let prefix_width = prefix.chars().count();
    let available = body_width.saturating_sub(prefix_width);
    if available == 0 {
        return vec![prefix.trim_end().to_string()];
    }

    let wrapped = wrap_tokens(value, available);
    let continuation = " ".repeat(prefix_width);

    wrapped
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 {
                format!("{prefix}{line}")
            } else {
                format!("{continuation}{line}")
            }
        })
        .collect()
}

fn wrap_tokens(value: &str, max_width: usize) -> Vec<String> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for token in tokens {
        let token_width = token.chars().count();
        if current.is_empty() {
            current.push_str(token);
            continue;
        }

        let next_width = current.chars().count() + 1 + token_width;
        if next_width <= max_width {
            current.push(' ');
            current.push_str(token);
        } else {
            lines.push(current);
            current = token.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn field_is_wrappable(label: &str) -> bool {
    matches!(label, "Building" | "Docked")
}

fn row_priority(label: &str) -> usize {
    match label {
        "Planet" => 0,
        "Owner" => 1,
        "State" => 2,
        "Intel" => 3,
        "Production" => 4,
        "Potential Production" => 5,
        "Treasury" => 6,
        "Armies" => 7,
        "Ground Batteries" => 8,
        "Starbases" => 9,
        "Building" => 10,
        "Docked" => 11,
        "Orbit" => 12,
        _ => 13,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_BODY_ROWS, preferred_body_rows, preferred_body_width, rendered_widget_lines,
        wrap_field_value_lines,
    };
    use crate::app::state::DashApp;
    use crate::layout::dashboard_layout;
    use crate::planet_view::DetailLine;
    use crate::theme;
    use nc_data::GameStateBuilder;
    use nc_ui::{PlayfieldBuffer, ScreenGeometry};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn widget_rows_keep_top_priority_lines_when_height_is_tight() {
        let rows = vec![
            DetailLine {
                label: "Planet",
                value: String::from("Foo"),
            },
            DetailLine {
                label: "Owner",
                value: String::from("You"),
            },
            DetailLine {
                label: "State",
                value: String::from("Normal"),
            },
            DetailLine {
                label: "Production",
                value: String::from("9"),
            },
            DetailLine {
                label: "Potential Production",
                value: String::from("10"),
            },
            DetailLine {
                label: "Treasury",
                value: String::from("8"),
            },
            DetailLine {
                label: "Armies",
                value: String::from("1"),
            },
            DetailLine {
                label: "Ground Batteries",
                value: String::from("2"),
            },
            DetailLine {
                label: "Starbases",
                value: String::from("3"),
            },
            DetailLine {
                label: "Orbit",
                value: String::from("Fleet"),
            },
            DetailLine {
                label: "Building",
                value: String::from("Nothing"),
            },
            DetailLine {
                label: "Docked",
                value: String::from("Nothing"),
            },
        ];

        assert_eq!(
            rendered_widget_lines(&rows, 19, 5),
            vec![
                String::from("Planet: Foo"),
                String::from("Owner: You"),
                String::from("State: Normal"),
                String::from("Production: 9"),
                String::from("Pot Prod: 10"),
            ]
        );
    }

    #[test]
    fn empty_sector_uses_compact_copy() {
        let mut app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(0, 0),
            1,
        );
        let occupied = app
            .game_data
            .planets
            .records
            .iter()
            .map(|planet| planet.coords_raw())
            .collect::<std::collections::BTreeSet<_>>();
        let empty = (1..=18)
            .flat_map(|y| (1..=18).map(move |x| [x, y]))
            .find(|coords| !occupied.contains(coords))
            .expect("empty sector");
        app.crosshair_x = empty[0];
        app.crosshair_y = empty[1];
        let widgets = dashboard_layout(&app).widgets;
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            theme::body_style(),
        );

        super::draw(&mut buffer, &app, widgets.right_sector_detail);

        assert!(
            buffer
                .plain_line(widgets.right_sector_detail.body.row)
                .contains("empty sector")
        );
    }

    #[test]
    fn preferred_width_covers_empty_and_world_states() {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 40),
            ScreenGeometry::new(0, 0),
            1,
        );

        assert!(preferred_body_width(&app) >= "empty sector".chars().count());
        assert_eq!(MAX_BODY_ROWS, 16);
        assert!(preferred_body_rows(&app) <= MAX_BODY_ROWS);
    }

    #[test]
    fn visible_sector_rows_do_not_pad_labels_to_align_colons() {
        let rows = vec![
            DetailLine {
                label: "Planet",
                value: String::from("Aurora"),
            },
            DetailLine {
                label: "Owner",
                value: String::from("You"),
            },
            DetailLine {
                label: "State",
                value: String::from("Stable"),
            },
            DetailLine {
                label: "Production",
                value: String::from("98"),
            },
            DetailLine {
                label: "Potential Production",
                value: String::from("120"),
            },
        ];
        let formatted = rendered_widget_lines(&rows, 19, rows.len());

        assert_eq!(formatted[0], "Planet: Aurora");
        assert_eq!(formatted[1], "Owner: You");
        assert_eq!(formatted[2], "State: Stable");
    }

    #[test]
    fn building_and_docked_wrap_on_token_boundaries() {
        let lines = wrap_field_value_lines("Building: ", "5BB 10CA 4DD 6TT 3ET 2SB", 24);

        assert_eq!(
            lines,
            vec![
                String::from("Building: 5BB 10CA 4DD"),
                String::from("          6TT 3ET 2SB"),
            ]
        );
    }
}
