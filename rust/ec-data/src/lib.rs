use std::fmt;

pub const PLAYER_RECORD_SIZE: usize = 88;
pub const PLAYER_RECORD_COUNT: usize = 5;
pub const PLAYER_DAT_SIZE: usize = PLAYER_RECORD_SIZE * PLAYER_RECORD_COUNT;

pub const PLANET_RECORD_SIZE: usize = 97;
pub const PLANET_RECORD_COUNT: usize = 20;
pub const PLANETS_DAT_SIZE: usize = PLANET_RECORD_SIZE * PLANET_RECORD_COUNT;

pub const FLEET_RECORD_SIZE: usize = 54;
pub const FLEET_RECORD_COUNT: usize = 16;
pub const FLEETS_DAT_SIZE: usize = FLEET_RECORD_SIZE * FLEET_RECORD_COUNT;

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
        if self.owner_mode_raw() == 0xff {
            format!("rogue label='{}'", self.legacy_status_name_summary())
        } else if self.assigned_player_flag_raw() != 0
            || !self.assigned_player_handle_summary().is_empty()
            || !self.controlled_empire_name_summary().is_empty()
        {
            format!(
                "player handle='{}' empire='{}'",
                self.assigned_player_handle_summary(),
                self.controlled_empire_name_summary()
            )
        } else {
            format!("unowned label='{}'", self.legacy_status_name_summary())
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

    pub fn build_slot_raw(&self) -> u8 {
        self.raw[0x24]
    }

    pub fn build_kind_raw(&self) -> u8 {
        self.raw[0x2E]
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
    GuardBlockadeWorld,
    BombardWorld,
    ViewWorld,
    ColonizeWorld,
    JoinAnotherFleet,
    Unknown(u8),
}

impl FleetStandingOrderKind {
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::HoldPosition,
            1 => Self::MoveOnly,
            2 => Self::SeekHome,
            3 => Self::PatrolSector,
            5 => Self::GuardBlockadeWorld,
            6 => Self::BombardWorld,
            9 => Self::ViewWorld,
            12 => Self::ColonizeWorld,
            13 => Self::JoinAnotherFleet,
            other => Self::Unknown(other),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HoldPosition => "hold",
            Self::MoveOnly => "move",
            Self::SeekHome => "seek_home",
            Self::PatrolSector => "patrol",
            Self::GuardBlockadeWorld => "guard_blockade",
            Self::BombardWorld => "bombard",
            Self::ViewWorld => "view",
            Self::ColonizeWorld => "colonize",
            Self::JoinAnotherFleet => "join_fleet",
            Self::Unknown(_) => "unknown",
        }
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::HoldPosition => "Hold position",
            Self::MoveOnly => "Move fleet",
            Self::SeekHome => "Seek home",
            Self::PatrolSector => "Patrol sector",
            Self::GuardBlockadeWorld => "Guard/blockade world",
            Self::BombardWorld => "Bombard world",
            Self::ViewWorld => "View world",
            Self::ColonizeWorld => "Colonize world",
            Self::JoinAnotherFleet => "Join another fleet",
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

    pub fn mission_code(&self) -> u8 {
        self.raw[0x0A]
    }

    pub fn home_system_coords_raw(&self) -> [u8; 2] {
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
            FleetStandingOrderKind::GuardBlockadeWorld => {
                format!("Guard/blockade world in System ({x},{y})")
            }
            FleetStandingOrderKind::BombardWorld => {
                format!("Bombard world in System ({x},{y})")
            }
            FleetStandingOrderKind::ViewWorld => format!("View world in System ({x},{y})"),
            FleetStandingOrderKind::ColonizeWorld => {
                format!("Colonize world in System ({x},{y})")
            }
            FleetStandingOrderKind::JoinAnotherFleet => {
                format!("Join another fleet at raw target ({x},{y})")
            }
            FleetStandingOrderKind::Unknown(code) => {
                format!("Unknown order {code} target ({x},{y})")
            }
        }
    }

    pub fn rules_of_engagement(&self) -> u8 {
        self.raw[0x25]
    }

    pub fn cruiser_count(&self) -> u8 {
        self.raw[0x28]
    }

    pub fn destroyer_count(&self) -> u8 {
        self.raw[0x2A]
    }

    pub fn etac_count(&self) -> u8 {
        self.raw[0x30]
    }

    pub fn ship_composition_summary(&self) -> String {
        let parts = [
            ("CA", self.cruiser_count()),
            ("DD", self.destroyer_count()),
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
    pub records: [FleetRecord; FLEET_RECORD_COUNT],
}

impl FleetDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, FLEETS_DAT_SIZE, "FLEETS.DAT")?;
        Ok(Self {
            records: std::array::from_fn(|idx| {
                let start = idx * FLEET_RECORD_SIZE;
                let end = start + FLEET_RECORD_SIZE;
                FleetRecord {
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
    fn round_trip_setup_dat() {
        let bytes = read_fixture("SETUP.DAT");
        let parsed = SetupDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
        assert_eq!(parsed.version_tag(), b"EC151");
        assert_eq!(parsed.option_prefix(), &[4, 3, 4, 3, 1, 1, 1, 1]);
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
    fn shipped_fleets_dat_is_not_the_initialized_layout() {
        let bytes = read_fixture("FLEETS.DAT");
        let err = FleetDat::parse(&bytes).unwrap_err();
        assert_eq!(
            err,
            ParseError::WrongSize {
                file_type: "FLEETS.DAT",
                expected: FLEETS_DAT_SIZE,
                actual: 702,
            }
        );
    }

    #[test]
    fn round_trip_initialized_fleets_dat() {
        let bytes = read_initialized_fixture("FLEETS.DAT");
        let parsed = FleetDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
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
    }

    #[test]
    fn can_set_purge_after_turns_raw() {
        let mut setup = SetupDat::parse(&read_fixture("SETUP.DAT")).unwrap();
        assert_eq!(setup.purge_after_turns_raw(), 0);
        setup.set_purge_after_turns_raw(1);
        assert_eq!(setup.purge_after_turns_raw(), 1);
        assert_eq!(setup.raw[518], 1);
    }
}
