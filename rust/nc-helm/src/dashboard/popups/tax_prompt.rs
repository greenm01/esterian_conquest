use crate::dashboard::app::state::{ActivePopup, DashApp};
use crate::dashboard::buffer::PlayfieldBuffer;
use crate::dashboard::layout::{self, MapWidgetFrame, dashboard};
use crate::dashboard::modal::Rect;
use crate::dashboard::overlays::frame::{
    OverlaySizePolicy, dashboard_overlay_parent_rect,
    draw_overlay_frame_for_body_in_parent_with_policy_and_origin,
    overlay_popup_rect_for_body_in_parent,
};
use crate::dashboard::table::{TableFooter, with_command_line_toast};
use crate::dashboard::theme;

const TITLE: &str = "TAX RATE";
const BODY_WIDTH: usize = 44;
const BODY_HEIGHT: usize = 3;

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, _map_frame: MapWidgetFrame) {
    let frame = draw_overlay_frame_for_body_in_parent_with_policy_and_origin(
        buf,
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        TITLE,
        BODY_WIDTH,
        BODY_HEIGHT,
        OverlaySizePolicy::default(),
        footer(app),
        app.popup_position_for(ActivePopup::TaxPrompt),
    );
    let tax = current_tax(app);
    let lines = [
        format!("Current empire tax rate: {tax}%"),
        String::from("Enter 0-100. Rates above 65% can damage production."),
        app.tax_prompt_status.clone().unwrap_or_default(),
    ];
    for (idx, line) in lines.into_iter().enumerate().take(frame.body_height) {
        let style = if idx == 2 {
            theme::alert_style()
        } else {
            theme::value_style()
        };
        layout::write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            &line,
            style,
        );
    }
}

pub fn popup_rect(app: &DashApp, _map_frame: MapWidgetFrame) -> Rect {
    overlay_popup_rect_for_body_in_parent(
        dashboard_overlay_parent_rect(dashboard::dashboard_layout(app).widgets),
        TITLE,
        BODY_WIDTH,
        BODY_HEIGHT,
        OverlaySizePolicy::default(),
        footer(app),
        app.popup_position_for(ActivePopup::TaxPrompt),
    )
}

fn footer<'a>(app: &'a DashApp) -> TableFooter<'a> {
    with_command_line_toast(
        TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Tax rate -> ",
            input: &app.tax_prompt_input,
        },
        app.active_command_line_toast(),
    )
}

fn current_tax(app: &DashApp) -> u8 {
    app.game_data
        .player
        .records
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|player| player.tax_rate())
        .unwrap_or(0)
}
