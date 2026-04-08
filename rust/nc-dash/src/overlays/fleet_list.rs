//! F overlay: dashboard-sized fleet and starbase command table.

use nc_engine::{FLEET_MISSION_OPTIONS, fleet_list_eta_label, starbase_eta_label};
use nc_ui::PlayfieldBuffer;
use nc_ui::coords::format_sector_coords_table;
use nc_ui::modal::Rect;
use nc_ui::table::{
    TableColumn, TableFooter, TableWidthMode, centered_table_start_col, resolve_table_columns,
    table_render_width, write_table_window_with_theme_at,
};
use nc_ui::table_selection;

use crate::app::state::{
    ActiveOverlay, DashApp, FleetOverlayFilter, FleetOverlayPromptMode, FleetOverlayRowKey,
    FleetOverlaySort,
};
use crate::layout::MapWidgetFrame;
use crate::overlays::frame::{
    OverlaySizePolicy, assert_overlay_body_write_fits, draw_overlay_frame_for_body_in_map,
    draw_overlay_frame_for_body_in_map_with_origin, max_overlay_body_width,
    overlay_popup_rect_for_body_in_map, standard_table_body_height, write_clipped,
};
use nc_data::Order;

pub fn order_abbrev(order: Order) -> &'static str {
    match order {
        Order::HoldPosition => "Hd",
        Order::MoveOnly => "Mv",
        Order::SeekHome => "Sk",
        Order::PatrolSector => "Pa",
        Order::GuardStarbase => "Gs",
        Order::GuardBlockadeWorld => "Gb",
        Order::BombardWorld => "Bo",
        Order::InvadeWorld => "In",
        Order::BlitzWorld => "Bz",
        Order::ViewWorld => "Vw",
        Order::ScoutSector => "Ss",
        Order::ScoutSolarSystem => "Sy",
        Order::ColonizeWorld => "Co",
        Order::JoinAnotherFleet => "Jn",
        Order::RendezvousSector => "Rz",
        Order::Salvage => "Sa",
        Order::Unknown(_) => "??",
    }
}
use crate::theme;

pub(crate) const HOTKEYS: &str = "? F S O SPACE <Q>";
pub(crate) const FILTER_HOTKEYS: &str = "? A H M C <Q>";
pub(crate) const SORT_HOTKEYS: &str = "? I L O E T <Q>";
const GROUP_ORDER_BODY_WIDTH: usize = 54;

const COLUMNS: [TableColumn<'static>; 10] = [
    TableColumn::right("ID", 4),
    TableColumn::left("Sel.", 4),
    TableColumn::left("Location", 8),
    TableColumn::left("Order", 5),
    TableColumn::left("Target", 8),
    TableColumn::right("Spd", 3),
    TableColumn::right("ETA", 4),
    TableColumn::right("ROE", 3),
    TableColumn::right("ARs", 3),
    TableColumn::left_flex("Ships / Forces", 24, 1),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FleetOverlayRow {
    pub key: FleetOverlayRowKey,
    pub id_label: String,
    pub coords: [u8; 2],
    pub order: Order,
    pub eta_label: String,
    pub strength_key: (u16, u16, u16, u16, u8, u16, u16),
    pub cells: Vec<String>,
}

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::MissionPicker => {
            draw_mission_picker(buf, app, map_frame);
            return;
        }
        FleetOverlayPromptMode::OrderTarget
        | FleetOverlayPromptMode::OrderTargetX
        | FleetOverlayPromptMode::OrderTargetY
        | FleetOverlayPromptMode::OrderConfirm => {
            if app.fleet_order_is_group_scope() {
                draw_group_fleet_order_prompt(buf, app, map_frame);
            } else {
                draw_fleet_order_prompt(buf, app, map_frame);
            }
            return;
        }
        FleetOverlayPromptMode::StarbaseMoveDecision
        | FleetOverlayPromptMode::StarbaseMoveDestination
        | FleetOverlayPromptMode::StarbaseHaltConfirm => {
            draw_starbase_move_prompt(buf, app, map_frame);
            return;
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app.fleet_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows.get(selected).map(|row| row.id_label.as_str());
    let footer = match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default,
            input: &app.fleet_overlay.jump_input,
        },
        FleetOverlayPromptMode::FilterMenu => TableFooter::LabeledCommandBar {
            label: "FILTER",
            hotkeys_markup: FILTER_HOTKEYS,
            default: None,
            input: "",
        },
        FleetOverlayPromptMode::SortMenu => TableFooter::LabeledCommandBar {
            label: "SORT",
            hotkeys_markup: SORT_HOTKEYS,
            default: None,
            input: "",
        },
        FleetOverlayPromptMode::MissionPicker
        | FleetOverlayPromptMode::OrderTarget
        | FleetOverlayPromptMode::OrderTargetX
        | FleetOverlayPromptMode::OrderTargetY
        | FleetOverlayPromptMode::OrderConfirm
        | FleetOverlayPromptMode::StarbaseMoveDecision
        | FleetOverlayPromptMode::StarbaseMoveDestination
        | FleetOverlayPromptMode::StarbaseHaltConfirm => {
            unreachable!("order flows render separately")
        }
    };
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();

    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You have no active fleets or starbases.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body_in_map_with_origin(
        buf,
        map_frame,
        "FLEET LIST",
        body_width,
        standard_table_body_height(natural_visible_rows),
        footer,
        app.overlay_position_for(ActiveOverlay::FleetList),
    );
    let visible_rows = frame.body_height.saturating_sub(4);
    assert_overlay_body_write_fits(
        frame,
        "FLEET LIST",
        table_render_width(&columns),
        standard_table_body_height(visible_rows),
    );
    let scroll = clamp_scroll(app.fleet_overlay.scroll, selected, visible_rows, rows.len());
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let metrics = write_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &columns,
        &table_cells,
        scroll,
        visible_rows,
        theme::table_theme(),
        table_cells.get(selected).map(|_| selected),
        0,
        None,
    );

    if rows.is_empty() {
        write_clipped(
            buf,
            metrics.bottom_row.saturating_sub(1),
            frame.body_col,
            frame.body_width,
            "You have no active fleets or starbases.",
            theme::dim_style(),
        );
    }
}

