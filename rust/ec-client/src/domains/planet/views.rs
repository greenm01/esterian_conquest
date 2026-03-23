use crate::app::state::App;
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame, ScreenId, build_unit_spec_by_kind};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        planet_intel_snapshots: &app.planet_intel_snapshots,
    };
    match app.current_screen {
        ScreenId::PlanetMenu => app
            .planet_menu
            .render_with_notice(app.command_menu_notice.as_deref()),
        ScreenId::PlanetHelp => app.planet_help.render(&frame),
        ScreenId::PlanetAutoCommissionConfirm => app.planet_auto_commission.render_confirm(),
        ScreenId::PlanetAutoCommissionDone => app.planet_auto_commission.render_done(
            app.planet
                .auto_commission_status
                .as_deref()
                .unwrap_or("Auto-commission complete."),
        ),
        ScreenId::PlanetTransportPlanetSelect(mode) => app.planet_transport.render_planet_select(
            crate::screen::command_menu_label(app.command_return_menu),
            mode,
            &app.planet_transport_planet_rows(mode),
            app.planet.transport_planet_scroll_offset,
            app.planet.transport_planet_cursor,
            &app.planet.transport_planet_input,
            app.planet_transport_planet_default_coords(mode),
            app.status_if_no_modal(app.planet.transport_status.as_deref()),
        ),
        ScreenId::PlanetTransportFleetSelect(mode) => app.planet_transport.render_fleet_select(
            crate::screen::command_menu_label(app.command_return_menu),
            mode,
            &app.current_planet_transport_planet_row(mode)?,
            &app.current_planet_transport_fleet_rows(mode)?,
            app.planet.transport_fleet_scroll_offset,
            app.planet.transport_fleet_cursor,
            &app.planet.transport_qty_input,
            app.status_if_no_modal(app.planet.transport_status.as_deref()),
        ),
        ScreenId::PlanetTransportQuantityPrompt(mode) => {
            app.planet_transport.render_quantity_prompt(
                crate::screen::command_menu_label(app.command_return_menu),
                mode,
                &app.current_planet_transport_planet_row(mode)?,
                &app.current_planet_transport_fleet_row(mode)?,
                &app.planet.transport_qty_input,
                app.status_if_no_modal(app.planet.transport_status.as_deref()),
            )
        }
        ScreenId::PlanetTransportDone(mode) => app.planet_transport.render_done(
            crate::screen::command_menu_label(app.command_return_menu),
            mode,
            app.planet
                .transport_status
                .as_deref()
                .unwrap_or("Transport order completed."),
        ),
        ScreenId::PlanetCommissionMenu => app.planet_commission.render_menu(
            &app.current_planet_commission_view()?,
            app.planet.commission_scroll_offset,
            app.planet.commission_cursor,
            &app.planet.commission_selected_slots,
            app.planet.commission_status.as_deref(),
        ),
        ScreenId::PlanetBuildHelp => app.build_help.render(&frame),
        ScreenId::PlanetBuildMenu => app.planet_build.render_menu(
            &app.current_planet_build_view()?,
            app.planet.build_status.as_deref(),
        ),
        ScreenId::PlanetBuildReview => app.planet_build.render_review(
            &app.current_planet_build_view()?,
            &app.current_planet_build_orders(),
        ),
        ScreenId::PlanetBuildList => app.planet_build.render_list(
            &app.current_planet_build_view()?,
            &app.planet_build_list_rows(),
            app.planet.build_list_scroll_offset,
            app.planet.build_list_cursor,
            app.planet.build_list_confirming,
        ),
        ScreenId::PlanetBuildChange => app.planet_build.render_change(
            &app.build_change_rows(),
            app.planet.build_change_scroll_offset,
            app.planet.build_change_cursor,
        ),
        ScreenId::PlanetBuildAbortConfirm => app.planet_build.render_abort_confirm(
            &app.current_planet_build_view()?,
            &app.current_planet_build_orders(),
        ),
        ScreenId::PlanetBuildSpecify => app.planet_build.render_specify(
            &app.current_planet_build_view()?,
            &app.current_planet_build_orders(),
            &app.planet.build_unit_input,
            app.planet.build_unit_status.as_deref(),
        ),
        ScreenId::PlanetBuildQuantity => app.planet_build.render_quantity_prompt(
            &app.current_planet_build_view()?,
            &app.current_planet_build_orders(),
            build_unit_spec_by_kind(
                app.planet
                    .build_selected_kind
                    .ok_or("planet build kind not selected")?,
            )
            .ok_or("planet build unit missing")?,
            app.current_planet_build_max_quantity()?,
            &app.planet.build_quantity_input,
            app.planet.build_quantity_status.as_deref(),
        ),
        ScreenId::PlanetListSortPrompt(mode) => app.planet_list.render_sort_prompt(
            &frame,
            mode,
            &app.sorted_planet_rows(app.planet.list_sort),
            app.planet.list_sort,
            app.planet.brief_scroll_offset,
            app.planet.brief_cursor,
            &app.planet.brief_input,
            app.planet.list_sort_status.as_deref(),
        ),
        ScreenId::PlanetBriefList(sort) => app.planet_list.render_brief_list(
            &frame,
            &app.sorted_planet_rows(sort),
            sort,
            app.planet.brief_scroll_offset,
            app.planet.brief_cursor,
            &app.planet.brief_input,
        ),
        ScreenId::PlanetDetailList(sort) => app.planet_list.render_detail(
            &frame,
            &app.sorted_planet_rows(sort),
            app.planet.detail_index,
        ),
        ScreenId::PlanetTaxPrompt => {
            let current_tax = app.game_data.player.records[app.player.record_index_1_based - 1]
                .tax_rate()
                .to_string();
            app.planet_tax.render_prompt(
                &current_tax,
                &app.planet.tax_input,
                app.planet.tax_status.as_deref(),
            )
        }
        ScreenId::PlanetTaxDone => app.planet_tax.render_done(
            app.planet
                .tax_status
                .as_deref()
                .unwrap_or("Tax rate updated."),
        ),
        ScreenId::PlanetDatabaseList => app.planet_database.render_list(
            &app.planet_database_rows(),
            app.planet.database_scroll_offset,
            app.planet.database_cursor,
            app.default_planet_prompt_coords(),
            &app.planet.database_input,
            app.status_if_no_modal(app.planet.database_status.as_deref()),
            app.command_return_menu,
        ),
        ScreenId::PlanetDatabaseFilterPrompt => app.planet_database.render_filter_prompt(
            &app.planet_database_rows(),
            app.planet.database_scroll_offset,
            app.planet.database_cursor,
            app.default_planet_prompt_coords(),
            &app.planet.database_input,
            app.status_if_no_modal(app.planet.database_status.as_deref()),
            app.command_return_menu,
        ),
        ScreenId::PlanetDatabaseDetail => {
            let rows = app.planet_database_rows();
            let row = rows
                .get(app.planet.database_detail_index)
                .ok_or("planet database row missing")?;
            app.planet_database
                .render_detail(row, app.planet.database_detail_index, rows.len())
        }
        ScreenId::PlanetInfoPrompt => app.planet_info.render_prompt(
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
            app.command_return_menu,
        ),
        ScreenId::PlanetInfoDetail => app.planet_info.render_detail(
            &frame,
            app.planet
                .info_selected
                .ok_or("planet info detail not selected")?,
            app.command_return_menu,
        ),
        _ => unreachable!("planet views called for non-planet screen"),
    }
}
