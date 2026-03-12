use std::fmt;

pub const PLAYER_RECORD_SIZE: usize = 88;
pub const PLAYER_RECORD_COUNT: usize = 5;
pub const PLAYER_DAT_SIZE: usize = PLAYER_RECORD_SIZE * PLAYER_RECORD_COUNT;

pub const PLANET_RECORD_SIZE: usize = 97;
pub const PLANET_RECORD_COUNT: usize = 20;
pub const PLANETS_DAT_SIZE: usize = PLANET_RECORD_SIZE * PLANET_RECORD_COUNT;

pub const FLEET_RECORD_SIZE: usize = 54;
pub const INITIALIZED_FLEET_RECORD_COUNT: usize = 16;
pub const INITIALIZED_FLEETS_DAT_SIZE: usize = FLEET_RECORD_SIZE * INITIALIZED_FLEET_RECORD_COUNT;
pub const BASE_RECORD_SIZE: usize = 35;
pub const IPBM_RECORD_SIZE: usize = 32;

pub const SETUP_DAT_SIZE: usize = 522;
pub const CONQUEST_DAT_SIZE: usize = 2085;
pub const MAINTENANCE_DAY_ENABLED_CODES: [u8; 7] = [0x01, 0x01, 0xCA, 0x01, 0x0A, 0x01, 0x26];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    WrongSize {
        file_type: &'static str,
        expected: usize,
        actual: usize,
    },
    WrongRecordMultiple {
        file_type: &'static str,
        record_size: usize,
        actual: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSize {
                file_type,
                expected,
                actual,
            } => write!(
                f,
                "{file_type} had wrong size: expected {expected} bytes, got {actual}"
            ),
            Self::WrongRecordMultiple {
                file_type,
                record_size,
                actual,
            } => write!(
                f,
                "{file_type} had wrong size: expected a multiple of {record_size} bytes, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

fn expect_size(data: &[u8], expected: usize, file_type: &'static str) -> Result<(), ParseError> {
    if data.len() == expected {
        Ok(())
    } else {
        Err(ParseError::WrongSize {
            file_type,
            expected,
            actual: data.len(),
        })
    }
}

fn copy_array<const N: usize>(data: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(data);
    out
}

fn trim_ascii_field(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.trim_matches(char::from(0)).trim().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRecord {
    pub raw: [u8; PLAYER_RECORD_SIZE],
}

impl PlayerRecord {
    pub fn occupied_flag(&self) -> u8 {
        self.raw[0]
    }

    pub fn owner_mode_raw(&self) -> u8 {
        self.raw[0]
    }

    pub fn handle_bytes(&self) -> &[u8] {
        &self.raw[1..=0x1A]
    }

    pub fn empire_name_bytes(&self) -> &[u8] {
        &self.raw[0x1C..=0x2E]
    }

    pub fn assigned_player_flag_raw(&self) -> u8 {
        self.raw[22]
    }

    pub fn legacy_status_name_max_len_raw(&self) -> u8 {
        self.raw[26]
    }

    pub fn legacy_status_name_len_raw(&self) -> u8 {
        self.raw[27]
    }

    pub fn legacy_status_name_summary(&self) -> String {
        let len = self.legacy_status_name_len_raw() as usize;
        let end = (28 + len).min(self.raw.len());
        trim_ascii_field(&self.raw[28..end])
    }

    pub fn assigned_player_handle_summary(&self) -> String {
        trim_ascii_field(&self.raw[23..48])
    }

    pub fn controlled_empire_name_len_raw(&self) -> u8 {
        self.raw[49]
    }

    pub fn controlled_empire_name_summary(&self) -> String {
        let len = self.controlled_empire_name_len_raw() as usize;
        let end = (50 + len).min(self.raw.len());
        trim_ascii_field(&self.raw[50..end])
    }

    pub fn ownership_summary(&self) -> String {
        let legacy = self.legacy_status_name_summary();
        let handle = self.assigned_player_handle_summary();
        let empire = self.controlled_empire_name_summary();

        if self.owner_mode_raw() == 0xff {
            format!("rogue label='{legacy}'")
        } else if legacy.starts_with("In Civil Disorder") || legacy == "Unowned" {
            format!("unowned label='{legacy}'")
        } else if self.assigned_player_flag_raw() != 0 || !handle.is_empty() || !empire.is_empty() {
            format!(
                "player handle='{}' empire='{}'",
                handle,
                empire
            )
        } else {
            format!("unowned label='{legacy}'")
        }
    }

    pub fn tax_rate(&self) -> u8 {
        self.raw[0x51]
    }

    pub fn starbase_count_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x44], self.raw[0x45]])
    }

    pub fn set_starbase_count_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x44] = lo;
        self.raw[0x45] = hi;
    }

    pub fn fleet_chain_head_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x40], self.raw[0x41]])
    }

    pub fn ipbm_count_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x48], self.raw[0x49]])
    }

    pub fn set_ipbm_count_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x48] = lo;
        self.raw[0x49] = hi;
    }

    pub fn last_run_year(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x4E], self.raw[0x4F]])
    }

    pub fn treasury(&self) -> u32 {
        u32::from_le_bytes([self.raw[0x52], self.raw[0x53], self.raw[0x54], self.raw[0x55]])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDat {
    pub records: [PlayerRecord; PLAYER_RECORD_COUNT],
}

impl PlayerDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, PLAYER_DAT_SIZE, "PLAYER.DAT")?;
        Ok(Self {
            records: std::array::from_fn(|idx| {
                let start = idx * PLAYER_RECORD_SIZE;
                let end = start + PLAYER_RECORD_SIZE;
                PlayerRecord {
                    raw: copy_array(&data[start..end]),
                }
            }),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetRecord {
    pub raw: [u8; PLANET_RECORD_SIZE],
}

impl PlanetRecord {
    pub fn header_bytes(&self) -> &[u8] {
        &self.raw[..3]
    }

    pub fn coords_raw(&self) -> [u8; 2] {
        [self.raw[0], self.raw[1]]
    }

    pub fn header_value_raw(&self) -> u8 {
        self.raw[2]
    }

    pub fn string_len(&self) -> u8 {
        self.raw[0x0F]
    }

    pub fn status_or_name_bytes(&self) -> &[u8] {
        &self.raw[0x10..=0x1C]
    }

    pub fn potential_production_raw(&self) -> [u8; 2] {
        [self.raw[0x02], self.raw[0x03]]
    }

    pub fn factories_raw(&self) -> [u8; 6] {
        copy_array(&self.raw[0x04..0x0A])
    }

    pub fn stored_goods_raw(&self) -> u32 {
        u32::from_le_bytes(copy_array(&self.raw[0x0A..0x0E]))
    }

    pub fn planet_tax_rate_raw(&self) -> u8 {
        self.raw[0x0E]
    }

    pub fn build_count_raw(&self, slot: usize) -> u8 {
        self.raw[0x24 + slot]
    }

    pub fn build_kind_raw(&self, slot: usize) -> u8 {
        self.raw[0x2E + slot]
    }

    pub fn set_build_count_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x24 + slot] = value;
    }

    pub fn set_build_kind_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x2E + slot] = value;
    }

    pub fn stardock_count_raw(&self, slot: usize) -> u16 {
        u16::from_le_bytes([self.raw[0x38 + slot * 2], self.raw[0x38 + slot * 2 + 1]])
    }

    pub fn stardock_kind_raw(&self, slot: usize) -> u8 {
        self.raw[0x4C + slot]
    }

    pub fn set_stardock_count_raw(&mut self, slot: usize, value: u16) {
        self.raw[0x38 + slot * 2..0x38 + slot * 2 + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn set_stardock_kind_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x4C + slot] = value;
    }

    pub fn population_raw(&self) -> [u8; 6] {
        copy_array(&self.raw[0x52..0x58])
    }

    pub fn owner_empire_slot_raw(&self) -> u8 {
        self.raw[0x5D]
    }

    pub fn army_count_raw(&self) -> u8 {
        self.raw[0x58]
    }

    pub fn likely_army_count_raw(&self) -> u8 {
        self.raw[0x5A]
    }

    pub fn ground_batteries_raw(&self) -> u8 {
        self.raw[0x5A]
    }

    pub fn developed_value_raw(&self) -> u8 {
        self.raw[0x58] // Alias to avoid breaking tests temporarily
    }

    pub fn ownership_status_raw(&self) -> u8 {
        self.raw[0x5C]
    }

    pub fn status_or_name_summary(&self) -> String {
        let len = self.string_len() as usize;
        let text = &self.status_or_name_bytes()[..len.min(self.status_or_name_bytes().len())];
        String::from_utf8_lossy(text)
            .trim_matches(char::from(0))
            .trim()
            .to_string()
    }

    pub fn is_named_homeworld_seed(&self) -> bool {
        self.status_or_name_summary() == "Not Named Yet"
    }

    pub fn derived_summary(&self) -> String {
        let [x, y] = self.coords_raw();
        let text = self.status_or_name_summary();
        let mut parts = vec![format!("({},{}): {}", x, y, text)];
        if self.is_named_homeworld_seed() {
            parts.push("likely_homeworld_seed".to_string());
        }
        if self.build_count_raw(0) != 0 || self.build_kind_raw(0) != 0 {
            parts.push(format!(
                "build_raw={:02x}/{:02x}",
                self.build_count_raw(0),
                self.build_kind_raw(0)
            ));
        }
        if self.owner_empire_slot_raw() != 0 {
            parts.push(format!(
                "owner_slot={} owner_status={:02x}",
                self.owner_empire_slot_raw(),
                self.ownership_status_raw()
            ));
        }
        if self.likely_army_count_raw() != 0 {
            parts.push(format!("likely_armies={}", self.likely_army_count_raw()));
        }
        if self.developed_value_raw() != 0 {
            parts.push(format!("dev58={}", self.developed_value_raw()));
        }
        parts.join(" | ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDat {
    pub records: [PlanetRecord; PLANET_RECORD_COUNT],
}

impl PlanetDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, PLANETS_DAT_SIZE, "PLANETS.DAT")?;
        Ok(Self {
            records: std::array::from_fn(|idx| {
                let start = idx * PLANET_RECORD_SIZE;
                let end = start + PLANET_RECORD_SIZE;
                PlanetRecord {
                    raw: copy_array(&data[start..end]),
                }
            }),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

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

    pub fn local_slot(&self) -> u8 {
        self.raw[0x00]
    }

    pub fn next_fleet_link_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x03], self.raw[0x04]])
    }

    pub fn next_fleet_id(&self) -> u8 {
        self.raw[0x03]
    }

    pub fn fleet_id_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }

    pub fn fleet_id(&self) -> u8 {
        self.raw[0x05]
    }

    pub fn previous_fleet_id(&self) -> u8 {
        self.raw[0x07]
    }

    pub fn max_speed(&self) -> u8 {
        self.raw[0x09]
    }

    pub fn current_speed(&self) -> u8 {
        self.raw[0x0A]
    }

    pub fn set_current_speed(&mut self, value: u8) {
        self.raw[0x0A] = value;
    }

    pub fn current_location_coords_raw(&self) -> [u8; 2] {
        [self.raw[0x0B], self.raw[0x0C]]
    }

    pub fn mission_param_bytes(&self) -> &[u8] {
        &self.raw[0x1F..=0x21]
    }

    pub fn standing_order_code_raw(&self) -> u8 {
        self.raw[0x1F]
    }

    pub fn set_standing_order_code_raw(&mut self, value: u8) {
        self.raw[0x1F] = value;
    }

    pub fn standing_order_kind(&self) -> FleetStandingOrderKind {
        FleetStandingOrderKind::from_raw(self.standing_order_code_raw())
    }

    pub fn standing_order_target_coords_raw(&self) -> [u8; 2] {
        [self.raw[0x20], self.raw[0x21]]
    }

    pub fn set_standing_order_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x20] = coords[0];
        self.raw[0x21] = coords[1];
    }

    pub fn mission_aux_bytes(&self) -> [u8; 2] {
        [self.raw[0x22], self.raw[0x23]]
    }

    pub fn guard_starbase_index_raw(&self) -> u8 {
        self.raw[0x22]
    }

    pub fn guard_starbase_enable_raw(&self) -> u8 {
        self.raw[0x23]
    }

    pub fn set_mission_aux_bytes(&mut self, value: [u8; 2]) {
        self.raw[0x22] = value[0];
        self.raw[0x23] = value[1];
    }

    pub fn tuple_a_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x0D..0x12])
    }

    pub fn set_tuple_a_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x0D..0x12].copy_from_slice(&payload);
    }

    pub fn tuple_b_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x13..0x18])
    }

    pub fn set_tuple_b_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x13..0x18].copy_from_slice(&payload);
    }

    pub fn tuple_c_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x19..0x1E])
    }

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
            FleetStandingOrderKind::GuardStarbase => {
                format!("Guard starbase at Sector ({x},{y})")
            }
            FleetStandingOrderKind::GuardBlockadeWorld => {
                format!("Guard/blockade world in System ({x},{y})")
            }
            FleetStandingOrderKind::BombardWorld => {
                format!("Bombard world in System ({x},{y})")
            }
            FleetStandingOrderKind::InvadeWorld => {
                format!("Invade world in System ({x},{y})")
            }
            FleetStandingOrderKind::BlitzWorld => {
                format!("Blitz world in System ({x},{y})")
            }
            FleetStandingOrderKind::ViewWorld => format!("View world in System ({x},{y})"),
            FleetStandingOrderKind::ScoutSector => format!("Scout Sector ({x},{y})"),
            FleetStandingOrderKind::ScoutSolarSystem => format!("Scout solar system ({x},{y})"),
            FleetStandingOrderKind::ColonizeWorld => {
                format!("Colonize world in System ({x},{y})")
            }
            FleetStandingOrderKind::JoinAnotherFleet => {
                format!("Join another fleet at raw target ({x},{y})")
            }
            FleetStandingOrderKind::RendezvousSector => {
                format!("Rendezvous at Sector ({x},{y})")
            }
            FleetStandingOrderKind::Salvage => {
                format!("Salvage at Sector ({x},{y})")
            }
            FleetStandingOrderKind::Unknown(code) => {
                format!("Unknown order {code} target ({x},{y})")
            }
        }
    }

    pub fn scout_count(&self) -> u8 {
        self.raw[0x24]
    }

    pub fn rules_of_engagement(&self) -> u8 {
        self.raw[0x25]
    }

    pub fn battleship_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x26], self.raw[0x27]])
    }

    pub fn cruiser_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x28], self.raw[0x29]])
    }

    pub fn destroyer_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x2A], self.raw[0x2B]])
    }

    pub fn troop_transport_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x2C], self.raw[0x2D]])
    }

    pub fn army_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x2E], self.raw[0x2F]])
    }

    pub fn etac_count(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x30], self.raw[0x31]])
    }

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

        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(" ")
        }
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
                .map(|chunk| FleetRecord {
                    raw: copy_array(chunk),
                })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseRecord {
    pub raw: [u8; BASE_RECORD_SIZE],
}