pub(crate) fn popup_rect(app: &DashApp, map_frame: MapWidgetFrame) -> Option<Rect> {
    match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::MissionPicker
        | FleetOverlayPromptMode::OrderTarget
        | FleetOverlayPromptMode::OrderTargetX
        | FleetOverlayPromptMode::OrderTargetY
        | FleetOverlayPromptMode::OrderConfirm
        | FleetOverlayPromptMode::StarbaseMoveDecision
        | FleetOverlayPromptMode::StarbaseMoveDestination
        | FleetOverlayPromptMode::StarbaseHaltConfirm => {
            return None;
        }
        _ => {}
    }
    let rows = table_rows(app);
    let selected = app.fleet_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows.get(selected).map(|row| row.id_label.as_str());
    let footer = match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::None => TableFooter::CommandBar {
            hotkeys_markup: HOTKEYS,
            default: selected_default,
            input: &app.fleet_overlay.jump_input,
        },
        FleetOverlayPromptMode::FilterMenu => TableFooter::LabeledCommandBar {
            label: "FILTER",
            hotkeys_markup: FILTER_HOTKEYS,
            default: None,
            input: "",
        },
        FleetOverlayPromptMode::SortMenu => TableFooter::LabeledCommandBar {
            label: "SORT",
            hotkeys_markup: SORT_HOTKEYS,
            default: None,
            input: "",
        },
        FleetOverlayPromptMode::MissionPicker
        | FleetOverlayPromptMode::OrderTarget
        | FleetOverlayPromptMode::OrderTargetX
        | FleetOverlayPromptMode::OrderTargetY
        | FleetOverlayPromptMode::OrderConfirm
        | FleetOverlayPromptMode::StarbaseMoveDecision
        | FleetOverlayPromptMode::StarbaseMoveDestination
        | FleetOverlayPromptMode::StarbaseHaltConfirm => {
            unreachable!("prompt overlays are not draggable")
        }
    };
    let table_cells = rows.iter().map(|row| row.cells.clone()).collect::<Vec<_>>();
    let natural_visible_rows = table_cells.len().max(1);
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You have no active fleets or starbases.".chars().count() + 4);
    Some(overlay_popup_rect_for_body_in_map(
        map_frame,
        "FLEET LIST",
        body_width,
        standard_table_body_height(natural_visible_rows),
        OverlaySizePolicy::default(),
        footer,
        app.overlay_position_for(ActiveOverlay::FleetList),
    ))
}

