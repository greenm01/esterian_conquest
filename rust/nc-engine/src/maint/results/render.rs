use super::{fleet_label, owned_fleet_source_clause};

pub(super) fn join_host_retarget_text(
    previous_host_fleet_number: Option<u8>,
    new_host_fleet_number: Option<u8>,
) -> String {
    match (previous_host_fleet_number, new_host_fleet_number) {
        (Some(previous), Some(new)) => format!(
            " Join mission report: Our intended host fleet ({}) merged into the {}. We are now joining that surviving fleet instead.",
            fleet_label(previous),
            fleet_label(new),
        ),
        (Some(previous), None) => format!(
            " Join mission report: Our intended host fleet ({}) merged into another surviving host fleet. We are now joining that surviving fleet instead.",
            fleet_label(previous),
        ),
        (None, Some(new)) => format!(
            " Join mission report: Our intended host fleet merged into the {}. We are now joining that surviving fleet instead.",
            fleet_label(new),
        ),
        (None, None) => {
            " Join mission report: Our intended host fleet merged into another surviving host fleet. We are now joining that surviving fleet instead.".to_string()
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
    reporting_fleet_number: Option<u8>,
    current_coords: [u8; 2],
) -> String {
    owned_fleet_source_clause(
        reporting_fleet_number,
        &format!("Sector({},{})", current_coords[0], current_coords[1]),
    )
}
