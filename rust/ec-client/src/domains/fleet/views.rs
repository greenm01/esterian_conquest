use crate::app::state::App;
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame, ScreenId};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        planet_intel_snapshots: &app.planet_intel_snapshots,
    };
    match app.current_screen {
        ScreenId::FleetHelp => app.fleet_help.render(&frame),
        ScreenId::FleetMenu => app
            .fleet_menu
            .render_with_notice(app.command_menu_notice.as_deref(), app.expert_mode),
        ScreenId::FleetList(mode) => app.fleet_list.render(
            mode,
            &app.fleet_rows(),
            app.fleet.scroll_offset,
            app.fleet.cursor,
        ),
        ScreenId::FleetReviewSelect => app.fleet_review.render_select(
            &app.fleet_rows(),
            app.fleet.scroll_offset,
            app.fleet.cursor,
            &app.fleet.review_select_input,
            app.status_if_no_modal(app.fleet.review_status.as_deref()),
        ),
        ScreenId::FleetReview => {
            let rows = app.fleet_rows();
            let row = rows
                .get(app.fleet.review_index)
                .ok_or("fleet review row missing")?;
            app.fleet_review
                .render(row, app.fleet.review_index, rows.len())
        }
        ScreenId::FleetRoeSelect => app.fleet_roe.render_select(
            &app.fleet_rows(),
            app.fleet.roe_scroll_offset,
            app.fleet.roe_cursor,
            app.fleet.roe_editing,
            &app.fleet.roe_select_input,
            &app.fleet.roe_input,
            app.status_if_no_modal(app.fleet.roe_status.as_deref()),
        ),
        ScreenId::FleetOrder => app.fleet_order.render(
            &app.fleet_rows(),
            app.fleet.order_scroll_offset,
            app.fleet.order_cursor,
            app.fleet.order_mode,
            &app.fleet_order_target_status_line(),
            &app.fleet_order_target_prompt(),
            &app.fleet_order_target_default(),
            &app.fleet.order_input,
            app.status_if_no_modal(app.fleet.order_status.as_deref()),
        ),
        ScreenId::FleetGroupOrder => app.fleet_group.render(
            &app.fleet_rows(),
            app.fleet.group_scroll_offset,
            app.fleet.group_cursor,
            &app.fleet.group_selected_fleets,
            app.fleet.group_mode,
            &app.fleet_group_target_status_line(),
            &app.fleet_group_target_prompt(),
            &app.fleet_group_target_default(),
            &app.fleet.group_input,
            app.status_if_no_modal(app.fleet.group_status.as_deref()),
        ),
        ScreenId::FleetMissionPicker => app.fleet_mission_picker.render(
            app.fleet.mission_picker_cursor,
            &app.fleet.mission_picker_input,
            &app.fleet_mission_picker_enabled_flags(),
            app.status_if_no_modal(app.fleet.mission_picker_status.as_deref()),
        ),
        ScreenId::FleetMerge => {
            let rows = app.current_fleet_merge_rows();
            let input = app.current_fleet_merge_input().to_string();
            let status = app.status_if_no_modal(app.fleet.merge_status.as_deref());
            app.fleet_merge.render(
                &rows,
                app.fleet.merge_scroll_offset,
                app.fleet.merge_cursor,
                app.fleet.merge_mode,
                &input,
                status,
            )
        }
        ScreenId::FleetTransfer => {
            let rows = app.current_fleet_transfer_rows();
            let input = app.current_fleet_transfer_input().to_string();
            let status = app.status_if_no_modal(app.fleet.transfer_status.as_deref());
            let (prompt, default) = app.fleet_transfer_prompt_and_default(&rows);
            app.fleet_transfer.render(
                &rows,
                app.fleet.transfer_scroll_offset,
                app.fleet.transfer_cursor,
                app.fleet.transfer_mode,
                &app.fleet.transfer_selected_fleets,
                app.fleet
                    .transfer_donor_record_index_1_based
                    .and_then(|idx| app.fleet_number_for_record_index(idx)),
                app.fleet
                    .transfer_host_record_index_1_based
                    .and_then(|idx| app.fleet_number_for_record_index(idx)),
                &input,
                status,
                &prompt,
                &default,
            )
        }
        ScreenId::FleetDetach => {
            let rows = app.fleet_rows();
            let (prompt, default) = app.fleet_detach_prompt_and_default(&rows);
            let input = app.fleet_detach_current_input().to_string();
            let status = app.status_if_no_modal(app.fleet.detach_status.as_deref());
            app.fleet_detach.render(
                &rows,
                app.fleet.detach_scroll_offset,
                app.fleet.detach_cursor,
                &prompt,
                &default,
                &input,
                status,
            )
        }
        ScreenId::FleetEta => app.fleet_eta.render(
            &app.fleet_rows(),
            app.fleet.eta_scroll_offset,
            app.fleet.eta_cursor,
            app.fleet.eta_mode,
            &app.fleet.eta_select_input,
            app.fleet_eta_default_destination(),
            &app.fleet.eta_destination_input,
            &app.fleet.eta_include_system_input,
            app.status_if_no_modal(app.fleet.eta_status.as_deref()),
        ),
        _ => unreachable!("fleet views called for non-fleet screen"),
    }
}