fn draw_mission_picker(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let rows = FLEET_MISSION_OPTIONS
        .iter()
        .map(|option| {
            vec![
                format!("{:02}", option.code),
                option.mission.to_string(),
                option.requirements.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let columns = resolve_table_columns(
        &[
            TableColumn::right("No", 2),
            TableColumn::left("Mission", 24),
            TableColumn::left("Need", 18),
        ],
        &rows,
        max_overlay_body_width(map_frame),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns);
    let default = FLEET_MISSION_OPTIONS
        .get(app.fleet_overlay.mission_picker_cursor)
        .map(|option| option.code.to_string())
        .unwrap_or_else(|| "1".to_string());
    let natural_visible_rows = rows.len().max(1);
    let status_rows = usize::from(app.fleet_overlay.mission_picker_status.is_some());
    let frame = draw_overlay_frame_for_body_in_map(
        buf,
        map_frame,
        "FLEET MISSION ORDERS",
        body_width,
        standard_table_body_height(natural_visible_rows) + status_rows,
        TableFooter::CommandBar {
            hotkeys_markup: "? <Q>",
            default: Some(&default),
            input: &app.fleet_overlay.mission_picker_input,
        },
    );
    let visible_rows = frame
        .body_height
        .saturating_sub(standard_table_body_height(0) + status_rows);
    assert_overlay_body_write_fits(
        frame,
        "FLEET MISSION ORDERS",
        table_render_width(&columns),
        standard_table_body_height(visible_rows) + status_rows,
    );
    let table_col = frame.body_col + centered_table_start_col(frame.body_width, &columns);
    let scroll = clamp_scroll(
        app.fleet_overlay.mission_picker_cursor,
        app.fleet_overlay.mission_picker_cursor,
        visible_rows,
        rows.len(),
    );
    let row_states = row_states_from_enabled_flags(&app.fleet_mission_picker_enabled_flags());
    let _ = write_table_window_with_theme_at(
        buf,
        frame.body_row,
        table_col,
        &columns,
        &rows,
        scroll,
        visible_rows,
        theme::table_theme(),
        Some(app.fleet_overlay.mission_picker_cursor),
        0,
        Some(&row_states),
    );
    if let Some(status) = app.fleet_overlay.mission_picker_status.as_deref() {
        write_clipped(
            buf,
            frame.body_row + standard_table_body_height(visible_rows),
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn draw_fleet_order_prompt(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let target_prompt = app.fleet_order_target_prompt();
    let target_default = app.fleet_order_target_default_value();
    let target_x_default = app.fleet_order_target_x_default_value();
    let target_x_input = app.fleet_order_target_x_display_input();
    let target_y_default = app.fleet_order_target_y_default_value();
    let target_y_input = app.fleet_order_target_y_display_input();
    let coordinate_rows = [
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Target XX ",
            default: &target_x_default,
            input: &target_x_input,
        },
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Target YY ",
            default: &target_y_default,
            input: &target_y_input,
        },
    ];
    let footer = match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::OrderTarget => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &target_prompt,
            default: &target_default,
            input: &app.fleet_overlay.order_input,
        },
        FleetOverlayPromptMode::OrderTargetX => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Target XX ",
            default: &target_x_default,
            input: &target_x_input,
        },
        FleetOverlayPromptMode::OrderTargetY => TableFooter::Stacked {
            rows: &coordinate_rows,
            active_row: 1,
        },
        FleetOverlayPromptMode::OrderConfirm => TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Confirm [Y]/N <Q> -> ",
            input: &app.fleet_overlay.order_confirm_input,
        },
        _ => unreachable!("fleet order prompt expected"),
    };
    let status = app.fleet_overlay.order_status.as_deref();
    let lines = if let Some(row) = app.selected_fleet_order_row() {
        vec![
            format!("Fleet #{}", row.fleet_number),
            format!("Location: {}", format_coords(row.coords)),
            format!("Current Order: {}", app.fleet_order_current_order_label()),
            format!("New Order: {}", app.fleet_order_new_order_label()),
            app.fleet_order_target_status_line(),
        ]
    } else {
        vec!["Selected fleet is no longer available.".to_string()]
    };
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .chain(status.iter().map(|line| line.chars().count()))
        .max()
        .unwrap_or(1);
    let body_height = lines.len() + usize::from(status.is_some());
    let frame = draw_overlay_frame_for_body_in_map(
        buf,
        map_frame,
        "ORDER FLEET",
        body_width,
        body_height,
        footer,
    );
    assert_overlay_body_write_fits(frame, "ORDER FLEET", body_width, body_height);
    if app.selected_fleet_order_row().is_none() {
        write_clipped(
            buf,
            frame.body_row,
            frame.body_col,
            frame.body_width,
            "Selected fleet is no longer available.",
            theme::error_style(),
        );
        return;
    }
    for (idx, line) in lines.iter().take(frame.body_height).enumerate().take(5) {
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            line,
            if idx == 4 {
                theme::dim_style()
            } else {
                theme::label_style()
            },
        );
    }
    if let Some(status) = status {
        write_clipped(
            buf,
            frame.body_row + lines.len(),
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn draw_group_fleet_order_prompt(
    buf: &mut PlayfieldBuffer,
    app: &DashApp,
    map_frame: MapWidgetFrame,
) {
    let target_prompt = app.fleet_order_target_prompt();
    let target_default = app.fleet_order_target_default_value();
    let target_x_default = app.fleet_order_target_x_default_value();
    let target_x_input = app.fleet_order_target_x_display_input();
    let target_y_default = app.fleet_order_target_y_default_value();
    let target_y_input = app.fleet_order_target_y_display_input();
    let coordinate_rows = [
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Target XX ",
            default: &target_x_default,
            input: &target_x_input,
        },
        TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Target YY ",
            default: &target_y_default,
            input: &target_y_input,
        },
    ];
    let footer = match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::OrderTarget => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: &target_prompt,
            default: &target_default,
            input: &app.fleet_overlay.order_input,
        },
        FleetOverlayPromptMode::OrderTargetX => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Target XX ",
            default: &target_x_default,
            input: &target_x_input,
        },
        FleetOverlayPromptMode::OrderTargetY => TableFooter::Stacked {
            rows: &coordinate_rows,
            active_row: 1,
        },
        FleetOverlayPromptMode::OrderConfirm => TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Confirm [Y]/N <Q> -> ",
            input: &app.fleet_overlay.order_confirm_input,
        },
        _ => unreachable!("group fleet order prompt expected"),
    };
    let selected_summary = app.selected_group_order_fleet_summary();
    let status = app.fleet_overlay.order_status.as_deref();
    let mut lines = match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::OrderConfirm => vec![
            format!("Stardate: {}", app.game_data.conquest.game_year()),
            format!("Selected fleets: {selected_summary}"),
            app.fleet_order_target_status_line(),
            format!("New Order: {}", app.fleet_order_new_order_label()),
        ],
        _ => vec![
            format!("Selected fleets: {selected_summary}"),
            app.fleet_order_target_status_line(),
            format!("New Order: {}", app.fleet_order_new_order_label()),
        ],
    };
    let body_width = GROUP_ORDER_BODY_WIDTH.min(max_overlay_body_width(map_frame).max(1));
    let wrapped_lines = lines
        .drain(..)
        .flat_map(|line| wrap_group_prompt_line(&line, body_width))
        .collect::<Vec<_>>();
    let status_rows = usize::from(status.is_some());
    let body_height = wrapped_lines.len() + status_rows;
    let frame = draw_overlay_frame_for_body_in_map(
        buf,
        map_frame,
        "GROUP FLEET ORDER",
        body_width,
        body_height,
        footer,
    );
    assert_overlay_body_write_fits(
        frame,
        "GROUP FLEET ORDER",
        body_width.min(frame.body_width),
        body_height,
    );
    for (idx, line) in wrapped_lines.iter().enumerate().take(frame.body_height) {
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            line,
            if matches!(
                app.fleet_overlay.prompt_mode,
                FleetOverlayPromptMode::OrderConfirm
            ) && idx == 2
            {
                theme::dim_style()
            } else {
                theme::label_style()
            },
        );
    }
    if let Some(status) = status {
        write_clipped(
            buf,
            frame.body_row + wrapped_lines.len(),
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn draw_starbase_move_prompt(buf: &mut PlayfieldBuffer, app: &DashApp, map_frame: MapWidgetFrame) {
    let footer = match app.fleet_overlay.prompt_mode {
        FleetOverlayPromptMode::StarbaseMoveDecision => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Halt or move <H>, <M> ",
            default: "M",
            input: &app.fleet_overlay.starbase_move_input,
        },
        FleetOverlayPromptMode::StarbaseMoveDestination => TableFooter::CommandInput {
            label: "COMMAND",
            prompt: "Destination ",
            default: &app
                .selected_starbase_move_row()
                .map(|row| {
                    format!(
                        "{},{}",
                        row.destination_coords[0], row.destination_coords[1]
                    )
                })
                .unwrap_or_default(),
            input: &app.fleet_overlay.starbase_move_input,
        },
        FleetOverlayPromptMode::StarbaseHaltConfirm => TableFooter::CommandPromptInput {
            label: "COMMAND",
            prompt: "Halt this starbase? [Y]/N <Q> -> ",
            input: &app.fleet_overlay.starbase_move_input,
        },
        _ => unreachable!("starbase move prompt expected"),
    };
    let lines = if let Some(row) = app.selected_starbase_move_row() {
        let mut lines = vec![
            format!("Starbase #{}", row.base_id),
            format!("Location: {}", format_coords(row.coords)),
            format!("Destination: {}", format_coords(row.destination_coords)),
            "Choose move/halt for the selected starbase.".to_string(),
        ];
        if let Some(status) = app.fleet_overlay.starbase_move_status.as_deref() {
            lines.push(status.to_string());
        }
        lines
    } else {
        vec!["Selected starbase is no longer available.".to_string()]
    };
    let body_width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1);
    let frame = draw_overlay_frame_for_body_in_map(
        buf,
        map_frame,
        "STARBASE MOVE/HALT",
        body_width,
        lines.len(),
        footer,
    );
    assert_overlay_body_write_fits(frame, "STARBASE MOVE/HALT", body_width, lines.len());
    if app.selected_starbase_move_row().is_none() {
        write_clipped(
            buf,
            frame.body_row,
            frame.body_col,
            frame.body_width,
            "Selected starbase is no longer available.",
            theme::error_style(),
        );
        return;
    }
    for (idx, line) in lines.iter().take(frame.body_height).enumerate().take(4) {
        write_clipped(
            buf,
            frame.body_row + idx,
            frame.body_col,
            frame.body_width,
            line,
            if idx == 3 {
                theme::dim_style()
            } else {
                theme::label_style()
            },
        );
    }
    if let Some(status) = app.fleet_overlay.starbase_move_status.as_deref() {
        write_clipped(
            buf,
            frame.body_row + 4,
            frame.body_col,
            frame.body_width,
            status,
            theme::error_style(),
        );
    }
}

