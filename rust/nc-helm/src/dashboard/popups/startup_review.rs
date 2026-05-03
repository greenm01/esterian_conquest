use crate::dashboard::app::state::{DashApp, StartupReviewMode};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::{MapWidgetFrame, dashboard};
use crate::dashboard::modal::{Rect, max_content_width};
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::dashboard::table::TableFooter;
use crate::dashboard::theme;
use nc_data::wrap_review_text_preserving_spacing;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_width = max_content_width(parent).min(80);
    let body_height = 20.min(parent.height.saturating_sub(6));

    let (blocks, block_idx, nonstop, title) = match app.startup_review.mode {
        StartupReviewMode::Results => (
            &app.startup_review.reports.result_blocks,
            app.startup_review.results_block,
            app.startup_review.results_nonstop,
            "EMPIRE STATUS REPORT",
        ),
        StartupReviewMode::Messages => (
            &app.startup_review.reports.message_blocks,
            app.startup_review.messages_block,
            app.startup_review.messages_nonstop,
            "SUBSPACE MESSAGES",
        ),
    };

    let footer = if nonstop {
        TableFooter::CommandPrompt {
            label: "REVIEW",
            prompt: "<ESC> Stop nonstop ->",
        }
    } else {
        TableFooter::CommandPrompt {
            label: "REVIEW",
            prompt: "[S] Nonstop / (slap a key) ->",
        }
    };

    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        parent,
        title,
        body_width,
        body_height.into(),
        OverlaySizePolicy::default(),
        footer,
        app.popup_position_for(crate::dashboard::app::state::ActivePopup::StartupReview),
    );

    let mut lines = Vec::new();
    if blocks.is_empty() {
        lines.extend(wrap_review_text_preserving_spacing(
            "No review items pending.",
            body_width,
        ));
    } else if let Some(block) = blocks.get(block_idx) {
        for line in &block.lines {
            lines.extend(wrap_review_text_preserving_spacing(line, body_width));
        }
    }

    // Bottom-aligned drawing
    let visible_lines = lines.len().min(body_height.into());
    let start_y = frame.body_row + usize::from(body_height) - visible_lines;
    let lines_to_draw = &lines[lines.len().saturating_sub(visible_lines)..];

    for (idx, line) in lines_to_draw.iter().enumerate() {
        buf.write_text(start_y + idx, frame.body_col, line, theme::body_style());
    }
}

pub fn popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    let parent = dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets);
    let body_width = max_content_width(parent).min(80);
    let body_height = 20.min(parent.height.saturating_sub(6));

    let (title, nonstop) = match app.startup_review.mode {
        StartupReviewMode::Results => ("EMPIRE STATUS REPORT", app.startup_review.results_nonstop),
        StartupReviewMode::Messages => ("SUBSPACE MESSAGES", app.startup_review.messages_nonstop),
    };

    let footer = if nonstop {
        TableFooter::CommandPrompt {
            label: "REVIEW",
            prompt: "<ESC> Stop nonstop ->",
        }
    } else {
        TableFooter::CommandPrompt {
            label: "REVIEW",
            prompt: "[S] Nonstop / (slap a key) ->",
        }
    };

    overlay_popup_rect_for_body_in_parent(
        parent,
        title,
        body_width,
        body_height.into(),
        OverlaySizePolicy::default(),
        footer,
        app.popup_position_for(crate::dashboard::app::state::ActivePopup::StartupReview),
    )
}