impl BaseRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; BASE_RECORD_SIZE],
        }
    }

    pub fn local_slot_raw(&self) -> u8 {
        self.raw[0x00]
    }

    pub fn set_local_slot_raw(&mut self, value: u8) {
        self.raw[0x00] = value;
    }

    pub fn active_flag_raw(&self) -> u8 {
        self.raw[0x02]
    }

    pub fn summary_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x02], self.raw[0x03]])
    }

    pub fn set_summary_word_raw(&mut self, value: u16) {
        self.raw[0x02..0x04].copy_from_slice(&value.to_le_bytes());
    }

    pub fn set_active_flag_raw(&mut self, value: u8) {
        self.raw[0x02] = value;
    }

    pub fn base_id_raw(&self) -> u8 {
        self.raw[0x04]
    }

    pub fn set_base_id_raw(&mut self, value: u8) {
        self.raw[0x04] = value;
    }

    pub fn link_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }

    pub fn set_link_word_raw(&mut self, value: u16) {
        self.raw[0x05..0x07].copy_from_slice(&value.to_le_bytes());
    }

    pub fn chain_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x07], self.raw[0x08]])
    }

    pub fn set_chain_word_raw(&mut self, value: u16) {
        self.raw[0x07..0x09].copy_from_slice(&value.to_le_bytes());
    }

    pub fn coords_raw(&self) -> [u8; 2] {
        [self.raw[0x0B], self.raw[0x0C]]
    }

    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x0B..0x0D].copy_from_slice(&coords);
    }

    pub fn tuple_a_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x0D..0x12])
    }

    pub fn set_tuple_a_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x0D..0x12].copy_from_slice(&payload);
    }

    pub fn tuple_b_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x13..0x18])
    }

    pub fn set_tuple_b_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x13..0x18].copy_from_slice(&payload);
    }

    pub fn tuple_c_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x19..0x1E])
    }

    pub fn set_tuple_c_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x19..0x1E].copy_from_slice(&payload);
    }

    pub fn trailing_coords_raw(&self) -> [u8; 2] {
        copy_array(&self.raw[0x20..0x22])
    }

    pub fn set_trailing_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x20..0x22].copy_from_slice(&coords);
    }

    pub fn owner_empire_raw(&self) -> u8 {
        self.raw[0x22]
    }

    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.raw[0x22] = value;
    }

    pub fn from_raw(raw: [u8; BASE_RECORD_SIZE]) -> Self {
        Self { raw }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseDat {
    pub records: Vec<BaseRecord>,
}

impl BaseDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % BASE_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "BASES.DAT",
                record_size: BASE_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(BASE_RECORD_SIZE)
                .map(|chunk| BaseRecord {
                    raw: copy_array(chunk),
                })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpbmRecord {
    pub raw: [u8; IPBM_RECORD_SIZE],
}

impl IpbmRecord {
    pub fn primary_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x00], self.raw[0x01]])
    }

    pub fn set_primary_word_raw(&mut self, value: u16) {
        self.raw[0x00..0x02].copy_from_slice(&value.to_le_bytes());
    }

    pub fn owner_empire_raw(&self) -> u8 {
        self.raw[0x02]
    }

    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.raw[0x02] = value;
    }

    pub fn gate_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x03], self.raw[0x04]])
    }

    pub fn set_gate_word_raw(&mut self, value: u16) {
        self.raw[0x03..0x05].copy_from_slice(&value.to_le_bytes());
    }

    pub fn follow_on_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }

    pub fn set_follow_on_word_raw(&mut self, value: u16) {
        self.raw[0x05..0x07].copy_from_slice(&value.to_le_bytes());
    }

    pub fn tuple_a_tag_raw(&self) -> u8 {
        self.raw[0x09]
    }

    pub fn set_tuple_a_tag_raw(&mut self, value: u8) {
        self.raw[0x09] = value;
    }

    pub fn tuple_b_tag_raw(&self) -> u8 {
        self.raw[0x0A]
    }

    pub fn set_tuple_b_tag_raw(&mut self, value: u8) {
        self.raw[0x0A] = value;
    }

    pub fn tuple_a_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x0B..0x10])
    }

    pub fn set_tuple_a_payload_raw(&mut self, value: [u8; 5]) {
        self.raw[0x0B..0x10].copy_from_slice(&value);
    }

    pub fn tuple_b_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x11..0x16])
    }

    pub fn set_tuple_b_payload_raw(&mut self, value: [u8; 5]) {
        self.raw[0x11..0x16].copy_from_slice(&value);
    }

    pub fn tuple_c_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x17..0x1C])
    }

    pub fn set_tuple_c_payload_raw(&mut self, value: [u8; 5]) {
        self.raw[0x17..0x1C].copy_from_slice(&value);
    }

    pub fn trailing_control_raw(&self) -> [u8; 3] {
        copy_array(&self.raw[0x1D..0x20])
    }

    pub fn set_trailing_control_raw(&mut self, value: [u8; 3]) {
        self.raw[0x1D..0x20].copy_from_slice(&value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpbmDat {
    pub records: Vec<IpbmRecord>,
}

impl IpbmDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % IPBM_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "IPBM.DAT",
                record_size: IPBM_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(IPBM_RECORD_SIZE)
                .map(|chunk| IpbmRecord {
                    raw: copy_array(chunk),
                })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupDat {
    pub raw: [u8; SETUP_DAT_SIZE],
}

impl SetupDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, SETUP_DAT_SIZE, "SETUP.DAT")?;
        Ok(Self {
            raw: copy_array(data),
        })
    }

    pub fn version_tag(&self) -> &[u8] {
        &self.raw[..5]
    }

    pub fn option_prefix(&self) -> &[u8] {
        &self.raw[5..13]
    }

    pub fn com_irq_raw(&self, com_index: usize) -> Option<u8> {
        (com_index < 4).then(|| self.raw[5 + com_index])
    }

    pub fn set_com_irq_raw(&mut self, com_index: usize, irq: u8) -> bool {
        if com_index < 4 {
            self.raw[5 + com_index] = irq;
            true
        } else {
            false
        }
    }

    pub fn com_hardware_flow_control_enabled(&self, com_index: usize) -> Option<bool> {
        (com_index < 4).then(|| self.raw[9 + com_index] != 0)
    }

    pub fn set_com_hardware_flow_control_enabled(
        &mut self,
        com_index: usize,
        enabled: bool,
    ) -> bool {
        if com_index < 4 {
            self.raw[9 + com_index] = u8::from(enabled);
            true
        } else {
            false
        }
    }

    pub fn snoop_enabled(&self) -> bool {
        self.raw[512] != 0
    }

    pub fn set_snoop_enabled(&mut self, enabled: bool) {
        self.raw[512] = u8::from(enabled);
    }

    pub fn max_time_between_keys_minutes_raw(&self) -> u8 {
        self.raw[513]
    }

    pub fn set_max_time_between_keys_minutes_raw(&mut self, minutes: u8) {
        self.raw[513] = minutes;
    }

    pub fn remote_timeout_enabled(&self) -> bool {
        self.raw[515] != 0
    }

    pub fn set_remote_timeout_enabled(&mut self, enabled: bool) {
        self.raw[515] = u8::from(enabled);
    }

    pub fn local_timeout_enabled(&self) -> bool {
        self.raw[516] != 0
    }

    pub fn set_local_timeout_enabled(&mut self, enabled: bool) {
        self.raw[516] = u8::from(enabled);
    }

    pub fn minimum_time_granted_minutes_raw(&self) -> u8 {
        self.raw[517]
    }

    pub fn set_minimum_time_granted_minutes_raw(&mut self, minutes: u8) {
        self.raw[517] = minutes;
    }

    pub fn purge_after_turns_raw(&self) -> u8 {
        self.raw[518]
    }

    pub fn set_purge_after_turns_raw(&mut self, turns: u8) {
        self.raw[518] = turns;
    }

    pub fn autopilot_inactive_turns_raw(&self) -> u8 {
        self.raw[520]
    }

    pub fn set_autopilot_inactive_turns_raw(&mut self, turns: u8) {
        self.raw[520] = turns;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConquestDat {
    pub raw: [u8; CONQUEST_DAT_SIZE],
}

impl ConquestDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, CONQUEST_DAT_SIZE, "CONQUEST.DAT")?;
        Ok(Self {
            raw: copy_array(data),
        })
    }

    pub fn control_header(&self) -> &[u8] {
        &self.raw[..0x55]
    }

    pub fn game_year(&self) -> u16 {
        u16::from_le_bytes([self.raw[0], self.raw[1]])
    }

    pub fn player_count(&self) -> u8 {
        self.raw[2]
    }

    pub fn player_config_word(&self) -> u16 {
        u16::from_le_bytes([self.raw[2], self.raw[3]])
    }

    pub fn maintenance_schedule_bytes(&self) -> [u8; 7] {
        self.raw[3..10]
            .try_into()
            .expect("maintenance schedule should be 7 bytes")
    }

    pub fn maintenance_schedule_enabled(&self) -> [bool; 7] {
        self.maintenance_schedule_bytes().map(|byte| byte != 0)
    }

    pub fn set_maintenance_schedule_enabled(&mut self, enabled: [bool; 7]) {
        for (idx, enabled) in enabled.into_iter().enumerate() {
            self.raw[3 + idx] = if enabled {
                MAINTENANCE_DAY_ENABLED_CODES[idx]
            } else {
                0
            };
        }
    }

    pub fn header_words(&self) -> Vec<u16> {
        self.control_header()
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }
}

