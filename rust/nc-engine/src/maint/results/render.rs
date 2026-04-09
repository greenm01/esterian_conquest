use nc_data::CoreGameData;

use super::{fleet_label, owned_fleet_source_clause_from_idx};

pub(super) fn join_host_retarget_text(
    previous_host_fleet_number: Option<u8>,
    new_host_fleet_number: Option<u8>,
) -> String {
    match (previous_host_fleet_number, new_host_fleet_number) {
        (Some(previous), Some(new)) => format!(
            " Join mission report: Our intended host fleet ({}) has moved. We are now joining the {} instead.",
            fleet_label(previous),
            fleet_label(new),
        ),
        (Some(previous), None) => format!(
            " Join mission report: Our intended host fleet ({}) has moved. We are now joining a new host fleet instead.",
            fleet_label(previous),
        ),
        (None, Some(new)) => format!(
            " Join mission report: Our intended host fleet has moved. We are now joining the {} instead.",
            fleet_label(new),
        ),
        (None, None) => {
            " Join mission report: Our intended host fleet has moved. We are now joining a new host fleet instead.".to_string()
        }
    }
}

pub(super) fn join_host_destroyed_text(
    destroyed_host_fleet_number: Option<u8>,
    coords: [u8; 2],
) -> String {
    let [x, y] = coords;
    match destroyed_host_fleet_number {
        Some(fleet_number) => format!(
            " Join mission report: Our intended host fleet ({}) was destroyed. We are holding our position in Sector({x},{y}) and awaiting orders.",
            fleet_label(fleet_number),
        ),
        None => format!(
            " Join mission report: Our intended host fleet was destroyed. We are holding our position in Sector({x},{y}) and awaiting orders."
        ),
    }
}

pub(super) fn mission_retarget_source(
    game_data: &CoreGameData,
    fleet_idx: usize,
    current_coords: [u8; 2],
) -> String {
    owned_fleet_source_clause_from_idx(
        game_data,
        fleet_idx,
        &format!("Sector({},{})", current_coords[0], current_coords[1]),
    )
}
