use crate::support::{copy_array, ParseError};
use crate::FLEET_RECORD_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRecord {
    pub raw: [u8; FLEET_RECORD_SIZE],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetStandingOrderKind {
    HoldPosition,
    MoveOnly,
    SeekHome,
    PatrolSector,
    GuardStarbase,
    GuardBlockadeWorld,
    BombardWorld,
    InvadeWorld,
    BlitzWorld,
    ViewWorld,
    ScoutSector,
    ScoutSolarSystem,
    ColonizeWorld,
    JoinAnotherFleet,
    RendezvousSector,
    Salvage,
    Unknown(u8),
}

impl FleetStandingOrderKind {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::HoldPosition,
            1 => Self::MoveOnly,
            2 => Self::SeekHome,
            3 => Self::PatrolSector,
            4 => Self::GuardStarbase,
            5 => Self::GuardBlockadeWorld,
            6 => Self::BombardWorld,
            7 => Self::InvadeWorld,
            8 => Self::BlitzWorld,
            9 => Self::ViewWorld,
            10 => Self::ScoutSector,
            11 => Self::ScoutSolarSystem,
            12 => Self::ColonizeWorld,
            13 => Self::JoinAnotherFleet,
            14 => Self::RendezvousSector,
            15 => Self::Salvage,
            other => Self::Unknown(other),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HoldPosition => "hold",
            Self::MoveOnly => "move",
            Self::SeekHome => "seek_home",
            Self::PatrolSector => "patrol",
            Self::GuardStarbase => "guard_starbase",
            Self::GuardBlockadeWorld => "guard_blockade",
            Self::BombardWorld => "bombard",
            Self::InvadeWorld => "invade",
            Self::BlitzWorld => "blitz",
            Self::ViewWorld => "view",
            Self::ScoutSector => "scout_sector",
            Self::ScoutSolarSystem => "scout_system",
            Self::ColonizeWorld => "colonize",
            Self::JoinAnotherFleet => "join_fleet",
            Self::RendezvousSector => "rendezvous",
            Self::Salvage => "salvage",
            Self::Unknown(_) => "unknown",
        }
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::HoldPosition => "Hold position",
            Self::MoveOnly => "Move fleet",
            Self::SeekHome => "Seek home",
            Self::PatrolSector => "Patrol sector",
            Self::GuardStarbase => "Guard starbase",
            Self::GuardBlockadeWorld => "Guard/blockade world",
            Self::BombardWorld => "Bombard world",
            Self::InvadeWorld => "Invade world",
            Self::BlitzWorld => "Blitz world",
            Self::ViewWorld => "View world",
            Self::ScoutSector => "Scout sector",
            Self::ScoutSolarSystem => "Scout solar system",
            Self::ColonizeWorld => "Colonize world",
            Self::JoinAnotherFleet => "Join another fleet",
            Self::RendezvousSector => "Rendezvous at sector",
            Self::Salvage => "Salvage",
            Self::Unknown(_) => "Unknown order",
        }
    }
}

