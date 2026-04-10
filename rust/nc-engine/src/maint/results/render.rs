use super::owned_fleet_source_clause;

pub(super) fn mission_retarget_source(
    reporting_fleet_number: Option<u8>,
    current_coords: [u8; 2],
) -> String {
    owned_fleet_source_clause(
        reporting_fleet_number,
        &format!("Sector({},{})", current_coords[0], current_coords[1]),
    )
}