fn wrap_group_prompt_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 || line.chars().count() <= width {
        return vec![line.to_string()];
    }
    if let Some((prefix, values)) = line.split_once(": ") {
        let mut rows = Vec::new();
        let prefix_text = format!("{prefix}: ");
        let prefix_width = prefix_text.chars().count();
        let continuation = " ".repeat(prefix_width);
        let mut current = prefix_text.clone();
        let mut current_width = prefix_width;
        for part in values.split(", ") {
            let token = if current_width > prefix_width {
                format!(", {part}")
            } else {
                part.to_string()
            };
            let token_width = token.chars().count();
            if current_width > prefix_width && current_width + token_width > width {
                rows.push(current);
                current = format!("{continuation}{part}");
                current_width = prefix_width + part.chars().count();
            } else {
                current.push_str(&token);
                current_width += token_width;
            }
        }
        rows.push(current);
        return rows;
    }
    vec![line.chars().take(width).collect()]
}

fn row_states_from_enabled_flags(flags: &[bool]) -> Vec<nc_ui::table::TableRowState> {
    flags
        .iter()
        .map(|enabled| {
            if *enabled {
                nc_ui::table::TableRowState::Normal
            } else {
                nc_ui::table::TableRowState::Disabled
            }
        })
        .collect()
}

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .map(|row| vec![row.id_label])
        .collect()
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<FleetOverlayRow> {
    let owner_slot = app.player_record_index_1_based as u8;
    let location_filter = app.fleet_overlay.location_filter;
    let mut rows = Vec::new();

    for (idx, fleet) in app.game_data.fleets.records.iter().enumerate() {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }
        if location_filter.is_some_and(|coords| fleet.current_location_coords_raw() != coords) {
            continue;
        }
        rows.push(FleetOverlayRow {
            key: FleetOverlayRowKey::Fleet(idx + 1),
            id_label: fleet.local_slot_word_raw().to_string(),
            coords: fleet.current_location_coords_raw(),
            order: fleet.standing_order_kind(),
            eta_label: fleet_list_eta_label(&app.game_data, idx),
            strength_key: fleet_strength_key(fleet),
            cells: vec![
                fleet.local_slot_word_raw().to_string(),
                if app
                    .fleet_overlay
                    .selected_fleet_record_indexes
                    .contains(&(idx + 1))
                {
                    "X".to_string()
                } else {
                    String::new()
                },
                format_coords(fleet.current_location_coords_raw()),
                order_abbrev(fleet.standing_order_kind()).to_string(),
                format_target(fleet.standing_order_target_coords_raw()),
                fleet.current_speed().to_string(),
                fleet_list_eta_label(&app.game_data, idx),
                fleet.rules_of_engagement().to_string(),
                fleet.army_count().to_string(),
                truncate(&fleet.ship_composition_summary(), COLUMNS[9].width),
            ],
        });
    }

    if location_filter.is_none() {
        for (idx, base) in app.game_data.bases.records.iter().enumerate() {
            if base.owner_empire_raw() != owner_slot || base.active_flag_raw() == 0 {
                continue;
            }
            rows.push(FleetOverlayRow {
                key: FleetOverlayRowKey::Starbase(idx + 1),
                id_label: format!("SB{}", base.base_id_raw()),
                coords: base.coords_raw(),
                order: Order::GuardStarbase,
                eta_label: starbase_eta_label(base.coords_raw(), base.trailing_coords_raw()),
                strength_key: (0, 0, 0, 0, 0, 0, u16::from(base.base_id_raw())),
                cells: vec![
                    format!("SB{}", base.base_id_raw()),
                    String::new(),
                    format_coords(base.coords_raw()),
                    String::from("Gs"),
                    String::from("--"),
                    String::from("0"),
                    starbase_eta_label(base.coords_raw(), base.trailing_coords_raw()),
                    String::from("0"),
                    String::from("0"),
                    String::from("Starbase"),
                ],
            });
        }
    }

    rows.retain(|row| match app.fleet_overlay.filter {
        FleetOverlayFilter::All => true,
        FleetOverlayFilter::Holding => row.order == Order::HoldPosition,
        FleetOverlayFilter::Moving => matches!(
            row.order,
            Order::MoveOnly
                | Order::SeekHome
                | Order::PatrolSector
                | Order::ViewWorld
                | Order::ScoutSector
                | Order::ScoutSolarSystem
                | Order::ColonizeWorld
                | Order::JoinAnotherFleet
                | Order::RendezvousSector
                | Order::Salvage
        ),
        FleetOverlayFilter::Combat => matches!(
            row.order,
            Order::GuardStarbase
                | Order::GuardBlockadeWorld
                | Order::BombardWorld
                | Order::InvadeWorld
                | Order::BlitzWorld
        ),
    });

    rows.sort_by(|left, right| match app.fleet_overlay.sort {
        FleetOverlaySort::Id => right.id_label.cmp(&left.id_label),
        FleetOverlaySort::Location => left
            .coords
            .cmp(&right.coords)
            .then_with(|| right.id_label.cmp(&left.id_label)),
        FleetOverlaySort::Order => left.cells[3]
            .cmp(&right.cells[3])
            .then_with(|| right.id_label.cmp(&left.id_label)),
        FleetOverlaySort::Eta => eta_sort_key(&left.eta_label)
            .cmp(&eta_sort_key(&right.eta_label))
            .then_with(|| right.id_label.cmp(&left.id_label)),
        FleetOverlaySort::Strength => right
            .strength_key
            .cmp(&left.strength_key)
            .then_with(|| right.id_label.cmp(&left.id_label)),
    });

    rows
}