impl FleetRecord {
    pub fn local_slot_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x00], self.raw[0x01]])
    }
    pub fn local_slot(&self) -> u8 { self.raw[0x00] }
    pub fn next_fleet_link_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x03], self.raw[0x04]])
    }
    pub fn next_fleet_id(&self) -> u8 { self.raw[0x03] }
    pub fn fleet_id_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }
    pub fn fleet_id(&self) -> u8 { self.raw[0x05] }
    pub fn previous_fleet_id(&self) -> u8 { self.raw[0x07] }
    pub fn max_speed(&self) -> u8 { self.raw[0x09] }
    pub fn current_speed(&self) -> u8 { self.raw[0x0A] }
    pub fn set_current_speed(&mut self, value: u8) { self.raw[0x0A] = value; }
    pub fn current_location_coords_raw(&self) -> [u8; 2] { [self.raw[0x0B], self.raw[0x0C]] }
    pub fn mission_param_bytes(&self) -> &[u8] { &self.raw[0x1F..=0x21] }
    pub fn standing_order_code_raw(&self) -> u8 { self.raw[0x1F] }
    pub fn set_standing_order_code_raw(&mut self, value: u8) { self.raw[0x1F] = value; }
    pub fn standing_order_kind(&self) -> FleetStandingOrderKind {
        FleetStandingOrderKind::from_raw(self.standing_order_code_raw())
    }
    pub fn standing_order_target_coords_raw(&self) -> [u8; 2] { [self.raw[0x20], self.raw[0x21]] }
    pub fn set_standing_order_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x20] = coords[0];
        self.raw[0x21] = coords[1];
    }
    pub fn mission_aux_bytes(&self) -> [u8; 2] { [self.raw[0x22], self.raw[0x23]] }
    pub fn guard_starbase_index_raw(&self) -> u8 { self.raw[0x22] }
    pub fn guard_starbase_enable_raw(&self) -> u8 { self.raw[0x23] }
    pub fn set_mission_aux_bytes(&mut self, value: [u8; 2]) {
        self.raw[0x22] = value[0];
        self.raw[0x23] = value[1];
    }
    pub fn tuple_a_payload_raw(&self) -> [u8; 5] { copy_array(&self.raw[0x0D..0x12]) }
    pub fn set_tuple_a_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x0D..0x12].copy_from_slice(&payload);
    }
    pub fn tuple_b_payload_raw(&self) -> [u8; 5] { copy_array(&self.raw[0x13..0x18]) }
    pub fn set_tuple_b_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x13..0x18].copy_from_slice(&payload);
    }
    pub fn tuple_c_payload_raw(&self) -> [u8; 5] { copy_array(&self.raw[0x19..0x1E]) }
    pub fn set_tuple_c_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x19..0x1E].copy_from_slice(&payload);
    }

    pub fn standing_order_summary(&self) -> String {
        let [x, y] = self.standing_order_target_coords_raw();
        match self.standing_order_kind() {
            FleetStandingOrderKind::HoldPosition => "Hold position".to_string(),
            FleetStandingOrderKind::MoveOnly => format!("Move fleet to Sector ({x},{y})"),
            FleetStandingOrderKind::SeekHome => "Seek home".to_string(),
            FleetStandingOrderKind::PatrolSector => format!("Patrol Sector ({x},{y})"),
            FleetStandingOrderKind::GuardStarbase => format!("Guard starbase at Sector ({x},{y})"),
            FleetStandingOrderKind::GuardBlockadeWorld => format!("Guard/blockade world in System ({x},{y})"),
            FleetStandingOrderKind::BombardWorld => format!("Bombard world in System ({x},{y})"),
            FleetStandingOrderKind::InvadeWorld => format!("Invade world in System ({x},{y})"),
            FleetStandingOrderKind::BlitzWorld => format!("Blitz world in System ({x},{y})"),
            FleetStandingOrderKind::ViewWorld => format!("View world in System ({x},{y})"),
            FleetStandingOrderKind::ScoutSector => format!("Scout Sector ({x},{y})"),
            FleetStandingOrderKind::ScoutSolarSystem => format!("Scout solar system ({x},{y})"),
            FleetStandingOrderKind::ColonizeWorld => format!("Colonize world in System ({x},{y})"),
            FleetStandingOrderKind::JoinAnotherFleet => format!("Join another fleet at raw target ({x},{y})"),
            FleetStandingOrderKind::RendezvousSector => format!("Rendezvous at Sector ({x},{y})"),
            FleetStandingOrderKind::Salvage => format!("Salvage at Sector ({x},{y})"),
            FleetStandingOrderKind::Unknown(code) => format!("Unknown order {code} target ({x},{y})"),
        }
    }

    pub fn scout_count(&self) -> u8 { self.raw[0x24] }
    pub fn rules_of_engagement(&self) -> u8 { self.raw[0x25] }
    pub fn battleship_count(&self) -> u16 { u16::from_le_bytes([self.raw[0x26], self.raw[0x27]]) }
    pub fn cruiser_count(&self) -> u16 { u16::from_le_bytes([self.raw[0x28], self.raw[0x29]]) }
    pub fn destroyer_count(&self) -> u16 { u16::from_le_bytes([self.raw[0x2A], self.raw[0x2B]]) }
    pub fn troop_transport_count(&self) -> u16 { u16::from_le_bytes([self.raw[0x2C], self.raw[0x2D]]) }
    pub fn army_count(&self) -> u16 { u16::from_le_bytes([self.raw[0x2E], self.raw[0x2F]]) }
    pub fn etac_count(&self) -> u16 { u16::from_le_bytes([self.raw[0x30], self.raw[0x31]]) }

    pub fn ship_composition_summary(&self) -> String {
        let parts = [
            ("SC", self.scout_count() as u16),
            ("BB", self.battleship_count()),
            ("CA", self.cruiser_count()),
            ("DD", self.destroyer_count()),
            ("TT", self.troop_transport_count()),
            ("ARMY", self.army_count()),
            ("ET", self.etac_count()),
        ]
        .into_iter()
        .filter_map(|(label, count)| (count > 0).then(|| format!("{label}={count}")))
        .collect::<Vec<_>>();

        if parts.is_empty() { "none".to_string() } else { parts.join(" ") }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetDat {
    pub records: Vec<FleetRecord>,
}

impl FleetDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % FLEET_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "FLEETS.DAT",
                record_size: FLEET_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(FLEET_RECORD_SIZE)
                .map(|chunk| FleetRecord { raw: copy_array(chunk) })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records.iter().flat_map(|record| record.raw).collect::<Vec<_>>()
    }
}
