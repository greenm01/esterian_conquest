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

    pub fn build_slot_raw(&self) -> u8 {
        self.raw[0x24]
    }

    pub fn build_kind_raw(&self) -> u8 {
        self.raw[0x2E]
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
        self.raw[0x58] // Alias to avoid breaking tests temporarily
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
        if self.build_slot_raw() != 0 || self.build_kind_raw() != 0 {
            parts.push(format!(
                "build_raw={:02x}/{:02x}",
                self.build_slot_raw(),
                self.build_kind_raw()
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
    pub fn local_slot(&self) -> u8 {
        self.raw[0x00]
    }

    pub fn next_fleet_id(&self) -> u8 {
        self.raw[0x03]
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

    pub fn current_location_coords_raw(&self) -> [u8; 2] {
        [self.raw[0x0B], self.raw[0x0C]]
    }

    pub fn mission_param_bytes(&self) -> &[u8] {
        &self.raw[0x1F..=0x21]
    }

    pub fn standing_order_code_raw(&self) -> u8 {
        self.raw[0x1F]
    }

    pub fn standing_order_kind(&self) -> FleetStandingOrderKind {
        FleetStandingOrderKind::from_raw(self.standing_order_code_raw())
    }

    pub fn standing_order_target_coords_raw(&self) -> [u8; 2] {
        [self.raw[0x20], self.raw[0x21]]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../original/v1.5")
            .join(name)
    }

    fn initialized_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecutil-init/v1.5")
            .join(name)
    }

    fn post_maint_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-post/v1.5")
            .join(name)
    }

    fn f3_owner_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecutil-f3-owner/v1.5")
            .join(name)
    }

    fn ecmaint_build_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-build-pre/v1.5")
            .join(name)
    }

    fn ecmaint_build_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-build-post/v1.5")
            .join(name)
    }

    fn ecmaint_fleet_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-fleet-pre/v1.5")
            .join(name)
    }

    fn ecmaint_fleet_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-fleet-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_arrive_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-arrive/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_dev0_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-dev0-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army0_dev0_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army0-dev0-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_e0c_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-e0c-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_e0c_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-e0c-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b08_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b08-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b08_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b08-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b09_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b09-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_army1_dev0_b09_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-army1-dev0-b09-post/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_heavy_pre_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-heavy-pre/v1.5")
            .join(name)
    }

    fn ecmaint_bombard_heavy_post_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ecmaint-bombard-heavy-post/v1.5")
            .join(name)
    }

    fn read_fixture(name: &str) -> Vec<u8> {
        fs::read(fixture_path(name)).expect("fixture should exist")
    }

    fn read_initialized_fixture(name: &str) -> Vec<u8> {
        fs::read(initialized_fixture_path(name)).expect("initialized fixture should exist")
    }

    fn read_post_maint_fixture(name: &str) -> Vec<u8> {
        fs::read(post_maint_fixture_path(name)).expect("post-maint fixture should exist")
    }

    fn read_f3_owner_fixture(name: &str) -> Vec<u8> {
        fs::read(f3_owner_fixture_path(name)).expect("f3 owner fixture should exist")
    }

    fn read_ecmaint_build_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_build_pre_fixture_path(name)).expect("ecmaint build-pre fixture should exist")
    }

    fn read_ecmaint_build_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_build_post_fixture_path(name)).expect("ecmaint build-post fixture should exist")
    }

    fn read_ecmaint_fleet_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_fleet_pre_fixture_path(name)).expect("ecmaint fleet-pre fixture should exist")
    }

    fn read_ecmaint_fleet_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_fleet_post_fixture_path(name)).expect("ecmaint fleet-post fixture should exist")
    }

    fn read_ecmaint_bombard_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_pre_fixture_path(name)).expect("ecmaint bombard-pre fixture should exist")
    }

    fn read_ecmaint_bombard_arrive_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_arrive_fixture_path(name)).expect("ecmaint bombard-arrive fixture should exist")
    }

    fn read_ecmaint_bombard_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_post_fixture_path(name)).expect("ecmaint bombard-post fixture should exist")
    }

    fn read_ecmaint_bombard_army0_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_pre_fixture_path(name))
            .expect("ecmaint bombard-army0-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army0_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_post_fixture_path(name))
            .expect("ecmaint bombard-army0-post fixture should exist")
    }

    fn read_ecmaint_bombard_army0_dev0_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_dev0_pre_fixture_path(name))
            .expect("ecmaint bombard-army0-dev0-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army0_dev0_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army0_dev0_post_fixture_path(name))
            .expect("ecmaint bombard-army0-dev0-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_post_fixture_path(name))
            .expect("ecmaint bombard-army1-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_e0c_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_e0c_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-e0c-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_e0c_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_e0c_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-e0c-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b08_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b08_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b08-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b08_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b08_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b08-post fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b09_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b09_pre_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b09-pre fixture should exist")
    }

    fn read_ecmaint_bombard_army1_dev0_b09_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_army1_dev0_b09_post_fixture_path(name))
            .expect("ecmaint bombard-army1-dev0-b09-post fixture should exist")
    }

    fn read_ecmaint_bombard_heavy_pre_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_heavy_pre_fixture_path(name))
            .expect("ecmaint bombard-heavy-pre fixture should exist")
    }

    fn read_ecmaint_bombard_heavy_post_fixture(name: &str) -> Vec<u8> {
        fs::read(ecmaint_bombard_heavy_post_fixture_path(name))
            .expect("ecmaint bombard-heavy-post fixture should exist")
    }

    #[test]
    fn round_trip_player_dat() {
        let bytes = read_fixture("PLAYER.DAT");
        let parsed = PlayerDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
    }

    #[test]
    fn round_trip_planets_dat() {
        let bytes = read_fixture("PLANETS.DAT");
        let parsed = PlanetDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
    }

    #[test]
    fn initialized_planets_expose_named_homeworld_seeds() {
        let bytes = read_initialized_fixture("PLANETS.DAT");
        let parsed = PlanetDat::parse(&bytes).unwrap();
        let seeds = parsed
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| record.is_named_homeworld_seed())
            .map(|(idx, record)| (idx + 1, record.coords_raw(), record.header_value_raw()))
            .collect::<Vec<_>>();

        assert_eq!(
            seeds,
            vec![
                (5, [6, 5], 100),
                (6, [13, 5], 100),
                (13, [4, 13], 100),
                (15, [16, 13], 100),
            ]
        );
    }

    #[test]
    fn planet_tail_fields_expose_owner_slot_and_likely_armies() {
        let init = PlanetDat::parse(&read_initialized_fixture("PLANETS.DAT")).unwrap();
        assert_eq!(init.records[12].owner_empire_slot_raw(), 2);
        assert_eq!(init.records[12].ownership_status_raw(), 2);
        assert_eq!(init.records[12].likely_army_count_raw(), 4);

        assert_eq!(init.records[14].owner_empire_slot_raw(), 1);
        assert_eq!(init.records[14].ownership_status_raw(), 2);
        assert_eq!(init.records[14].likely_army_count_raw(), 4);

        let fleet_post = PlanetDat::parse(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();
        assert_eq!(fleet_post.records[13].owner_empire_slot_raw(), 1);
        assert_eq!(fleet_post.records[13].ownership_status_raw(), 2);
        assert_eq!(fleet_post.records[13].likely_army_count_raw(), 0);
    }

    #[test]
    fn round_trip_setup_dat() {
        let bytes = read_fixture("SETUP.DAT");
        let parsed = SetupDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
        assert_eq!(parsed.version_tag(), b"EC151");
        assert_eq!(parsed.option_prefix(), &[4, 3, 4, 3, 1, 1, 1, 1]);
        assert_eq!(parsed.com_irq_raw(0), Some(4));
        assert_eq!(parsed.com_irq_raw(1), Some(3));
        assert_eq!(parsed.com_irq_raw(2), Some(4));
        assert_eq!(parsed.com_irq_raw(3), Some(3));
        assert_eq!(parsed.com_hardware_flow_control_enabled(0), Some(true));
        assert_eq!(parsed.com_hardware_flow_control_enabled(1), Some(true));
        assert_eq!(parsed.com_hardware_flow_control_enabled(2), Some(true));
        assert_eq!(parsed.com_hardware_flow_control_enabled(3), Some(true));
        assert!(parsed.snoop_enabled());
        assert_eq!(parsed.max_time_between_keys_minutes_raw(), 10);
        assert!(parsed.remote_timeout_enabled());
        assert!(!parsed.local_timeout_enabled());
        assert_eq!(parsed.minimum_time_granted_minutes_raw(), 0);
        assert_eq!(parsed.purge_after_turns_raw(), 0);
        assert_eq!(parsed.autopilot_inactive_turns_raw(), 0);
    }

    #[test]
    fn round_trip_conquest_dat() {
        let bytes = read_fixture("CONQUEST.DAT");
        let parsed = ConquestDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
        assert_eq!(parsed.control_header().len(), 0x55);
        assert_eq!(parsed.header_words()[0], 0x0bce);
        assert_eq!(parsed.game_year(), 3022);
        assert_eq!(parsed.player_count(), 4);
        assert_eq!(parsed.player_config_word(), 0x0104);
    }

    #[test]
    fn player_tax_rate_matches_current_notes() {
        let bytes = read_fixture("PLAYER.DAT");
        let parsed = PlayerDat::parse(&bytes).unwrap();
        assert_eq!(parsed.records[0].tax_rate(), 65);
    }

    #[test]
    fn f3_owner_fixture_exposes_rogue_and_player_controlled_empire_summaries() {
        let bytes = read_f3_owner_fixture("PLAYER.DAT");
        let parsed = PlayerDat::parse(&bytes).unwrap();

        assert_eq!(parsed.records[0].owner_mode_raw(), 0xff);
        assert_eq!(parsed.records[0].legacy_status_name_len_raw(), 6);
        assert_eq!(parsed.records[0].legacy_status_name_summary(), "Rogues");
        assert_eq!(parsed.records[0].ownership_summary(), "rogue label='Rogues'");

        assert_eq!(parsed.records[1].assigned_player_flag_raw(), 1);
        assert_eq!(parsed.records[1].assigned_player_handle_summary(), "FOO");
        assert_eq!(parsed.records[1].controlled_empire_name_len_raw(), 3);
        assert_eq!(parsed.records[1].controlled_empire_name_summary(), "foo");
        assert_eq!(
            parsed.records[1].ownership_summary(),
            "player handle='FOO' empire='foo'"
        );
    }

    #[test]
    fn shipped_fleets_dat_uses_a_variable_record_count() {
        let bytes = read_fixture("FLEETS.DAT");
        let parsed = FleetDat::parse(&bytes).unwrap();
        assert_eq!(parsed.records.len(), 13);
    }

    #[test]
    fn round_trip_initialized_fleets_dat() {
        let bytes = read_initialized_fixture("FLEETS.DAT");
        let parsed = FleetDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
        assert_eq!(parsed.records.len(), INITIALIZED_FLEET_RECORD_COUNT);
        assert_eq!(parsed.records[0].fleet_id(), 1);
        assert_eq!(parsed.records[0].local_slot(), 1);
        assert_eq!(parsed.records[0].next_fleet_id(), 2);
        assert_eq!(parsed.records[0].previous_fleet_id(), 0);
        assert_eq!(parsed.records[0].max_speed(), 3);
        assert_eq!(parsed.records[0].rules_of_engagement(), 6);
        assert_eq!(parsed.records[0].cruiser_count(), 1);
        assert_eq!(parsed.records[0].destroyer_count(), 0);
        assert_eq!(parsed.records[0].etac_count(), 1);
        assert_eq!(parsed.records[0].standing_order_code_raw(), 5);
        assert_eq!(
            parsed.records[0].standing_order_kind(),
            FleetStandingOrderKind::GuardBlockadeWorld
        );
        assert_eq!(parsed.records[0].standing_order_target_coords_raw(), [16, 13]);
        assert_eq!(
            parsed.records[0].standing_order_summary(),
            "Guard/blockade world in System (16,13)"
        );
        assert_eq!(parsed.records[0].ship_composition_summary(), "CA=1 ET=1");

        assert_eq!(parsed.records[2].fleet_id(), 3);
        assert_eq!(parsed.records[2].local_slot(), 3);
        assert_eq!(parsed.records[2].next_fleet_id(), 4);
        assert_eq!(parsed.records[2].previous_fleet_id(), 2);
        assert_eq!(parsed.records[2].max_speed(), 6);
        assert_eq!(parsed.records[2].rules_of_engagement(), 6);
        assert_eq!(parsed.records[2].cruiser_count(), 0);
        assert_eq!(parsed.records[2].destroyer_count(), 1);
        assert_eq!(parsed.records[2].etac_count(), 0);
        assert_eq!(parsed.records[2].standing_order_code_raw(), 5);
        assert_eq!(
            parsed.records[2].standing_order_kind(),
            FleetStandingOrderKind::GuardBlockadeWorld
        );
        assert_eq!(parsed.records[2].standing_order_target_coords_raw(), [16, 13]);
        assert_eq!(
            parsed.records[2].standing_order_summary(),
            "Guard/blockade world in System (16,13)"
        );
        assert_eq!(parsed.records[2].ship_composition_summary(), "DD=1");
    }

    #[test]
    fn post_maintenance_matches_init_for_core_state_but_not_global_summaries() {
        assert_eq!(
            read_initialized_fixture("PLAYER.DAT"),
            read_post_maint_fixture("PLAYER.DAT")
        );
        assert_eq!(
            read_initialized_fixture("PLANETS.DAT"),
            read_post_maint_fixture("PLANETS.DAT")
        );
        assert_eq!(
            read_initialized_fixture("FLEETS.DAT"),
            read_post_maint_fixture("FLEETS.DAT")
        );
        assert_eq!(
            read_initialized_fixture("SETUP.DAT"),
            read_post_maint_fixture("SETUP.DAT")
        );

        assert_ne!(
            read_initialized_fixture("CONQUEST.DAT"),
            read_post_maint_fixture("CONQUEST.DAT")
        );
        assert_ne!(
            read_initialized_fixture("DATABASE.DAT"),
            read_post_maint_fixture("DATABASE.DAT")
        );
    }

    #[test]
    fn preserved_conquest_year_progression_matches_docs() {
        let original = ConquestDat::parse(&read_fixture("CONQUEST.DAT")).unwrap();
        let initialized = ConquestDat::parse(&read_initialized_fixture("CONQUEST.DAT")).unwrap();
        let post_maint = ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();

        assert_eq!(initialized.game_year(), 3000);
        assert_eq!(post_maint.game_year(), 3001);
        assert_eq!(original.game_year(), 3022);
        assert_eq!(initialized.player_count(), 4);
        assert_eq!(post_maint.player_count(), 4);
        assert_eq!(original.player_count(), 4);
    }

    #[test]
    fn post_maintenance_fixture_exposes_known_schedule_bytes() {
        let post_maint = ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();
        assert_eq!(post_maint.maintenance_schedule_bytes(), [0x01; 7]);
    }

    #[test]
    fn can_set_maintenance_schedule_from_enabled_days() {
        let mut post_maint = ConquestDat::parse(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();
        post_maint.set_maintenance_schedule_enabled([true, false, true, false, true, false, true]);
        assert_eq!(
            post_maint.maintenance_schedule_bytes(),
            [0x01, 0x00, 0xCA, 0x00, 0x0A, 0x00, 0x26]
        );
        assert_eq!(
            post_maint.maintenance_schedule_enabled(),
            [true, false, true, false, true, false, true]
        );
    }

    #[test]
    fn can_toggle_snoop_enabled() {
        let mut setup = SetupDat::parse(&read_fixture("SETUP.DAT")).unwrap();
        assert!(setup.snoop_enabled());
        setup.set_snoop_enabled(false);
        assert!(!setup.snoop_enabled());
        assert_eq!(setup.raw[512], 0);
    }

    #[test]
    fn can_set_other_setup_program_fields() {
        let mut setup = SetupDat::parse(&read_fixture("SETUP.DAT")).unwrap();
        assert!(setup.set_com_hardware_flow_control_enabled(0, false));
        assert!(setup.set_com_hardware_flow_control_enabled(1, false));
        assert!(setup.set_com_hardware_flow_control_enabled(2, false));
        assert!(setup.set_com_hardware_flow_control_enabled(3, false));
        setup.set_max_time_between_keys_minutes_raw(15);
        setup.set_remote_timeout_enabled(false);
        setup.set_local_timeout_enabled(true);
        setup.set_minimum_time_granted_minutes_raw(69);
        setup.set_purge_after_turns_raw(10);
        setup.set_autopilot_inactive_turns_raw(3);

        assert_eq!(setup.max_time_between_keys_minutes_raw(), 15);
        assert!(!setup.remote_timeout_enabled());
        assert!(setup.local_timeout_enabled());
        assert_eq!(setup.minimum_time_granted_minutes_raw(), 69);
        assert_eq!(setup.purge_after_turns_raw(), 10);
        assert_eq!(setup.autopilot_inactive_turns_raw(), 3);
        assert_eq!(setup.com_hardware_flow_control_enabled(0), Some(false));
        assert_eq!(setup.com_hardware_flow_control_enabled(1), Some(false));
        assert_eq!(setup.com_hardware_flow_control_enabled(2), Some(false));
        assert_eq!(setup.com_hardware_flow_control_enabled(3), Some(false));
    }

    #[test]
    fn can_set_purge_after_turns_raw() {
        let mut setup = SetupDat::parse(&read_fixture("SETUP.DAT")).unwrap();
        assert_eq!(setup.purge_after_turns_raw(), 0);
        setup.set_purge_after_turns_raw(1);
        assert_eq!(setup.purge_after_turns_raw(), 1);
        assert_eq!(setup.raw[518], 1);
    }

    #[test]
    fn ecmaint_build_scenario_consumes_queue_and_changes_planet_state() {
        let pre = PlanetDat::parse(&read_ecmaint_build_pre_fixture("PLANETS.DAT")).unwrap();
        let post = PlanetDat::parse(&read_ecmaint_build_post_fixture("PLANETS.DAT")).unwrap();

        let pre_record = &pre.records[14];
        let post_record = &post.records[14];

        assert_eq!(pre_record.raw[0x24], 0x03);
        assert_eq!(pre_record.raw[0x2e], 0x01);

        assert_eq!(post_record.raw[0x24], 0x00);
        assert_eq!(post_record.raw[0x2e], 0x00);
        assert_eq!(post_record.raw[0x38], 0x03);
        assert_eq!(post_record.raw[0x4c], 0x01);
    }

    #[test]
    fn ecmaint_fleet_scenario_consumes_order_and_updates_fleet_and_planet_state() {
        let pre_fleets = FleetDat::parse(&read_ecmaint_fleet_pre_fixture("FLEETS.DAT")).unwrap();
        let post_fleets = FleetDat::parse(&read_ecmaint_fleet_post_fixture("FLEETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();

        let pre_fleet = &pre_fleets.records[0];
        let post_fleet = &post_fleets.records[0];

        assert_eq!(pre_fleet.raw[0x0a], 0x03);
        assert_eq!(pre_fleet.raw[0x1f], 0x0c);
        assert_eq!(pre_fleet.raw[0x20], 0x0f);

        assert_eq!(post_fleet.raw[0x0b], 0x0f);
        assert_eq!(post_fleet.raw[0x19], 0x80);
        assert_eq!(post_fleet.raw[0x1a], 0xb9);
        assert_eq!(post_fleet.raw[0x1b], 0xff);
        assert_eq!(post_fleet.raw[0x1c], 0xff);
        assert_eq!(post_fleet.raw[0x1d], 0xff);
        assert_eq!(post_fleet.raw[0x1e], 0x7f);
        assert_eq!(post_fleet.raw[0x1f], 0x00);
        assert_eq!(post_fleet.raw[0x20], 0x0f);

        let post_planet = &post_planets.records[13];
        assert_eq!(post_planet.raw[0x58], 0x01);
        assert_eq!(post_planet.raw[0x5c], 0x02);
        assert_eq!(post_planet.raw[0x5d], 0x01);
    }

    #[test]
    fn ecmaint_bombard_scenario_arrival_preserves_attack_order() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_pre_fixture("FLEETS.DAT")).unwrap();
        let arrive = FleetDat::parse(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let arrive_fleet = &arrive.records[2];

        assert_eq!(pre_fleet.current_speed(), 0x03);
        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.standing_order_target_coords_raw(), [0x0f, 0x0d]);

        assert_eq!(arrive_fleet.current_speed(), 0x03);
        assert_eq!(arrive_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(arrive_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(arrive_fleet.standing_order_target_coords_raw(), [0x0f, 0x0d]);
    }

    #[test]
    fn ecmaint_bombard_scenario_second_pass_consumes_order_and_kills_attackers() {
        let arrive = FleetDat::parse(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_post_fixture("FLEETS.DAT")).unwrap();

        let arrive_fleet = &arrive.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(arrive_fleet.current_speed(), 0x03);
        assert_eq!(arrive_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(arrive_fleet.cruiser_count(), 0x03);
        assert_eq!(arrive_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x02);
        assert_eq!(post_fleet.destroyer_count(), 0x01);

        let arrive_planets = PlanetDat::parse(&read_ecmaint_bombard_arrive_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_post_fixture("PLANETS.DAT")).unwrap();
        assert_eq!(arrive_planets.records[13].raw, post_planets.records[13].raw);
    }

    #[test]
    fn ecmaint_bombard_zero_army_target_changes_planet_without_attacker_losses() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army0_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army0_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x03);
        assert_eq!(post_fleet.destroyer_count(), 0x05);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_eq!(pre_target.developed_value_raw(), 0x8e);
        assert_eq!(post_target.developed_value_raw(), 0x8a);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army0_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army0_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_zero_army_zero_dev_target_changes_damage_pattern() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army0_dev0_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army0_dev0_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x03);
        assert_eq!(post_fleet.destroyer_count(), 0x05);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_dev0_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army0_dev0_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(pre_target.likely_army_count_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0x00);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army0_dev0_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army0_dev0_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_one_army_target_causes_partial_attacker_losses() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x02);
        assert_eq!(post_fleet.destroyer_count(), 0x02);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_eq!(pre_target.developed_value_raw(), 0x8e);
        assert_eq!(post_target.developed_value_raw(), 0x8d);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_one_army_zero_dev_target_changes_loss_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x02);
        assert_eq!(post_fleet.destroyer_count(), 0x04);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(pre_target.owner_empire_slot_raw(), 2);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_byte_0e_increases_defender_damage_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x03);
        assert_eq!(post_fleet.destroyer_count(), 0x01);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x0e], 0x0c);
        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.raw[0x0e], 0x54);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_e0c_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_e0c_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_byte_08_changes_defender_loss_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        assert_eq!(post_fleet.cruiser_count(), 0x01);
        assert_eq!(post_fleet.destroyer_count(), 0x03);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x08], 0x00);
        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_b08_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_b08_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_byte_09_changes_attacker_loss_profile() {
        let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture("FLEETS.DAT")).unwrap();
        let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_post_fixture("FLEETS.DAT")).unwrap();

        let pre_fleet = &pre.records[2];
        let post_fleet = &post.records[2];

        assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
        assert_eq!(pre_fleet.cruiser_count(), 0x03);
        assert_eq!(pre_fleet.destroyer_count(), 0x05);

        assert_eq!(post_fleet.current_speed(), 0x00);
        assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
        assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
        // army1-dev0 base: CA 3->2 (1 loss), DD 5->4 (1 loss)
        // army1-dev0-b09 (0x09=0): CA 3->1 (2 losses), DD 5->5 (0 losses)
        assert_eq!(post_fleet.cruiser_count(), 0x01);
        assert_eq!(post_fleet.destroyer_count(), 0x05);

        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x09], 0x00);
        assert_eq!(pre_target.likely_army_count_raw(), 1);
        assert_eq!(pre_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.likely_army_count_raw(), 0);
        assert_eq!(post_target.developed_value_raw(), 0x00);
        assert_eq!(post_target.owner_empire_slot_raw(), 2);
        assert_ne!(pre_target.raw, post_target.raw);

        assert_eq!(read_ecmaint_bombard_army1_dev0_b09_post_fixture("MESSAGES.DAT"), Vec::<u8>::new());
        assert_eq!(read_ecmaint_bombard_army1_dev0_b09_post_fixture("RESULTS.DAT"), Vec::<u8>::new());
    }

    #[test]
    fn ecmaint_bombard_heavy_generates_combat_report() {
        let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_heavy_pre_fixture("PLANETS.DAT")).unwrap();
        let post_planets = PlanetDat::parse(&read_ecmaint_bombard_heavy_post_fixture("PLANETS.DAT")).unwrap();
        let pre_target = &pre_planets.records[13];
        let post_target = &post_planets.records[13];

        assert_eq!(pre_target.raw[0x5A], 15); // Ground batteries mapped
        assert_eq!(pre_target.raw[0x58], 0x8E); // Armies mapped

        // Target capacity goes to 0 due to heavy bombardment
        assert_ne!(pre_target.raw, post_target.raw);

        // A report should be generated in RESULTS.DAT for player "FOO" (Empire 2)
        let results = read_ecmaint_bombard_heavy_post_fixture("RESULTS.DAT");
        assert!(!results.is_empty(), "RESULTS.DAT should contain the bombardment report");
    }
}
