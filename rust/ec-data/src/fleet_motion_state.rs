use crate::records::fleet::FleetRecord;

const EXACT_POSITION_MAGIC: u8 = 0x42;
const EXACT_POSITION_SCALE: f64 = 256.0;

pub fn decode_exact_position(fleet: &FleetRecord) -> Option<[f64; 2]> {
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

pub fn store_exact_position(fleet: &mut FleetRecord, exact: [f64; 2]) {
    let x_fixed = encode_exact_coord(exact[0]);
    let y_fixed = encode_exact_coord(exact[1]);
    fleet.raw[0x1a..0x1c].copy_from_slice(&x_fixed.to_le_bytes());
    fleet.raw[0x1c..0x1e].copy_from_slice(&y_fixed.to_le_bytes());
    fleet.raw[0x1e] = EXACT_POSITION_MAGIC;
}

pub fn clear_exact_position(fleet: &mut FleetRecord) {
    fleet.raw[0x1a] = 0;
    fleet.raw[0x1b] = 0;
    fleet.raw[0x1c] = 0;
    fleet.raw[0x1d] = 0;
    fleet.raw[0x1e] = 0;
}

pub fn reset_motion_state_for_new_orders(fleet: &mut FleetRecord) {
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0x00;
    clear_exact_position(fleet);
}

fn encode_exact_coord(value: f64) -> u16 {
    (value * EXACT_POSITION_SCALE)
        .round()
        .clamp(0.0, f64::from(u16::MAX)) as u16
}
