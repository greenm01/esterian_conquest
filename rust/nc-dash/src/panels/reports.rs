//! Right panel: compact inbox summary.

use nc_ui::{CellStyle, PlayfieldBuffer};

use crate::app::state::DashApp;
use crate::inbox::{ReportsPanelSummary, project_inbox_items, summarize_reports_panel};
use crate::layout::{self, PanelWidgetFrame};
use crate::theme;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryRow {
    text: String,
    style: CellStyle,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, frame: PanelWidgetFrame) {
    let viewer = app.player_record_index_1_based as u8;
    let items = project_inbox_items(
        &app.game_data,
        viewer,
        &app.report_block_rows,
        &app.queued_mail,
    );
    let summary = summarize_reports_panel(&items, app.game_data.conquest.game_year());

    layout::write_panel_title(
        buf,
        frame,
        &format!(
            "REPORTS ({R}R,{M}M)",
            R = summary.report_count,
            M = summary.message_count
        ),
        theme::section_title_style(),
    );

    for (row_idx, row) in summary_rows(summary, frame.body.height)
        .into_iter()
        .enumerate()
    {
        layout::write_panel_body_line(buf, frame, row_idx, &row.text, row.style);
    }
}

fn summary_rows(summary: ReportsPanelSummary, max_rows: usize) -> Vec<SummaryRow> {
    if max_rows == 0 {
        return Vec::new();
    }

    let mut rows = vec![
        metric_row("Pending", summary.pending_count, theme::value_style()),
        metric_row("Reports", summary.report_count, theme::label_style()),
        metric_row("Messages", summary.message_count, theme::label_style()),
    ];

    let remaining = max_rows.saturating_sub(rows.len());
    if remaining >= 2 {
        if remaining >= 3 {
            rows.push(blank_row());
        }
        rows.push(metric_row(
            "Current",
            summary.current_count,
            theme::friendly_style(),
        ));
        rows.push(metric_row(
            "Backlog",
            summary.backlog_count,
            theme::dim_style(),
        ));
    } else if remaining == 1 {
        rows.push(metric_row(
            "Current",
            summary.current_count,
            theme::friendly_style(),
        ));
    }

    let remaining = max_rows.saturating_sub(rows.len());
    if remaining >= 3 {
        if remaining >= 4 {
            rows.push(blank_row());
        }
        rows.push(metric_row(
            "Combat",
            summary.combat_count,
            theme::enemy_style(),
        ));
        rows.push(metric_row(
            "Intel",
            summary.intel_count,
            theme::value_style(),
        ));
        rows.push(metric_row("Ops", summary.ops_count, theme::label_style()));
    } else if remaining > 0 {
        let buckets = [
            metric_row("Combat", summary.combat_count, theme::enemy_style()),
            metric_row("Intel", summary.intel_count, theme::value_style()),
            metric_row("Ops", summary.ops_count, theme::label_style()),
        ];
        rows.extend(buckets.into_iter().take(remaining));
    }

    let remaining = max_rows.saturating_sub(rows.len());
    if remaining >= 1 {
        if remaining >= 2 {
            rows.push(blank_row());
        }
        if rows.len() < max_rows {
            rows.push(SummaryRow {
                text: String::from(" Press R   Inbox"),
                style: theme::section_title_style(),
            });
        }
    }

    rows.truncate(max_rows);
    rows
}

fn metric_row(label: &str, count: usize, style: CellStyle) -> SummaryRow {
    SummaryRow {
        text: format!(" {:<8}{:>4}", label, count),
        style,
    }
}

fn blank_row() -> SummaryRow {
    SummaryRow {
        text: String::new(),
        style: theme::body_style(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_rows_prioritize_counts_before_breakdowns() {
        let rows = summary_rows(
            ReportsPanelSummary {
                pending_count: 12,
                report_count: 8,
                message_count: 4,
                current_count: 9,
                backlog_count: 3,
                combat_count: 5,
                intel_count: 2,
                ops_count: 1,
            },
            5,
        );

        let texts = rows.into_iter().map(|row| row.text).collect::<Vec<_>>();
        assert_eq!(texts[0], " Pending   12");
        assert_eq!(texts[1], " Reports    8");
        assert_eq!(texts[2], " Messages   4");
        assert!(texts.iter().any(|row| row == " Current    9"));
        assert!(!texts.iter().any(|row| row.contains("Body")));
    }

    #[test]
    fn summary_rows_include_inbox_hint_when_space_allows() {
        let rows = summary_rows(ReportsPanelSummary::default(), 12);
        assert!(rows.iter().any(|row| row.text == " Press R   Inbox"));
    }
}
