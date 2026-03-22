use crate::pathfinding::{PlannedRoute, VisibleHazardIntel};
use crate::records::fleet::FleetRecord;

const EXACT_POSITION_MAGIC: u8 = 0x42;
const EXACT_POSITION_SCALE: f64 = 256.0;

pub(crate) fn decode_exact_position(fleet: &FleetRecord) -> Option<[f64; 2]> {
    if fleet.raw[0x1e] != EXACT_POSITION_MAGIC {
        return None;
    }

    let x_fixed = u16::from_le_bytes([fleet.raw[0x1a], fleet.raw[0x1b]]);
    let y_fixed = u16::from_le_bytes([fleet.raw[0x1c], fleet.raw[0x1d]]);
    Some([
        f64::from(x_fixed) / EXACT_POSITION_SCALE,
        f64::from(y_fixed) / EXACT_POSITION_SCALE,
    ])
}

pub(crate) fn store_exact_position(fleet: &mut FleetRecord, exact: [f64; 2]) {
    let x_fixed = encode_exact_coord(exact[0]);
    let y_fixed = encode_exact_coord(exact[1]);
    fleet.raw[0x1a..0x1c].copy_from_slice(&x_fixed.to_le_bytes());
    fleet.raw[0x1c..0x1e].copy_from_slice(&y_fixed.to_le_bytes());
    fleet.raw[0x1e] = EXACT_POSITION_MAGIC;
}

pub(crate) fn clear_exact_position(fleet: &mut FleetRecord) {
    fleet.raw[0x1a] = 0;
    fleet.raw[0x1b] = 0;
    fleet.raw[0x1c] = 0;
    fleet.raw[0x1d] = 0;
    fleet.raw[0x1e] = 0;
}

pub(crate) fn reset_motion_state_for_new_orders(fleet: &mut FleetRecord) {
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0x00;
    clear_exact_position(fleet);
}

pub(crate) fn visible_hazard_intel_is_empty(intel: &VisibleHazardIntel) -> bool {
    intel.foreign_worlds.is_empty()
        && intel.foreign_starbases.is_empty()
        && intel.foreign_fleets.is_empty()
        && intel.hostile_blockades.is_empty()
        && intel.hostile_homeworlds.is_empty()
}

pub(crate) fn advance_exact_position(
    exact_start: [f64; 2],
    target: [u8; 2],
    travel_distance: f64,
    route: Option<&PlannedRoute>,
    use_route_geometry: bool,
) -> [f64; 2] {
    if travel_distance <= 0.0 {
        return exact_start;
    }

    if use_route_geometry {
        if let Some(route) = route {
            return advance_along_route(exact_start, route, travel_distance);
        }
    }

    advance_toward(
        exact_start,
        [f64::from(target[0]), f64::from(target[1])],
        travel_distance,
    )
}

pub(crate) fn rounded_coords_from_exact(exact: [f64; 2], target: [u8; 2]) -> [u8; 2] {
    let rounded_x = exact[0].round().clamp(0.0, f64::from(u8::MAX)) as u8;
    let rounded_y = exact[1].round().clamp(0.0, f64::from(u8::MAX)) as u8;
    if rounded_x == target[0] && rounded_y == target[1] {
        target
    } else {
        [rounded_x, rounded_y]
    }
}

fn advance_along_route(
    exact_start: [f64; 2],
    route: &PlannedRoute,
    mut travel_distance: f64,
) -> [f64; 2] {
    let mut current = exact_start;

    for step in route.steps.iter().skip(1) {
        let waypoint = [f64::from(step.coords[0]), f64::from(step.coords[1])];
        let dx = waypoint[0] - current[0];
        let dy = waypoint[1] - current[1];
        let segment_distance = (dx * dx + dy * dy).sqrt();
        if segment_distance <= f64::EPSILON {
            current = waypoint;
            continue;
        }

        if travel_distance < segment_distance {
            return [
                current[0] + dx * (travel_distance / segment_distance),
                current[1] + dy * (travel_distance / segment_distance),
            ];
        }

        current = waypoint;
        travel_distance -= segment_distance;
    }

    current
}

fn advance_toward(from: [f64; 2], to: [f64; 2], travel_distance: f64) -> [f64; 2] {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let distance = (dx * dx + dy * dy).sqrt();
    if distance <= f64::EPSILON || travel_distance >= distance {
        return to;
    }

    [
        from[0] + dx * (travel_distance / distance),
        from[1] + dy * (travel_distance / distance),
    ]
}

fn encode_exact_coord(value: f64) -> u16 {
    (value * EXACT_POSITION_SCALE)
        .round()
        .clamp(0.0, f64::from(u16::MAX)) as u16
}
