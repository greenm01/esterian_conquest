//! F overlay: dashboard-sized fleet and starbase command table.

use nc_ui::PlayfieldBuffer;
use nc_ui::coords::format_sector_coords_table;
use nc_ui::table::{
    TableColumn, TableFooter, TableWidthMode, centered_table_start_col, resolve_table_columns,
    table_render_width, write_table_window_with_theme_at,
};
use nc_ui::table_selection;

use crate::app::state::{DashApp, FleetOverlayFilter, FleetOverlayPromptMode, FleetOverlaySort};
use crate::overlays::frame::{draw_overlay_frame_for_body, write_clipped};
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

pub(crate) const HOTKEYS: &str = "? F S O C M T I <Q>";
pub(crate) const FILTER_HOTKEYS: &str = "? A H M C <Q>";
pub(crate) const SORT_HOTKEYS: &str = "? I L O E T <Q>";
const COLUMNS: [TableColumn<'static>; 9] = [
    TableColumn::right("ID", 4),
    TableColumn::left("Location", 8),
    TableColumn::left("Order", 5),
    TableColumn::left("Target", 8),
    TableColumn::right("Spd", 3),
    TableColumn::right("ETA", 4),
    TableColumn::right("ROE", 3),
    TableColumn::right("ARs", 3),
    TableColumn::left_flex("Ships / Forces", 24, 1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FleetOverlayRowKey {
    Fleet(usize),
    Starbase(usize),
}

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

pub fn draw(buf: &mut PlayfieldBuffer, app: &DashApp) {
    let rows = table_rows(app);
    let selected = app.fleet_overlay.selected.min(rows.len().saturating_sub(1));
    let selected_default = rows
        .get(selected)
        .map(|row| row.id_label.as_str());
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
    };
    let table_cells = rows
        .iter()
        .map(|row| row.cells.clone())
        .collect::<Vec<_>>();

    let desired_visible_rows = table_cells.len().clamp(1, buf.height().saturating_sub(10));
    let columns = resolve_table_columns(
        &COLUMNS,
        &table_cells,
        buf.width().saturating_sub(12),
        false,
        TableWidthMode::Compact,
    );
    let body_width = table_render_width(&columns)
        .max("You have no active fleets or starbases.".chars().count() + 4);
    let frame = draw_overlay_frame_for_body(
        buf,
        "FLEET LIST",
        body_width,
        desired_visible_rows + 4,
        footer,
    );
    let visible_rows = frame.body_height.saturating_sub(4);
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

pub(crate) fn selection_rows(app: &DashApp) -> Vec<Vec<String>> {
    table_rows(app)
        .into_iter()
        .map(|row| vec![row.id_label])
        .collect()
}

pub(crate) fn table_rows(app: &DashApp) -> Vec<FleetOverlayRow> {
    let owner_slot = app.player_record_index_1_based as u8;
    let mut rows = Vec::new();

    for (idx, fleet) in app.game_data.fleets.records.iter().enumerate() {
        if fleet.owner_empire_raw() != owner_slot || !fleet.has_any_force() {
            continue;
        }
        rows.push(FleetOverlayRow {
            key: FleetOverlayRowKey::Fleet(idx + 1),
            id_label: fleet.local_slot_word_raw().to_string(),
            coords: fleet.current_location_coords_raw(),
            order: fleet.standing_order_kind(),
            eta_label: String::from("--"),
            strength_key: fleet_strength_key(fleet),
            cells: vec![
                fleet.local_slot_word_raw().to_string(),
                format_coords(fleet.current_location_coords_raw()),
                order_abbrev(fleet.standing_order_kind()).to_string(),
                format_target(fleet.standing_order_target_coords_raw()),
                fleet.current_speed().to_string(),
                String::from("--"),
                fleet.rules_of_engagement().to_string(),
                fleet.army_count().to_string(),
                truncate(&fleet.ship_composition_summary(), COLUMNS[8].width),
            ],
        });
    }

    for (idx, base) in app.game_data.bases.records.iter().enumerate() {
        if base.owner_empire_raw() != owner_slot || base.active_flag_raw() == 0 {
            continue;
        }
        rows.push(FleetOverlayRow {
            key: FleetOverlayRowKey::Starbase(idx + 1),
            id_label: format!("SB{}", base.base_id_raw()),
            coords: base.coords_raw(),
            order: Order::GuardStarbase,
            eta_label: String::from("--"),
            strength_key: (0, 0, 0, 0, 0, 0, u16::from(base.base_id_raw())),
            cells: vec![
                format!("SB{}", base.base_id_raw()),
                format_coords(base.coords_raw()),
                String::from("Gs"),
                String::from("--"),
                String::from("0"),
                String::from("--"),
                String::from("0"),
                String::from("0"),
                String::from("Starbase"),
            ],
        });
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
        FleetOverlaySort::Order => left
            .cells[2]
            .cmp(&right.cells[2])
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
        _ => label.parse::<u16>().map(|value| (0, value)).unwrap_or((2, 0)),
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
