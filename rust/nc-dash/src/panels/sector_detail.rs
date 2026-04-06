//! Right panel: condensed detail for the currently selected map sector.

use nc_ui::PlayfieldBuffer;

use crate::app::state::DashApp;
use crate::layout::{self, PanelWidgetFrame};
use crate::planet_view::selected_planet_detail;
use crate::theme;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    layout::write_panel_title(buf, frame, "SECTOR DETAIL", theme::section_title_style());

    let Some(detail) = selected_planet_detail(app) else {
        layout::write_panel_body_line(buf, frame, 0, "No world in sector", theme::dim_style());
        return;
    };

    for (row_idx, line) in prioritized_widget_rows(&detail.widget_lines, frame.body.height)
        .into_iter()
        .enumerate()
    {
        layout::write_panel_body_line(buf, frame, row_idx, &line, theme::value_style());
    }
}

fn prioritized_widget_rows(rows: &[String], max_rows: usize) -> Vec<String> {
    rows.iter().take(max_rows).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::prioritized_widget_rows;

    #[test]
    fn widget_rows_keep_top_priority_lines_when_height_is_tight() {
        let rows = vec![
            String::from("Planet  Foo"),
            String::from("Owner   You"),
            String::from("Econ    10|9|8"),
            String::from("Def     1|2|3"),
            String::from("State   Normal"),
            String::from("Build   Nothing"),
            String::from("Docked  Nothing"),
        ];

        assert_eq!(prioritized_widget_rows(&rows, 5), rows[..5].to_vec());
    }
}