fn clamp_scroll(scroll: usize, selected: usize, max_rows: usize, total_rows: usize) -> usize {
    if max_rows == 0 || total_rows <= max_rows {
        return 0;
    }
    if selected < scroll {
        return selected;
    }
    if selected >= scroll + max_rows {
        return selected + 1 - max_rows;
    }
    scroll.min(total_rows.saturating_sub(max_rows))
}

fn format_coords(coords: [u8; 2]) -> String {
    format_sector_coords_table(coords)
}

fn format_target(coords: [u8; 2]) -> String {
    if coords[0] == 0 || coords[1] == 0 {
        String::from("--")
    } else {
        format_coords(coords)
    }
}

fn truncate(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

pub(crate) fn sync_cursor_to_jump_input(app: &mut DashApp) -> bool {
    let rows = selection_rows(app);
    let Some(matched) = table_selection::find_typed_jump(&rows, 0, &app.fleet_overlay.jump_input)
    else {
        return false;
    };
    app.fleet_overlay.selected = matched.index;
    matched.is_terminal_exact_match
}

fn eta_sort_key(label: &str) -> (u8, u16) {
    match label {
        "0" => (0, 0),
        "S" => (1, 0),
        "X" => (3, 0),
        _ => label
            .parse::<u16>()
            .map(|value| (0, value))
            .unwrap_or((2, 0)),
    }
}

fn fleet_strength_key(fleet: &nc_data::FleetRecord) -> (u16, u16, u16, u16, u8, u16, u16) {
    (
        fleet.battleship_count(),
        fleet.cruiser_count(),
        fleet.destroyer_count(),
        fleet.troop_transport_count(),
        fleet.scout_count(),
        fleet.etac_count(),
        fleet.local_slot_word_raw(),
    )
}

#[cfg(test)]
mod tests {
    use super::{HOTKEYS, draw, table_rows};
    use crate::app::state::{ActiveOverlay, DashApp, FleetOrderScope, FleetOverlayPromptMode};
    use crate::layout::dashboard_layout;
    use nc_data::{GameStateBuilder, Order};
    use nc_ui::{PlayfieldBuffer, ScreenGeometry};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn fleet_order_confirm_footer_renders_standard_yes_no_prompt() {
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
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        );
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        app.fleet_overlay.order_target_x_input = "03".to_string();
        app.fleet_overlay.order_target_y_input = "02".to_string();
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderConfirm;

        let layout = dashboard_layout(&app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::theme::body_style(),
        );

        draw(&mut buffer, &app, layout.widgets.center_map);

        let footer_line = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains("COMMAND <- Confirm [Y]/N "))
            .expect("fleet order confirm footer");

        assert!(footer_line.contains("COMMAND <- Confirm [Y]/N <Q> ->"));
        assert!(!footer_line.contains("COMMAND <- Confirm [Y]/N [Y] <Q> ->"));
        assert_eq!(footer_line.matches("<Q>").count(), 1);
    }

    #[test]
    fn fleet_target_x_footer_shows_adaptive_default() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;

        let lines = render_lines_for_prompt(&app);
        let expected = format!(
            "COMMAND <- Target XX [{}] <Q> ->",
            app.fleet_order_target_x_default_value()
        );

        assert!(lines.iter().any(|line| line.contains(&expected)));
    }

    #[test]
    fn fleet_target_y_step_keeps_x_prompt_visible_and_shows_adaptive_y_default() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetY;

        let lines = render_lines_for_prompt(&app);
        let expected_x = format!(
            "COMMAND <- Target XX [{}] <Q> ->",
            app.fleet_order_target_x_default_value()
        );
        let expected_y = format!(
            "COMMAND <- Target YY [{}] <Q> ->",
            app.fleet_order_target_y_default_value()
        );

        assert!(lines.iter().any(|line| line.contains(&expected_x)));
        assert!(lines.iter().any(|line| line.contains(&expected_y)));
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("COMMAND <- Target XX "))
                .count(),
            1
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("COMMAND <- Target YY "))
                .count(),
            1
        );
    }

    #[test]
    fn fleet_target_y_footer_stacks_x_above_y() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetY;
        app.fleet_overlay.order_target_x_input = "03".to_string();

        let rows = render_prompt_rows(&app);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].1.contains("COMMAND <- Target XX "));
        assert!(rows[0].1.contains("03"));
        assert!(rows[1].1.contains("COMMAND <- Target YY "));
        assert_eq!(rows[1].0, rows[0].0 + 1);
    }

    #[test]
    fn fleet_target_x_step_renders_command_line_in_body_like_nc_game() {
        let mut app = dash_app();
        app.overlay = ActiveOverlay::FleetList;
        app.open_selected_fleet_order_flow();
        app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;

        let lines = render_lines_for_prompt(&app);

        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("COMMAND <- Target XX "))
                .count(),
            1
        );
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("COMMAND <- Target YY "))
        );
    }

    #[test]
    fn fleet_browse_hotkeys_match_supported_commands() {
        assert_eq!(HOTKEYS, "? F S O SPACE <Q>");
    }

    #[test]
    fn group_fleet_order_confirm_footer_renders_standard_yes_no_prompt() {
        let mut app = dash_app();
        configure_group_confirm_prompt(&mut app, &[0, 1]);

        let layout = dashboard_layout(&app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::theme::body_style(),
        );

        draw(&mut buffer, &app, layout.widgets.center_map);

        let footer_line = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains("COMMAND <- Confirm [Y]/N "))
            .expect("group fleet order confirm footer");

        assert!(footer_line.contains("COMMAND <- Confirm [Y]/N <Q> ->"));
        assert!(!footer_line.contains("COMMAND <- Confirm [Y]/N [Y] <Q> ->"));
    }

    #[test]
    fn group_target_y_step_keeps_x_prompt_visible_and_shows_adaptive_y_default() {
        let mut app = dash_app();
        configure_group_confirm_prompt(&mut app, &[0, 1]);
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetY;
        app.fleet_overlay.order_target_x_input.clear();
        app.fleet_overlay.order_target_y_input.clear();

        let lines = render_lines_for_prompt(&app);
        let expected_x = format!(
            "COMMAND <- Target XX [{}] <Q> ->",
            app.fleet_order_target_x_default_value()
        );
        let expected_y = format!(
            "COMMAND <- Target YY [{}] <Q> ->",
            app.fleet_order_target_y_default_value()
        );

        assert!(lines.iter().any(|line| line.contains(&expected_x)));
        assert!(lines.iter().any(|line| line.contains(&expected_y)));
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("COMMAND <- Target XX "))
                .count(),
            1
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("COMMAND <- Target YY "))
                .count(),
            1
        );
    }

    #[test]
    fn group_target_y_footer_stacks_x_above_y() {
        let mut app = dash_app();
        configure_group_confirm_prompt(&mut app, &[0, 1]);
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetY;
        app.fleet_overlay.order_target_x_input = "03".to_string();

        let rows = render_prompt_rows(&app);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].1.contains("COMMAND <- Target XX "));
        assert!(rows[0].1.contains("03"));
        assert!(rows[1].1.contains("COMMAND <- Target YY "));
        assert_eq!(rows[1].0, rows[0].0 + 1);
    }

    #[test]
    fn group_target_x_step_renders_command_line_in_body_like_nc_game() {
        let mut app = dash_app();
        configure_group_confirm_prompt(&mut app, &[0, 1]);
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderTargetX;
        app.fleet_overlay.order_target_x_input.clear();

        let lines = render_lines_for_prompt(&app);

        assert_eq!(
            lines
                .iter()
                .filter(|line| line.contains("COMMAND <- Target XX "))
                .count(),
            1
        );
        assert!(
            !lines
                .iter()
                .any(|line| line.contains("COMMAND <- Target YY "))
        );
    }

    #[test]
    fn starbase_prompts_follow_standard_command_line_grammar() {
        let mut app = dash_app_with_starbase();
        app.overlay = ActiveOverlay::FleetList;
        let starbase_index = table_rows(&app)
            .iter()
            .position(|row| matches!(row.key, crate::app::state::FleetOverlayRowKey::Starbase(_)))
            .expect("starbase row");
        app.fleet_overlay.selected = starbase_index;
        app.open_selected_fleet_order_flow();

        let layout = dashboard_layout(&app);
        let mut decision_buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::theme::body_style(),
        );
        draw(&mut decision_buffer, &app, layout.widgets.center_map);
        let decision_line = (0..decision_buffer.height())
            .map(|row| decision_buffer.plain_line(row))
            .find(|line| line.contains("COMMAND <- Halt or move"))
            .expect("starbase decision footer");
        assert!(decision_line.contains("COMMAND <- Halt or move <H>, <M> [M] <Q> ->"));

        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::StarbaseHaltConfirm;
        let mut confirm_buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::theme::body_style(),
        );
        draw(&mut confirm_buffer, &app, layout.widgets.center_map);
        let confirm_line = (0..confirm_buffer.height())
            .map(|row| confirm_buffer.plain_line(row))
            .find(|line| line.contains("Halt this starbase?"))
            .expect("starbase halt confirm footer");
        assert!(confirm_line.contains("COMMAND <- Halt this starbase? [Y]/N <Q> ->"));
        assert!(!confirm_line.contains("[Y] <Q> ->"));
    }

    #[test]
    fn fleet_table_marks_selected_fleets_and_leaves_starbases_blank() {
        let mut app = dash_app_with_starbase();
        let selected_record = table_rows(&app)
            .into_iter()
            .find_map(|row| match row.key {
                crate::app::state::FleetOverlayRowKey::Fleet(record_index) => Some(record_index),
                crate::app::state::FleetOverlayRowKey::Starbase(_) => None,
            })
            .expect("fleet row");
        app.fleet_overlay
            .selected_fleet_record_indexes
            .insert(selected_record);

        let rows = table_rows(&app);
        let fleet_row = rows
            .iter()
            .find(|row| row.key == crate::app::state::FleetOverlayRowKey::Fleet(selected_record))
            .expect("selected fleet row");
        let starbase_row = rows
            .iter()
            .find(|row| matches!(row.key, crate::app::state::FleetOverlayRowKey::Starbase(_)))
            .expect("starbase row");

        assert_eq!(fleet_row.cells[1], "X");
        assert!(starbase_row.cells[1].is_empty());
    }

    #[test]
    fn group_fleet_order_prompt_keeps_stable_modal_width() {
        let mut short = dash_app();
        configure_group_confirm_prompt(&mut short, &[0]);

        let mut wide = dash_app();
        configure_group_confirm_prompt(&mut wide, &[0, 1, 2, 3]);

        let short_title = render_group_order_title(&short);
        let wide_title = render_group_order_title(&wide);

        assert_eq!(short_title, wide_title);
        assert!(
            render_group_order_body(&wide)
                .iter()
                .any(|line| line.contains("Selected fleets:"))
        );
        assert!(
            render_group_order_body(&wide)
                .iter()
                .any(|line| line.contains("New Order:"))
        );
    }

    fn dash_app() -> DashApp {
        DashApp::new_for_tests(
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
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        )
    }

    fn dash_app_with_starbase() -> DashApp {
        DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .with_guard_starbase(1, 1, [16, 13], 1)
                .build_initialized_baseline()
                .expect("baseline with starbase"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            ScreenGeometry::new(160, 45),
            ScreenGeometry::new(0, 0),
            1,
        )
    }

    fn configure_group_confirm_prompt(app: &mut DashApp, selected_row_indexes: &[usize]) {
        app.overlay = ActiveOverlay::FleetList;
        let rows = table_rows(app);
        let selected_records = selected_row_indexes
            .iter()
            .map(|index| match rows[*index].key {
                crate::app::state::FleetOverlayRowKey::Fleet(record_index) => record_index,
                crate::app::state::FleetOverlayRowKey::Starbase(_) => {
                    panic!("expected fleet row in test selection")
                }
            })
            .collect::<Vec<_>>();
        app.fleet_overlay.selected = selected_row_indexes[0];
        app.fleet_overlay.active_row_key = Some(rows[selected_row_indexes[0]].key);
        app.fleet_overlay.order_scope = FleetOrderScope::Group;
        app.fleet_overlay.prompt_mode = FleetOverlayPromptMode::OrderConfirm;
        app.fleet_overlay.order_mission_code = Some(Order::MoveOnly.to_raw());
        app.fleet_overlay.order_target_x_input = "03".to_string();
        app.fleet_overlay.order_target_y_input = "02".to_string();
        app.fleet_overlay.selected_fleet_record_indexes = selected_records.into_iter().collect();
    }

    fn render_group_order_title(app: &DashApp) -> String {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::theme::body_style(),
        );
        draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .find(|line| line.contains("GROUP FLEET ORDER"))
            .expect("group fleet order title")
    }

    fn render_group_order_body(app: &DashApp) -> Vec<String> {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::theme::body_style(),
        );
        draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .filter(|line| {
                line.contains("Selected fleets:")
                    || line.contains("New Order:")
                    || line.contains("Stardate:")
            })
            .collect()
    }

    fn render_lines_for_prompt(app: &DashApp) -> Vec<String> {
        render_prompt_rows(app)
            .into_iter()
            .map(|(_, line)| line)
            .collect()
    }

    fn render_prompt_rows(app: &DashApp) -> Vec<(usize, String)> {
        let layout = dashboard_layout(app);
        let mut buffer = PlayfieldBuffer::new(
            app.geometry.width(),
            app.geometry.height(),
            crate::theme::body_style(),
        );
        draw(&mut buffer, app, layout.widgets.center_map);
        (0..buffer.height())
            .map(|row| (row, buffer.plain_line(row)))
            .filter(|(_, line)| line.contains("COMMAND <- Target"))
            .collect()
    }
}
