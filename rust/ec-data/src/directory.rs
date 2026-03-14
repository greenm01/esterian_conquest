use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    BaseDat, BaseRecord, ConquestDat, DiplomaticRelation, FleetDat, FleetRecord, IPBM_RECORD_SIZE,
    IpbmDat, IpbmRecord, ParseError, PlanetDat, PlayerDat, SetupDat,
};

const CURRENT_KNOWN_POST_MAINT_CONQUEST_CONTROL_HEADER: [u8; 0x55] = [
    0xb9, 0x0b, 0x04, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x64, 0x00, 0x64, 0x00, 0x64, 0x00,
    0x64, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x74, 0x33, 0x00, 0x00, 0x00, 0x00,
    0x75, 0x03, 0x65, 0x20, 0x00, 0x00, 0x7e, 0x04, 0x20, 0x74, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3b, 0x86, 0xfe, 0xfc, 0x28, 0x8b, 0x01, 0x01, 0x01, 0x01,
    0xff, 0x00, 0x00, 0x00, 0xc2, 0x00, 0x00, 0x08, 0x6f, 0x00, 0x01, 0x6f, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x6a, 0x8d, 0x35,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreGameData {
    pub player: PlayerDat,
    pub planets: PlanetDat,
    pub fleets: FleetDat,
    pub bases: BaseDat,
    pub ipbm: IpbmDat,
    pub setup: SetupDat,
    pub conquest: ConquestDat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentKnownComplianceStatus {
    pub fleet_order: bool,
    pub planet_build: bool,
    pub guard_starbase: bool,
    pub ipbm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentKnownKeyWordSummary {
    pub player_starbase_count: u16,
    pub player_ipbm_count: u16,
    pub fleet1_local_slot: Option<u16>,
    pub fleet1_id: Option<u16>,
    pub fleet1_guard_index: Option<u8>,
    pub fleet1_guard_enable: Option<u8>,
    pub fleet1_target: Option<[u8; 2]>,
    pub base1_summary: Option<u16>,
    pub base1_id: Option<u8>,
    pub base1_chain: Option<u16>,
    pub base1_coords: Option<[u8; 2]>,
    pub ipbm_record_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentKnownGuardStarbaseLinkageSummary {
    pub player_record_index_1_based: usize,
    pub fleet_record_index_1_based: usize,
    pub player_starbase_count: u16,
    pub fleet_order: u8,
    pub fleet_local_slot: u16,
    pub fleet_id: u16,
    pub guard_index: u8,
    pub guard_enable: u8,
    pub target_coords: [u8; 2],
    pub selected_base_present: bool,
    pub selected_base_summary_word: Option<u16>,
    pub selected_base_id: Option<u8>,
    pub selected_base_chain_word: Option<u16>,
    pub selected_base_coords: Option<[u8; 2]>,
    pub selected_base_trailing_coords: Option<[u8; 2]>,
    pub selected_base_owner_empire: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFileDiffCount {
    pub name: &'static str,
    pub differing_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFileDiffOffsets {
    pub name: &'static str,
    pub differing_offsets: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignState {
    CivilDisorder,
    Rogue,
    Stable,
    MarginalExistence,
    DefectionRisk,
    Defeated,
}

#[derive(Debug)]
pub enum GameDirectoryError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: ParseError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameStateMutationError {
    MissingFleetRecord { index_1_based: usize },
    MissingIpbmRecord { index_1_based: usize },
    MissingPlanetRecord { index_1_based: usize },
    MissingPlayerRecord { index_1_based: usize },
}

impl std::fmt::Display for GameDirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::Parse { path, source } => write!(f, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for GameDirectoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

impl std::fmt::Display for GameStateMutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFleetRecord { index_1_based } => {
                write!(f, "missing fleet record {}", index_1_based)
            }
            Self::MissingIpbmRecord { index_1_based } => {
                write!(f, "missing IPBM record {}", index_1_based)
            }
            Self::MissingPlanetRecord { index_1_based } => {
                write!(f, "missing planet record {}", index_1_based)
            }
            Self::MissingPlayerRecord { index_1_based } => {
                write!(f, "missing player record {}", index_1_based)
            }
        }
    }
}

impl std::error::Error for GameStateMutationError {}

impl CoreGameData {
    pub fn load(dir: &Path) -> Result<Self, GameDirectoryError> {
        Ok(Self {
            player: load_parsed(dir, "PLAYER.DAT", PlayerDat::parse)?,
            planets: load_parsed(dir, "PLANETS.DAT", PlanetDat::parse)?,
            fleets: load_parsed(dir, "FLEETS.DAT", FleetDat::parse)?,
            bases: load_parsed(dir, "BASES.DAT", BaseDat::parse)?,
            ipbm: load_parsed(dir, "IPBM.DAT", IpbmDat::parse)?,
            setup: load_parsed(dir, "SETUP.DAT", SetupDat::parse)?,
            conquest: load_parsed(dir, "CONQUEST.DAT", ConquestDat::parse)?,
        })
    }

    pub fn save(&self, dir: &Path) -> Result<(), GameDirectoryError> {
        save_bytes(dir, "PLAYER.DAT", &self.player.to_bytes())?;
        save_bytes(dir, "PLANETS.DAT", &self.planets.to_bytes())?;
        save_bytes(dir, "FLEETS.DAT", &self.fleets.to_bytes())?;
        save_bytes(dir, "BASES.DAT", &self.bases.to_bytes())?;
        save_bytes(dir, "IPBM.DAT", &self.ipbm.to_bytes())?;
        save_bytes(dir, "SETUP.DAT", &self.setup.to_bytes())?;
        save_bytes(dir, "CONQUEST.DAT", &self.conquest.to_bytes())?;
        Ok(())
    }

    pub fn player1_starbase_count_current_known(&self) -> usize {
        self.player
            .records
            .first()
            .map(|record| record.starbase_count_raw() as usize)
            .unwrap_or(0)
    }

    pub fn player1_owned_base_record_count_current_known(&self) -> usize {
        self.player_owned_base_record_count_current_known(1)
    }

    pub fn player_owned_planet_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.planets
            .records
            .iter()
            .filter(|record| record.owner_empire_slot_raw() as usize == player_record_index_1_based)
            .count()
    }

    pub fn player_owned_base_record_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.bases
            .records
            .iter()
            .filter(|record| record.owner_empire_raw() as usize == player_record_index_1_based)
            .count()
    }

    pub fn player1_ipbm_count_current_known(&self) -> usize {
        self.player
            .records
            .first()
            .map(|record| record.ipbm_count_raw() as usize)
            .unwrap_or(0)
    }

    pub fn empire_campaign_state(&self, empire_raw: u8) -> Option<CampaignState> {
        let player_idx = empire_raw.checked_sub(1)? as usize;
        let player = self.player.records.get(player_idx)?;

        match player.owner_mode_raw() {
            0x00 => return Some(CampaignState::CivilDisorder),
            0xff => return Some(CampaignState::Rogue),
            0x01 => {}
            _ => {}
        }

        let owned_planets = self
            .planets
            .records
            .iter()
            .filter(|planet| planet.owner_empire_slot_raw() == empire_raw)
            .count();
        if owned_planets > 0 {
            return Some(CampaignState::Stable);
        }

        let mut has_any_fleet_presence = false;
        let mut can_recover_planet = false;

        for fleet in &self.fleets.records {
            if fleet.owner_empire_raw() != empire_raw {
                continue;
            }

            let has_presence = fleet.scout_count() > 0
                || fleet.battleship_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.destroyer_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.army_count() > 0
                || fleet.etac_count() > 0;
            if !has_presence {
                continue;
            }
            has_any_fleet_presence = true;

            if fleet.etac_count() > 0
                || (fleet.troop_transport_count() > 0 && fleet.army_count() > 0)
            {
                can_recover_planet = true;
                break;
            }
        }

        if can_recover_planet {
            Some(CampaignState::MarginalExistence)
        } else if has_any_fleet_presence {
            Some(CampaignState::DefectionRisk)
        } else {
            Some(CampaignState::Defeated)
        }
    }

    pub fn current_known_core_state_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let expected_ipbm = self.player1_ipbm_count_current_known();

        errors.extend(self.current_known_planet_owner_slot_errors());
        errors.extend(self.current_known_base_owner_empire_errors());
        errors.extend(self.current_known_player1_starbase_count_errors());
        errors.extend(self.current_known_initialized_fleet_block_errors());
        errors.extend(self.current_known_initialized_fleet_payload_errors());
        errors.extend(self.current_known_initialized_fleet_mission_errors());
        errors.extend(self.current_known_homeworld_seed_errors());
        errors.extend(self.current_known_initialized_planet_ownership_errors());
        errors.extend(self.current_known_homeworld_seed_payload_errors());
        errors.extend(self.current_known_unowned_planet_payload_errors());
        errors.extend(self.current_known_empty_auxiliary_state_errors());
        errors.extend(self.current_known_initialized_homeworld_alignment_errors());
        errors.extend(self.current_known_setup_baseline_errors());
        errors.extend(self.current_known_conquest_baseline_errors());
        if self.ipbm.records.len() != expected_ipbm {
            errors.push(format!(
                "IPBM.DAT record count expected {}, got {}",
                expected_ipbm,
                self.ipbm.records.len()
            ));
        }

        errors
    }

    pub fn current_known_player1_starbase_count_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let expected_bases = self.player1_starbase_count_current_known();
        let owned_bases = self.player1_owned_base_record_count_current_known();
        if owned_bases != expected_bases {
            errors.push(format!(
                "PLAYER[1]-owned BASES.DAT record count expected {}, got {}",
                expected_bases, owned_bases
            ));
        }
        errors
    }

    pub fn current_known_planet_owner_slot_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        for (idx, record) in self.planets.records.iter().enumerate() {
            let owner = record.owner_empire_slot_raw() as usize;
            if owner > player_count {
                errors.push(format!(
                    "PLANET[{}].owner_empire_slot expected <= {}, got {}",
                    idx + 1,
                    player_count,
                    owner
                ));
            }
        }
        errors
    }

    pub fn current_known_base_owner_empire_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        for (idx, record) in self.bases.records.iter().enumerate() {
            let owner = record.owner_empire_raw() as usize;
            if owner == 0 || owner > player_count {
                errors.push(format!(
                    "BASES[{}].owner_empire expected 1..={}, got {}",
                    idx + 1,
                    player_count,
                    owner
                ));
            }
        }
        errors
    }

    pub fn sync_player1_current_known_counts(&mut self) {
        let starbase_count = self.player1_owned_base_record_count_current_known() as u16;
        let ipbm_count = self.ipbm.records.len() as u16;
        if let Some(player1) = self.player.records.first_mut() {
            player1.set_starbase_count_raw(starbase_count);
            player1.set_ipbm_count_raw(ipbm_count);
        }
    }

    pub fn sync_current_known_baseline_controls_and_counts(&mut self) {
        self.sync_player1_current_known_counts();

        self.setup.raw[..5].copy_from_slice(b"EC151");
        self.setup.raw[5..13].copy_from_slice(&[4, 3, 4, 3, 1, 1, 1, 1]);
        self.setup.set_snoop_enabled(true);
        self.setup.set_max_time_between_keys_minutes_raw(10);
        self.setup.set_remote_timeout_enabled(true);
        self.setup.set_local_timeout_enabled(false);
        self.setup.set_minimum_time_granted_minutes_raw(0);
        self.setup.set_purge_after_turns_raw(0);
        self.setup.set_autopilot_inactive_turns_raw(0);

        if !matches!(self.conquest.game_year(), 3000 | 3001) {
            self.conquest.raw[0..2].copy_from_slice(&3001u16.to_le_bytes());
        }
        self.conquest.raw[2] = 4;
        self.conquest.raw[3..10].copy_from_slice(&[1; 7]);
    }

    pub fn sync_current_known_initialized_fleet_baseline(&mut self) {
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);
        let homeworld_coords = self.player_homeworld_seed_coords_current_known();

        let mut records = Vec::with_capacity(expected_fleet_count);
        for block_idx in 0..player_count {
            let coords = homeworld_coords
                .get(block_idx)
                .and_then(|coords| *coords)
                .unwrap_or([0, 0]);

            for slot_idx in 0..4 {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let mut record = FleetRecord::new_zeroed();
                let fleet_id = fleet_record_index_1_based as u16;
                let local_slot = (slot_idx + 1) as u16;
                let owner_empire = (block_idx + 1) as u8;
                let prev = if slot_idx == 0 { 0 } else { fleet_id - 1 };
                let next = if slot_idx == 3 { 0 } else { fleet_id + 1 };

                record.set_local_slot_word_raw(local_slot);
                record.set_owner_empire_raw(owner_empire);
                record.set_next_fleet_link_word_raw(next);
                record.set_fleet_id_word_raw(fleet_id);
                record.set_previous_fleet_id(prev as u8);
                record.set_max_speed(if slot_idx < 2 { 3 } else { 6 });
                record.set_current_speed(0);
                record.set_current_location_coords_raw(coords);
                record.set_tuple_a_payload_raw([0x80, 0, 0, 0, 0]);
                record.set_tuple_b_payload_raw([0x80, 0, 0, 0, 0]);
                record.set_tuple_c_payload_raw([0x81, 0, 0, 0, 0]);
                record.set_standing_order_kind(crate::Order::GuardBlockadeWorld);
                record.set_standing_order_target_coords_raw(coords);
                record.set_mission_aux_bytes([1, 0]);
                record.set_scout_count(0);
                record.set_rules_of_engagement(6);
                record.set_battleship_count(0);
                record.set_cruiser_count(if slot_idx < 2 { 1 } else { 0 });
                record.set_destroyer_count(if slot_idx < 2 { 0 } else { 1 });
                record.set_troop_transport_count(0);
                record.set_army_count(0);
                record.set_etac_count(if slot_idx < 2 { 1 } else { 0 });

                records.push(record);
            }
        }

        self.fleets.records = records;
    }

    pub fn sync_current_known_initialized_planet_payloads(&mut self) {
        let player_count = self.conquest.player_count() as usize;

        for record in &mut self.planets.records {
            let owner = record.owner_empire_slot_raw() as usize;
            if record.is_homeworld_seed_ignoring_name() && (1..=player_count).contains(&owner) {
                record.set_potential_production_raw([100, 135]);
                record.set_factories_raw([0, 0, 0, 0, 72, 134]);
                record.set_stored_goods_raw(0);
                record.set_planet_tax_rate_raw(12);
                record.set_status_or_name_summary_raw("Not Named Yet");
                for slot in 0..10 {
                    record.set_build_count_raw(slot, 0);
                    record.set_build_kind_raw(slot, 0);
                }
                for slot in 0..6 {
                    record.set_stardock_count_raw(slot, 0);
                    record.set_stardock_kind_raw(slot, 0);
                }
                record.set_population_raw([0; 6]);
                record.set_army_count_raw(10);
                record.set_ground_batteries_raw(4);
                record.set_ownership_status_raw(2);
            } else if owner == 0 {
                record.set_status_or_name_prefix_raw("Unowned");
                record.set_planet_tax_rate_raw(0);
                record.set_factories_raw([0; 6]);
                record.set_stored_goods_raw(0);
                for slot in 0..10 {
                    record.set_build_count_raw(slot, 0);
                    record.set_build_kind_raw(slot, 0);
                }
                for slot in 0..6 {
                    record.set_stardock_count_raw(slot, 0);
                    record.set_stardock_kind_raw(slot, 0);
                }
                record.set_population_raw([0; 6]);
                record.set_army_count_raw(0);
                record.set_ground_batteries_raw(0);
                record.set_ownership_status_raw(0);
            }
        }
    }

    pub fn sync_current_known_initialized_post_maint_baseline(&mut self) {
        self.sync_current_known_empty_auxiliary_state();
        self.sync_current_known_baseline_controls_and_counts();
        self.sync_current_known_initialized_fleet_baseline();
        self.sync_current_known_initialized_planet_payloads();
        self.sync_current_known_initialized_conquest_post_maint_header();
    }

    pub fn sync_current_known_initialized_conquest_post_maint_header(&mut self) {
        self.conquest.raw[..CURRENT_KNOWN_POST_MAINT_CONQUEST_CONTROL_HEADER.len()]
            .copy_from_slice(&CURRENT_KNOWN_POST_MAINT_CONQUEST_CONTROL_HEADER);
    }

    pub fn player_owned_base_record_counts_current_known(&self) -> Vec<usize> {
        (1..=self.player.records.len())
            .map(|player_record_index_1_based| {
                self.player_owned_base_record_count_current_known(player_record_index_1_based)
            })
            .collect()
    }

    pub fn player_owned_planet_counts_current_known(&self) -> Vec<usize> {
        (1..=self.player.records.len())
            .map(|player_record_index_1_based| {
                self.player_owned_planet_count_current_known(player_record_index_1_based)
            })
            .collect()
    }

    pub fn player_homeworld_seed_coords_current_known(&self) -> Vec<Option<[u8; 2]>> {
        let player_count = self.conquest.player_count() as usize;
        (1..=player_count)
            .map(|player_record_index_1_based| {
                self.planets
                    .records
                    .iter()
                    .find(|record| {
                        record.owner_empire_slot_raw() as usize == player_record_index_1_based
                            && record.is_homeworld_seed_ignoring_name()
                    })
                    .map(|record| record.coords_raw())
            })
            .collect()
    }

    pub fn current_known_homeworld_seed_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        for player_record_index_1_based in 1..=player_count {
            let matches = self
                .planets
                .records
                .iter()
                .enumerate()
                .filter(|(_, record)| {
                    record.owner_empire_slot_raw() as usize == player_record_index_1_based
                        && record.is_homeworld_seed_ignoring_name()
                })
                .map(|(idx, record)| (idx + 1, record.coords_raw()))
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                errors.push(format!(
                    "PLAYER[{}] homeworld seed expected 1 owned 'Not Named Yet' planet, got {}",
                    player_record_index_1_based,
                    matches.len()
                ));
            }
        }
        errors
    }

    pub fn current_known_initialized_planet_ownership_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;

        for (idx, record) in self.planets.records.iter().enumerate() {
            let planet_index_1_based = idx + 1;
            let owner = record.owner_empire_slot_raw() as usize;
            let is_homeworld_seed = record.is_homeworld_seed_ignoring_name();

            if owner != 0 && !is_homeworld_seed {
                errors.push(format!(
                    "PLANET[{}] expected unowned non-homeworld baseline, got owner {}",
                    planet_index_1_based, owner
                ));
            }

            if is_homeworld_seed {
                if owner == 0 || owner > player_count {
                    errors.push(format!(
                        "PLANET[{}] homeworld seed expected owner 1..={}, got {}",
                        planet_index_1_based, player_count, owner
                    ));
                }
                if owner != 0 && record.ownership_status_raw() != 2 {
                    errors.push(format!(
                        "PLANET[{}].ownership_status expected 2 for owned homeworld seed, got {}",
                        planet_index_1_based,
                        record.ownership_status_raw()
                    ));
                }
            }
        }

        for player_record_index_1_based in 1..=player_count {
            let owned_count =
                self.player_owned_planet_count_current_known(player_record_index_1_based);
            if owned_count != 1 {
                errors.push(format!(
                    "PLAYER[{}] owned_planet_count expected 1, got {}",
                    player_record_index_1_based, owned_count
                ));
            }
        }

        errors
    }

    pub fn current_known_homeworld_seed_payload_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (idx, record) in self.planets.records.iter().enumerate() {
            if !record.is_homeworld_seed_ignoring_name() {
                continue;
            }
            let planet_index_1_based = idx + 1;
            if record.header_value_raw() != 100 {
                errors.push(format!(
                    "PLANET[{}].header_value expected 100 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.header_value_raw()
                ));
            }
            if record.raw[0x03] != 135 {
                errors.push(format!(
                    "PLANET[{}].header[3] expected 135 for homeworld seed, got {}",
                    planet_index_1_based, record.raw[0x03]
                ));
            }
            if record.ownership_status_raw() != 2 {
                errors.push(format!(
                    "PLANET[{}].ownership_status expected 2 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.ownership_status_raw()
                ));
            }
            if record.army_count_raw() != 10 {
                errors.push(format!(
                    "PLANET[{}].army_count_raw expected 10 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.army_count_raw()
                ));
            }
            if record.ground_batteries_raw() != 4 {
                errors.push(format!(
                    "PLANET[{}].ground_batteries_raw expected 4 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.ground_batteries_raw()
                ));
            }
            if record.planet_tax_rate_raw() != 12 {
                errors.push(format!(
                    "PLANET[{}].planet_tax_rate expected 12 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.planet_tax_rate_raw()
                ));
            }
            if record.factories_raw() != [0, 0, 0, 0, 72, 134] {
                errors.push(format!(
                    "PLANET[{}].factories_raw expected [0, 0, 0, 0, 72, 134] for homeworld seed, got {:?}",
                    planet_index_1_based,
                    record.factories_raw()
                ));
            }
            if record.stored_goods_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].stored_goods_raw expected 0 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.stored_goods_raw()
                ));
            }
            if record.population_raw() != [0; 6] {
                errors.push(format!(
                    "PLANET[{}].population_raw expected all zeroes for homeworld seed, got {:?}",
                    planet_index_1_based,
                    record.population_raw()
                ));
            }
            if (0..10)
                .any(|slot| record.build_count_raw(slot) != 0 || record.build_kind_raw(slot) != 0)
            {
                errors.push(format!(
                    "PLANET[{}] build queue expected all zeroes for homeworld seed",
                    planet_index_1_based
                ));
            }
            if (0..6).any(|slot| {
                record.stardock_kind_raw(slot) != 0 || record.stardock_count_raw(slot) != 0
            }) {
                errors.push(format!(
                    "PLANET[{}] stardock expected all zeroes for homeworld seed",
                    planet_index_1_based
                ));
            }
        }

        errors
    }

    pub fn current_known_unowned_planet_payload_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (idx, record) in self.planets.records.iter().enumerate() {
            if record.is_homeworld_seed_ignoring_name() {
                continue;
            }
            let planet_index_1_based = idx + 1;
            if record.status_or_name_summary() != "Unowned" {
                errors.push(format!(
                    "PLANET[{}].status_or_name expected 'Unowned', got {:?}",
                    planet_index_1_based,
                    record.status_or_name_summary()
                ));
            }
            if record.owner_empire_slot_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].owner_empire_slot expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.owner_empire_slot_raw()
                ));
            }
            if record.ownership_status_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].ownership_status expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.ownership_status_raw()
                ));
            }
            if record.army_count_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].army_count_raw expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.army_count_raw()
                ));
            }
            if record.planet_tax_rate_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].planet_tax_rate expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.planet_tax_rate_raw()
                ));
            }
            if record.ground_batteries_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].ground_batteries_raw expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.ground_batteries_raw()
                ));
            }
            if record.factories_raw() != [0; 6] {
                errors.push(format!(
                    "PLANET[{}].factories_raw expected all zeroes for unowned baseline, got {:?}",
                    planet_index_1_based,
                    record.factories_raw()
                ));
            }
            if record.stored_goods_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].stored_goods_raw expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.stored_goods_raw()
                ));
            }
            if record.population_raw() != [0; 6] {
                errors.push(format!(
                    "PLANET[{}].population_raw expected all zeroes for unowned baseline, got {:?}",
                    planet_index_1_based,
                    record.population_raw()
                ));
            }
            if (0..10)
                .any(|slot| record.build_count_raw(slot) != 0 || record.build_kind_raw(slot) != 0)
            {
                errors.push(format!(
                    "PLANET[{}] build queue expected all zeroes for unowned baseline",
                    planet_index_1_based
                ));
            }
            if (0..6).any(|slot| {
                record.stardock_kind_raw(slot) != 0 || record.stardock_count_raw(slot) != 0
            }) {
                errors.push(format!(
                    "PLANET[{}] stardock expected all zeroes for unowned baseline",
                    planet_index_1_based
                ));
            }
        }

        errors
    }

    pub fn current_known_empty_auxiliary_state_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !self.bases.records.is_empty() {
            errors.push(format!(
                "BASES.DAT expected empty auxiliary baseline, got {} records",
                self.bases.records.len()
            ));
        }
        if !self.ipbm.records.is_empty() {
            errors.push(format!(
                "IPBM.DAT expected empty auxiliary baseline, got {} records",
                self.ipbm.records.len()
            ));
        }

        let guarding_fleet_count = self.guarding_fleet_record_indexes_current_known().len();
        if guarding_fleet_count != 0 {
            errors.push(format!(
                "guarding fleet count expected 0 in empty auxiliary baseline, got {}",
                guarding_fleet_count
            ));
        }

        errors
    }

    pub fn sync_current_known_empty_auxiliary_state(&mut self) {
        self.bases.records.clear();
        self.ipbm.records.clear();
    }

    pub fn current_known_setup_baseline_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.setup.version_tag() != b"EC151" {
            errors.push(format!(
                "SETUP.DAT.version_tag expected EC151, got {:?}",
                self.setup.version_tag()
            ));
        }
        if self.setup.option_prefix() != [4, 3, 4, 3, 1, 1, 1, 1] {
            errors.push(format!(
                "SETUP.DAT.option_prefix expected [4, 3, 4, 3, 1, 1, 1, 1], got {:?}",
                self.setup.option_prefix()
            ));
        }
        if !self.setup.snoop_enabled() {
            errors.push("SETUP.DAT.snoop expected enabled in baseline".to_string());
        }
        if self.setup.max_time_between_keys_minutes_raw() != 10 {
            errors.push(format!(
                "SETUP.DAT.max_time_between_keys expected 10, got {}",
                self.setup.max_time_between_keys_minutes_raw()
            ));
        }
        if !self.setup.remote_timeout_enabled() {
            errors.push("SETUP.DAT.remote_timeout expected enabled in baseline".to_string());
        }
        if self.setup.local_timeout_enabled() {
            errors.push("SETUP.DAT.local_timeout expected disabled in baseline".to_string());
        }
        if self.setup.minimum_time_granted_minutes_raw() != 0 {
            errors.push(format!(
                "SETUP.DAT.minimum_time_granted expected 0, got {}",
                self.setup.minimum_time_granted_minutes_raw()
            ));
        }
        if self.setup.purge_after_turns_raw() != 0 {
            errors.push(format!(
                "SETUP.DAT.purge_after_turns expected 0, got {}",
                self.setup.purge_after_turns_raw()
            ));
        }
        if self.setup.autopilot_inactive_turns_raw() != 0 {
            errors.push(format!(
                "SETUP.DAT.autopilot_inactive_turns expected 0, got {}",
                self.setup.autopilot_inactive_turns_raw()
            ));
        }

        errors
    }

    pub fn current_known_conquest_baseline_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !matches!(self.conquest.game_year(), 3000 | 3001) {
            errors.push(format!(
                "CONQUEST.DAT.game_year expected 3000 or 3001 for preserved initialized/post-maint baseline, got {}",
                self.conquest.game_year()
            ));
        }
        if self.conquest.player_count() != 4 {
            errors.push(format!(
                "CONQUEST.DAT.player_count expected 4, got {}",
                self.conquest.player_count()
            ));
        }
        if self.conquest.maintenance_schedule_bytes() != [1; 7] {
            errors.push(format!(
                "CONQUEST.DAT.maintenance_schedule expected [1, 1, 1, 1, 1, 1, 1], got {:?}",
                self.conquest.maintenance_schedule_bytes()
            ));
        }

        errors
    }

    pub fn player_fleet_chain_heads_current_known(&self) -> Vec<usize> {
        self.player
            .records
            .iter()
            .map(|record| record.fleet_chain_head_raw() as usize)
            .collect()
    }

    pub fn looks_like_initialized_fleet_blocks_current_known(&self) -> bool {
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);
        player_count > 0
            && self.fleets.records.len() == expected_fleet_count
            && self
                .fleets
                .records
                .chunks_exact(4)
                .enumerate()
                .all(|(block_idx, group)| {
                    group.iter().enumerate().all(|(slot_idx, record)| {
                        let expected_fleet_id = (block_idx * 4 + slot_idx + 1) as u8;
                        let expected_local_slot = (slot_idx + 1) as u8;
                        let expected_prev = if slot_idx == 0 {
                            0
                        } else {
                            expected_fleet_id - 1
                        };
                        let expected_next = if slot_idx == 3 {
                            0
                        } else {
                            expected_fleet_id + 1
                        };
                        record.fleet_id() == expected_fleet_id
                            && record.local_slot() == expected_local_slot
                            && record.previous_fleet_id() == expected_prev
                            && record.next_fleet_id() == expected_next
                    })
                })
    }

    pub fn current_known_initialized_fleet_block_head_ids(&self) -> Vec<usize> {
        self.fleets
            .records
            .chunks(4)
            .filter_map(|group| group.first())
            .map(|record| record.fleet_id() as usize)
            .collect()
    }

    pub fn current_known_initialized_fleet_block_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            errors.push(format!(
                "FLEETS.DAT record count expected {}, got {}",
                expected_fleet_count,
                self.fleets.records.len()
            ));
            return errors;
        }

        for (block_idx, group) in self.fleets.records.chunks_exact(4).enumerate() {
            for (slot_idx, record) in group.iter().enumerate() {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let expected_fleet_id = fleet_record_index_1_based as u8;
                let expected_local_slot = (slot_idx + 1) as u8;
                let expected_prev = if slot_idx == 0 {
                    0
                } else {
                    expected_fleet_id - 1
                };
                let expected_next = if slot_idx == 3 {
                    0
                } else {
                    expected_fleet_id + 1
                };

                if record.fleet_id() != expected_fleet_id {
                    errors.push(format!(
                        "FLEET[{}].fleet_id expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_fleet_id,
                        record.fleet_id()
                    ));
                }
                if record.local_slot() != expected_local_slot {
                    errors.push(format!(
                        "FLEET[{}].local_slot expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_local_slot,
                        record.local_slot()
                    ));
                }
                if record.previous_fleet_id() != expected_prev {
                    errors.push(format!(
                        "FLEET[{}].previous_fleet_id expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_prev,
                        record.previous_fleet_id()
                    ));
                }
                if record.next_fleet_id() != expected_next {
                    errors.push(format!(
                        "FLEET[{}].next_fleet_id expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_next,
                        record.next_fleet_id()
                    ));
                }
            }
        }
        errors
    }

    pub fn current_known_initialized_fleet_payload_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            return errors;
        }

        for (block_idx, group) in self.fleets.records.chunks_exact(4).enumerate() {
            let expected_loc = group[0].current_location_coords_raw();
            let expected_mission = group[0].mission_param_bytes().to_vec();

            for (slot_idx, record) in group.iter().enumerate() {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let expected_owner_empire = (block_idx + 1) as u8;
                let expected_max_speed = if slot_idx < 2 { 3 } else { 6 };
                let expected_cur_speed = 0;
                let expected_ca = if slot_idx < 2 { 1 } else { 0 };
                let expected_dd = if slot_idx < 2 { 0 } else { 1 };
                let expected_et = if slot_idx < 2 { 1 } else { 0 };
                let expected_roe = 6;

                if record.current_location_coords_raw() != expected_loc {
                    errors.push(format!(
                        "FLEET[{}].current_location expected {:?}, got {:?}",
                        fleet_record_index_1_based,
                        expected_loc,
                        record.current_location_coords_raw()
                    ));
                }
                if record.mission_param_bytes() != expected_mission.as_slice() {
                    errors.push(format!(
                        "FLEET[{}].mission_param_bytes expected {:?}, got {:?}",
                        fleet_record_index_1_based,
                        expected_mission,
                        record.mission_param_bytes()
                    ));
                }
                if record.owner_empire_raw() != expected_owner_empire {
                    errors.push(format!(
                        "FLEET[{}].owner_empire expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_owner_empire,
                        record.owner_empire_raw()
                    ));
                }
                if record.max_speed() != expected_max_speed {
                    errors.push(format!(
                        "FLEET[{}].max_speed expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_max_speed,
                        record.max_speed()
                    ));
                }
                if record.current_speed() != expected_cur_speed {
                    errors.push(format!(
                        "FLEET[{}].current_speed expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_cur_speed,
                        record.current_speed()
                    ));
                }
                if record.rules_of_engagement() != expected_roe {
                    errors.push(format!(
                        "FLEET[{}].roe expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_roe,
                        record.rules_of_engagement()
                    ));
                }
                if record.cruiser_count() != expected_ca {
                    errors.push(format!(
                        "FLEET[{}].cruiser_count expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_ca,
                        record.cruiser_count()
                    ));
                }
                if record.destroyer_count() != expected_dd {
                    errors.push(format!(
                        "FLEET[{}].destroyer_count expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_dd,
                        record.destroyer_count()
                    ));
                }
                if record.etac_count() != expected_et {
                    errors.push(format!(
                        "FLEET[{}].etac_count expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_et,
                        record.etac_count()
                    ));
                }
                if record.tuple_a_payload_raw() != [0x80, 0, 0, 0, 0] {
                    errors.push(format!(
                        "FLEET[{}].tuple_a_payload expected [128, 0, 0, 0, 0], got {:?}",
                        fleet_record_index_1_based,
                        record.tuple_a_payload_raw()
                    ));
                }
                if record.tuple_b_payload_raw() != [0x80, 0, 0, 0, 0] {
                    errors.push(format!(
                        "FLEET[{}].tuple_b_payload expected [128, 0, 0, 0, 0], got {:?}",
                        fleet_record_index_1_based,
                        record.tuple_b_payload_raw()
                    ));
                }
                if record.tuple_c_payload_raw() != [0x81, 0, 0, 0, 0] {
                    errors.push(format!(
                        "FLEET[{}].tuple_c_payload expected [129, 0, 0, 0, 0], got {:?}",
                        fleet_record_index_1_based,
                        record.tuple_c_payload_raw()
                    ));
                }
            }
        }

        errors
    }

    pub fn current_known_initialized_fleet_mission_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            return errors;
        }

        let homeworld_coords = self.player_homeworld_seed_coords_current_known();
        for block_idx in 0..player_count {
            let Some(expected_coords) = homeworld_coords.get(block_idx).and_then(|coords| *coords)
            else {
                continue;
            };

            for slot_idx in 0..4 {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let record = &self.fleets.records[fleet_record_index_1_based - 1];
                if record.standing_order_code_raw() != 5 {
                    errors.push(format!(
                        "FLEET[{}].standing_order expected 5 for initialized baseline, got {}",
                        fleet_record_index_1_based,
                        record.standing_order_code_raw()
                    ));
                }
                if record.standing_order_target_coords_raw() != expected_coords {
                    errors.push(format!(
                        "FLEET[{}].standing_order_target expected {:?} for initialized baseline, got {:?}",
                        fleet_record_index_1_based,
                        expected_coords,
                        record.standing_order_target_coords_raw()
                    ));
                }
                if record.mission_aux_bytes() != [1, 0] {
                    errors.push(format!(
                        "FLEET[{}].mission_aux expected [1, 0] for initialized baseline, got {:?}",
                        fleet_record_index_1_based,
                        record.mission_aux_bytes()
                    ));
                }
            }
        }

        errors
    }

    pub fn current_known_initialized_homeworld_alignment_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            return errors;
        }

        let homeworld_coords = self.player_homeworld_seed_coords_current_known();
        for block_idx in 0..player_count {
            let Some(expected_coords) = homeworld_coords.get(block_idx).and_then(|coords| *coords)
            else {
                continue;
            };
            let fleet = &self.fleets.records[block_idx * 4];
            let actual_loc = fleet.current_location_coords_raw();
            let actual_target = fleet.standing_order_target_coords_raw();
            if actual_loc != expected_coords {
                errors.push(format!(
                    "FLEET block {} location expected homeworld seed {:?}, got {:?}",
                    block_idx + 1,
                    expected_coords,
                    actual_loc
                ));
            }
            if actual_target != expected_coords {
                errors.push(format!(
                    "FLEET block {} target expected homeworld seed {:?}, got {:?}",
                    block_idx + 1,
                    expected_coords,
                    actual_target
                ));
            }
        }

        errors
    }

    pub fn set_fleet_order(
        &mut self,
        record_index_1_based: usize,
        speed: u8,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<[u8; 2], GameStateMutationError> {
        let record = self
            .fleets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: record_index_1_based,
            })?;
        record.set_current_speed(speed);
        record.set_standing_order_code_raw(order_code);
        record.set_standing_order_target_coords_raw(target);
        let mut mission_aux = record.mission_aux_bytes();
        if let Some(value) = aux0 {
            mission_aux[0] = value;
        }
        if let Some(value) = aux1 {
            mission_aux[1] = value;
        }
        record.set_mission_aux_bytes(mission_aux);
        Ok(record.mission_aux_bytes())
    }

    pub fn set_planet_build(
        &mut self,
        record_index_1_based: usize,
        slot_raw: u8,
        kind_raw: u8,
    ) -> Result<(), GameStateMutationError> {
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        record.set_build_count_raw(0, slot_raw);
        record.set_build_kind_raw(0, kind_raw);
        Ok(())
    }

    /// Set a guard-starbase order on the specified fleet, update the owning player's starbase
    /// count, and append a single base record derived from that fleet's linkage words.
    ///
    /// All record indices are 1-based. `base_id` and `owner_empire` are passed explicitly so
    /// the caller (ec-cli scenario wiring) owns all fixture-specific constants.
    pub fn set_guard_starbase(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        target: [u8; 2],
        base_id: u8,
        owner_empire: u8,
    ) -> Result<(), GameStateMutationError> {
        let player = self
            .player
            .records
            .get_mut(player_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_index_1_based,
            })?;
        player.set_starbase_count_raw(1);

        let fleet = self.fleets.records.get_mut(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        fleet.set_standing_order_kind(crate::Order::GuardStarbase);
        fleet.set_standing_order_target_coords_raw(target);
        fleet.set_mission_aux_bytes([0x01, 0x01]);

        let base_summary_word = fleet.local_slot_word_raw();
        let base_chain_word = fleet.fleet_id_word_raw();
        let tuple_a = fleet.tuple_a_payload_raw();
        let tuple_b = fleet.tuple_b_payload_raw();
        let tuple_c = fleet.tuple_c_payload_raw();

        self.bases = BaseDat {
            records: vec![build_guard_starbase_base_record(
                target,
                base_id,
                base_summary_word,
                base_chain_word,
                owner_empire,
                tuple_a,
                tuple_b,
                tuple_c,
            )],
        };

        Ok(())
    }

    pub fn set_ipbm_zero_records(&mut self, count: u16) {
        if let Some(player1) = self.player.records.first_mut() {
            player1.set_ipbm_count_raw(count);
        }

        self.ipbm = IpbmDat {
            records: (0..count)
                .map(|_| IpbmRecord {
                    raw: [0u8; IPBM_RECORD_SIZE],
                })
                .collect(),
        };
    }

    pub fn set_ipbm_record_prefix(
        &mut self,
        record_index_1_based: usize,
        primary: u16,
        owner: u8,
        gate: u16,
        follow_on: u16,
    ) -> Result<(), GameStateMutationError> {
        let record = self.ipbm.records.get_mut(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingIpbmRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        record.set_primary_word_raw(primary);
        record.set_owner_empire_raw(owner);
        record.set_gate_word_raw(gate);
        record.set_follow_on_word_raw(follow_on);
        Ok(())
    }

    pub fn fleet_order_errors_current_known(
        &self,
        record_index_1_based: usize,
        speed: u8,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        match self.fleets.records.get(record_index_1_based - 1) {
            Some(record) => {
                if record.current_speed() != speed {
                    errors.push(format!(
                        "FLEET[{}].current_speed expected {}, got {}",
                        record_index_1_based,
                        speed,
                        record.current_speed()
                    ));
                }
                if record.standing_order_code_raw() != order_code {
                    errors.push(format!(
                        "FLEET[{}].order expected {:#04x}, got {:#04x}",
                        record_index_1_based,
                        order_code,
                        record.standing_order_code_raw()
                    ));
                }
                if record.standing_order_target_coords_raw() != target {
                    errors.push(format!(
                        "FLEET[{}].target expected ({}, {}), got {:?}",
                        record_index_1_based,
                        target[0],
                        target[1],
                        record.standing_order_target_coords_raw()
                    ));
                }
                let mission_aux = record.mission_aux_bytes();
                if let Some(value) = aux0 {
                    if mission_aux[0] != value {
                        errors.push(format!(
                            "FLEET[{}].aux0 expected {:#04x}, got {:#04x}",
                            record_index_1_based, value, mission_aux[0]
                        ));
                    }
                }
                if let Some(value) = aux1 {
                    if mission_aux[1] != value {
                        errors.push(format!(
                            "FLEET[{}].aux1 expected {:#04x}, got {:#04x}",
                            record_index_1_based, value, mission_aux[1]
                        ));
                    }
                }
            }
            None => errors.push(format!("FLEETS.DAT missing record {record_index_1_based}")),
        }
        errors
    }

    pub fn planet_build_errors_current_known(
        &self,
        record_index_1_based: usize,
        slot_raw: u8,
        kind_raw: u8,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        match self.planets.records.get(record_index_1_based - 1) {
            Some(record) => {
                if record.build_count_raw(0) != slot_raw {
                    errors.push(format!(
                        "PLANET[{}].build_slot expected {:#04x}, got {:#04x}",
                        record_index_1_based,
                        slot_raw,
                        record.build_count_raw(0)
                    ));
                }
                if record.build_kind_raw(0) != kind_raw {
                    errors.push(format!(
                        "PLANET[{}].build_kind expected {:#04x}, got {:#04x}",
                        record_index_1_based,
                        kind_raw,
                        record.build_kind_raw(0)
                    ));
                }
            }
            None => errors.push(format!("PLANETS.DAT missing record {record_index_1_based}")),
        }
        errors
    }

    pub fn guard_starbase_onebase_errors_current_known(&self) -> Vec<String> {
        let mut errors = Vec::new();

        match self.player.records.first() {
            Some(record) if record.starbase_count_raw() == 1 => {}
            Some(record) => errors.push(format!(
                "PLAYER[1].starbase_count_raw expected 1, got {}",
                record.starbase_count_raw()
            )),
            None => errors.push("PLAYER.DAT missing record 1".to_string()),
        }

        match self.fleets.records.first() {
            Some(record) => {
                if record.standing_order_code_raw() != 0x04 {
                    errors.push(format!(
                        "FLEET[1].order expected 0x04, got {:#04x}",
                        record.standing_order_code_raw()
                    ));
                }
                if record.guard_starbase_enable_raw() != 0x01 {
                    errors.push(format!(
                        "FLEET[1].guard enable expected 0x01, got {:#04x}",
                        record.guard_starbase_enable_raw()
                    ));
                }
                if record.guard_starbase_index_raw() == 0 {
                    errors.push("FLEET[1].guard starbase index expected non-zero".to_string());
                }
            }
            None => errors.push("FLEETS.DAT missing record 1".to_string()),
        }

        let Some(fleet) = self.fleets.records.first() else {
            return errors;
        };
        let Some(player1) = self.player.records.first() else {
            return errors;
        };

        if self.bases.records.len() != 1 {
            errors.push(format!(
                "BASES.DAT expected 1 record, got {}",
                self.bases.records.len()
            ));
        } else {
            let base = &self.bases.records[0];
            if base.local_slot_raw() == 0 {
                errors.push("BASES[1].local_slot expected non-zero".to_string());
            }
            if base.summary_word_raw() != fleet.local_slot_word_raw() {
                errors.push(format!(
                    "BASES[1].summary_word expected FLEET[1].local_slot_word {}, got {}",
                    fleet.local_slot_word_raw(),
                    base.summary_word_raw()
                ));
            }
            if base.base_id_raw() != fleet.guard_starbase_index_raw() {
                errors.push(format!(
                    "BASES[1].base_id expected FLEET[1].guard index {}, got {}",
                    fleet.guard_starbase_index_raw(),
                    base.base_id_raw()
                ));
            }
            if base.coords_raw() != fleet.standing_order_target_coords_raw() {
                errors.push(format!(
                    "BASES[1].coords expected {:?}, got {:?}",
                    fleet.standing_order_target_coords_raw(),
                    base.coords_raw()
                ));
            }
            if base.trailing_coords_raw() != base.coords_raw() {
                errors.push(format!(
                    "BASES[1].trailing coords expected {:?}, got {:?}",
                    base.coords_raw(),
                    base.trailing_coords_raw()
                ));
            }
            if base.chain_word_raw() != player1.starbase_count_raw() {
                errors.push(format!(
                    "BASES[1].chain_word expected PLAYER[1].starbase_count_raw {}, got {}",
                    player1.starbase_count_raw(),
                    base.chain_word_raw()
                ));
            }
            if fleet.local_slot_word_raw() != player1.starbase_count_raw() {
                errors.push(format!(
                    "FLEET[1].local slot word expected PLAYER[1].starbase_count_raw {}, got {}",
                    player1.starbase_count_raw(),
                    fleet.local_slot_word_raw()
                ));
            }
            if fleet.fleet_id_word_raw() != base.chain_word_raw() {
                errors.push(format!(
                    "FLEET[1].fleet ID word expected BASES[1].chain_word {}, got {}",
                    base.chain_word_raw(),
                    fleet.fleet_id_word_raw()
                ));
            }
            if base.tuple_a_payload_raw() != fleet.tuple_a_payload_raw() {
                errors.push(format!(
                    "BASES[1].tuple_a_payload expected FLEET[1].tuple_a_payload {:?}, got {:?}",
                    fleet.tuple_a_payload_raw(),
                    base.tuple_a_payload_raw()
                ));
            }
            if base.tuple_b_payload_raw() != fleet.tuple_b_payload_raw() {
                errors.push(format!(
                    "BASES[1].tuple_b_payload expected FLEET[1].tuple_b_payload {:?}, got {:?}",
                    fleet.tuple_b_payload_raw(),
                    base.tuple_b_payload_raw()
                ));
            }
            if base.tuple_c_payload_raw() != fleet.tuple_c_payload_raw() {
                errors.push(format!(
                    "BASES[1].tuple_c_payload expected FLEET[1].tuple_c_payload {:?}, got {:?}",
                    fleet.tuple_c_payload_raw(),
                    base.tuple_c_payload_raw()
                ));
            }
        }

        errors
    }

    pub fn guard_starbase_linkage_summary_current_known(
        &self,
        player_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
    ) -> Result<CurrentKnownGuardStarbaseLinkageSummary, GameStateMutationError> {
        let player = self
            .player
            .records
            .get(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        let selected_base = fleet
            .guard_starbase_index_raw()
            .checked_sub(1)
            .and_then(|idx| self.bases.records.get(idx as usize));

        Ok(CurrentKnownGuardStarbaseLinkageSummary {
            player_record_index_1_based,
            fleet_record_index_1_based,
            player_starbase_count: player.starbase_count_raw(),
            fleet_order: fleet.standing_order_code_raw(),
            fleet_local_slot: fleet.local_slot_word_raw(),
            fleet_id: fleet.fleet_id_word_raw(),
            guard_index: fleet.guard_starbase_index_raw(),
            guard_enable: fleet.guard_starbase_enable_raw(),
            target_coords: fleet.standing_order_target_coords_raw(),
            selected_base_present: selected_base.is_some(),
            selected_base_summary_word: selected_base.map(|base| base.summary_word_raw()),
            selected_base_id: selected_base.map(|base| base.base_id_raw()),
            selected_base_chain_word: selected_base.map(|base| base.chain_word_raw()),
            selected_base_coords: selected_base.map(|base| base.coords_raw()),
            selected_base_trailing_coords: selected_base.map(|base| base.trailing_coords_raw()),
            selected_base_owner_empire: selected_base.map(|base| base.owner_empire_raw()),
        })
    }

    pub fn guard_starbase_linkage_errors_current_known(
        &self,
        player_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
    ) -> Vec<String> {
        let mut errors = Vec::new();

        let summary = match self.guard_starbase_linkage_summary_current_known(
            player_record_index_1_based,
            fleet_record_index_1_based,
        ) {
            Ok(summary) => summary,
            Err(GameStateMutationError::MissingPlayerRecord { index_1_based }) => {
                errors.push(format!("PLAYER.DAT missing record {index_1_based}"));
                return errors;
            }
            Err(GameStateMutationError::MissingFleetRecord { index_1_based }) => {
                errors.push(format!("FLEETS.DAT missing record {index_1_based}"));
                return errors;
            }
            Err(other) => {
                errors.push(other.to_string());
                return errors;
            }
        };

        if summary.fleet_order != 0x04 {
            errors.push(format!(
                "FLEET[{}].order expected 0x04, got {:#04x}",
                fleet_record_index_1_based, summary.fleet_order
            ));
        }
        if summary.guard_enable != 0x01 {
            errors.push(format!(
                "FLEET[{}].guard enable expected 0x01, got {:#04x}",
                fleet_record_index_1_based, summary.guard_enable
            ));
        }
        if summary.guard_index == 0 {
            errors.push(format!(
                "FLEET[{}].guard starbase index expected non-zero",
                fleet_record_index_1_based
            ));
            return errors;
        }
        if summary.player_starbase_count == 0 {
            errors.push(format!(
                "PLAYER[{}].starbase_count_raw expected non-zero, got 0",
                player_record_index_1_based
            ));
        }
        if summary.guard_index as u16 > summary.player_starbase_count {
            errors.push(format!(
                "FLEET[{}].guard index {} exceeds PLAYER[{}].starbase_count_raw {}",
                fleet_record_index_1_based,
                summary.guard_index,
                player_record_index_1_based,
                summary.player_starbase_count
            ));
        }
        if !summary.selected_base_present {
            errors.push(format!(
                "BASES.DAT missing selected starbase record {}",
                summary.guard_index
            ));
            return errors;
        }

        if summary.selected_base_id != Some(summary.guard_index) {
            errors.push(format!(
                "BASES[{}].base_id expected FLEET[{}].guard index {}, got {:?}",
                summary.guard_index,
                fleet_record_index_1_based,
                summary.guard_index,
                summary.selected_base_id
            ));
        }
        if summary.selected_base_summary_word != Some(summary.fleet_local_slot) {
            errors.push(format!(
                "BASES[{}].summary_word expected FLEET[{}].local_slot_word {}, got {:?}",
                summary.guard_index,
                fleet_record_index_1_based,
                summary.fleet_local_slot,
                summary.selected_base_summary_word
            ));
        }
        if summary.selected_base_coords != Some(summary.target_coords) {
            errors.push(format!(
                "BASES[{}].coords expected FLEET[{}].target {:?}, got {:?}",
                summary.guard_index,
                fleet_record_index_1_based,
                summary.target_coords,
                summary.selected_base_coords
            ));
        }
        if summary.selected_base_trailing_coords != summary.selected_base_coords {
            errors.push(format!(
                "BASES[{}].trailing coords expected {:?}, got {:?}",
                summary.guard_index,
                summary.selected_base_coords.unwrap_or([0, 0]),
                summary.selected_base_trailing_coords
            ));
        }
        let expected_owner_empire = player_record_index_1_based as u8;
        if summary.selected_base_owner_empire != Some(expected_owner_empire) {
            errors.push(format!(
                "BASES[{}].owner_empire expected {}, got {:?}",
                summary.guard_index, expected_owner_empire, summary.selected_base_owner_empire
            ));
        }

        errors
    }

    pub fn guarding_fleet_record_indexes_current_known(&self) -> Vec<usize> {
        self.fleets
            .records
            .iter()
            .enumerate()
            .filter_map(|(idx, fleet)| (fleet.standing_order_code_raw() == 0x04).then_some(idx + 1))
            .collect()
    }

    pub fn guard_starbase_linkage_summaries_for_guarding_fleets_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> Vec<CurrentKnownGuardStarbaseLinkageSummary> {
        self.guarding_fleet_record_indexes_current_known()
            .into_iter()
            .filter_map(|fleet_record_index_1_based| {
                self.guard_starbase_linkage_summary_current_known(
                    player_record_index_1_based,
                    fleet_record_index_1_based,
                )
                .ok()
            })
            .collect()
    }

    pub fn guard_starbase_linkage_errors_for_guarding_fleets_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> Vec<String> {
        let guarding_fleets = self.guarding_fleet_record_indexes_current_known();
        if guarding_fleets.is_empty() {
            return vec!["no guarding fleets found".to_string()];
        }

        let mut errors = Vec::new();
        for fleet_record_index_1_based in guarding_fleets {
            errors.extend(self.guard_starbase_linkage_errors_current_known(
                player_record_index_1_based,
                fleet_record_index_1_based,
            ));
        }
        errors
    }

    pub fn ipbm_count_length_errors_current_known(&self) -> Vec<String> {
        let expected_count = self.player1_ipbm_count_current_known();
        let actual_count = self.ipbm.records.len();
        let expected_size = expected_count * crate::IPBM_RECORD_SIZE;
        let actual_size = self.ipbm.to_bytes().len();

        let mut errors = Vec::new();
        if actual_count != expected_count {
            errors.push(format!(
                "IPBM record count expected {}, got {}",
                expected_count, actual_count
            ));
        }
        if actual_size != expected_size {
            errors.push(format!(
                "IPBM.DAT size expected {}, got {}",
                expected_size, actual_size
            ));
        }
        errors
    }

    pub fn current_known_compliance_status(&self) -> CurrentKnownComplianceStatus {
        CurrentKnownComplianceStatus {
            fleet_order: self
                .fleet_order_errors_current_known(1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
                .is_empty(),
            planet_build: self
                .planet_build_errors_current_known(15, 0x03, 0x01)
                .is_empty(),
            guard_starbase: self
                .guard_starbase_linkage_errors_for_guarding_fleets_current_known(1)
                .is_empty(),
            ipbm: self.ipbm_count_length_errors_current_known().is_empty(),
        }
    }

    pub fn current_known_key_word_summary(&self) -> CurrentKnownKeyWordSummary {
        let player1 = self.player.records.first();
        let fleet1 = self.fleets.records.first();
        let base1 = self.bases.records.first();

        CurrentKnownKeyWordSummary {
            player_starbase_count: player1
                .map(|record| record.starbase_count_raw())
                .unwrap_or(0),
            player_ipbm_count: player1.map(|record| record.ipbm_count_raw()).unwrap_or(0),
            fleet1_local_slot: fleet1.map(|record| record.local_slot_word_raw()),
            fleet1_id: fleet1.map(|record| record.fleet_id_word_raw()),
            fleet1_guard_index: fleet1.map(|record| record.guard_starbase_index_raw()),
            fleet1_guard_enable: fleet1.map(|record| record.guard_starbase_enable_raw()),
            fleet1_target: fleet1.map(|record| record.standing_order_target_coords_raw()),
            base1_summary: base1.map(|record| record.summary_word_raw()),
            base1_id: base1.map(|record| record.base_id_raw()),
            base1_chain: base1.map(|record| record.chain_word_raw()),
            base1_coords: base1.map(|record| record.coords_raw()),
            ipbm_record_count: self.ipbm.records.len(),
        }
    }

    pub fn current_known_baseline_diff_counts(&self) -> Vec<CoreFileDiffCount> {
        let mut normalized = self.clone();
        normalized.sync_current_known_initialized_post_maint_baseline();
        self.diff_counts_against(&normalized)
    }

    pub fn current_known_baseline_diff_offsets(&self) -> Vec<CoreFileDiffOffsets> {
        let mut normalized = self.clone();
        normalized.sync_current_known_initialized_post_maint_baseline();
        self.diff_offsets_against(&normalized)
    }

    pub fn diff_counts_against(&self, other: &Self) -> Vec<CoreFileDiffCount> {
        [
            (
                "PLAYER.DAT",
                self.player.to_bytes(),
                other.player.to_bytes(),
            ),
            (
                "PLANETS.DAT",
                self.planets.to_bytes(),
                other.planets.to_bytes(),
            ),
            (
                "FLEETS.DAT",
                self.fleets.to_bytes(),
                other.fleets.to_bytes(),
            ),
            ("BASES.DAT", self.bases.to_bytes(), other.bases.to_bytes()),
            ("IPBM.DAT", self.ipbm.to_bytes(), other.ipbm.to_bytes()),
            ("SETUP.DAT", self.setup.to_bytes(), other.setup.to_bytes()),
            (
                "CONQUEST.DAT",
                self.conquest.to_bytes(),
                other.conquest.to_bytes(),
            ),
        ]
        .into_iter()
        .map(|(name, current, other)| CoreFileDiffCount {
            name,
            differing_bytes: byte_diff_count(&current, &other),
        })
        .collect()
    }

    pub fn diff_offsets_against(&self, other: &Self) -> Vec<CoreFileDiffOffsets> {
        [
            (
                "PLAYER.DAT",
                self.player.to_bytes(),
                other.player.to_bytes(),
            ),
            (
                "PLANETS.DAT",
                self.planets.to_bytes(),
                other.planets.to_bytes(),
            ),
            (
                "FLEETS.DAT",
                self.fleets.to_bytes(),
                other.fleets.to_bytes(),
            ),
            ("BASES.DAT", self.bases.to_bytes(), other.bases.to_bytes()),
            ("IPBM.DAT", self.ipbm.to_bytes(), other.ipbm.to_bytes()),
            ("SETUP.DAT", self.setup.to_bytes(), other.setup.to_bytes()),
            (
                "CONQUEST.DAT",
                self.conquest.to_bytes(),
                other.conquest.to_bytes(),
            ),
        ]
        .into_iter()
        .map(|(name, current, other)| CoreFileDiffOffsets {
            name,
            differing_offsets: byte_diff_offsets(&current, &other),
        })
        .collect()
    }

    pub fn exact_match_errors_against(&self, other: &Self, label: &str) -> Vec<String> {
        self.diff_counts_against(other)
            .into_iter()
            .filter(|diff| diff.differing_bytes != 0)
            .map(|diff| {
                format!(
                    "{} differs by {} bytes from {}",
                    diff.name, diff.differing_bytes, label
                )
            })
            .collect()
    }

    pub fn set_player_tax_rate(
        &mut self,
        player_record_index_1_based: usize,
        tax_rate: u8,
    ) -> Result<(), GameStateMutationError> {
        let record = self
            .player
            .records
            .get_mut(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        record.set_tax_rate_raw(tax_rate);
        Ok(())
    }

    /// Returns the stored diplomatic relation from one empire toward another
    /// when the backing `PLAYER.DAT` bytes are mapped. At present that field
    /// mapping is still unresolved, so callers generally receive `None` and
    /// must rely on documented manual hostility triggers instead.
    pub fn stored_diplomatic_relation(
        &self,
        from_empire_raw: u8,
        to_empire_raw: u8,
    ) -> Option<DiplomaticRelation> {
        if from_empire_raw == 0 || to_empire_raw == 0 || from_empire_raw == to_empire_raw {
            return None;
        }
        self.player
            .records
            .get(from_empire_raw.saturating_sub(1) as usize)
            .and_then(|record| record.diplomatic_relation_toward(to_empire_raw))
    }

    pub fn set_stored_diplomatic_relation(
        &mut self,
        from_empire_raw: u8,
        to_empire_raw: u8,
        relation: DiplomaticRelation,
    ) -> Result<bool, GameStateMutationError> {
        if from_empire_raw == 0 || to_empire_raw == 0 || from_empire_raw == to_empire_raw {
            return Ok(false);
        }
        let Some(record) = self
            .player
            .records
            .get_mut(from_empire_raw.saturating_sub(1) as usize)
        else {
            return Err(GameStateMutationError::MissingPlayerRecord {
                index_1_based: from_empire_raw as usize,
            });
        };
        Ok(record.set_diplomatic_relation_toward(to_empire_raw, relation))
    }
}

fn load_parsed<T>(
    dir: &Path,
    file_name: &'static str,
    parse: impl Fn(&[u8]) -> Result<T, ParseError>,
) -> Result<T, GameDirectoryError> {
    let path = dir.join(file_name);
    let bytes = fs::read(&path).map_err(|source| GameDirectoryError::Io {
        path: path.clone(),
        source,
    })?;
    parse(&bytes).map_err(|source| GameDirectoryError::Parse { path, source })
}

fn save_bytes(dir: &Path, file_name: &'static str, bytes: &[u8]) -> Result<(), GameDirectoryError> {
    let path = dir.join(file_name);
    fs::write(&path, bytes).map_err(|source| GameDirectoryError::Io { path, source })
}

fn byte_diff_count(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .filter(|(a, b)| a != b)
        .count()
        + left.len().abs_diff(right.len())
}

fn byte_diff_offsets(left: &[u8], right: &[u8]) -> Vec<usize> {
    let shared_len = left.len().min(right.len());
    let mut offsets: Vec<usize> = left[..shared_len]
        .iter()
        .zip(right[..shared_len].iter())
        .enumerate()
        .filter_map(|(idx, (a, b))| (a != b).then_some(idx))
        .collect();

    let extra_len = left.len().max(right.len());
    offsets.extend(shared_len..extra_len);
    offsets
}

impl CoreGameData {
    /// Comprehensive ECMAINT preflight validation.
    ///
    /// This aggregates all cross-file integrity rules that ECMAINT checks at 2000:5EE4.
    /// Returns empty Vec if the gamestate would pass ECMAINT integrity checks.
    pub fn ecmaint_preflight_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // CONQUEST header: year in valid range, player_count consistent
        errors.extend(self.conquest_header_errors());

        // SETUP header: version tag check
        errors.extend(self.setup_header_errors());

        // PLAYER starbase_count ↔ BASES.DAT linkage
        errors.extend(self.player_starbase_bases_linkage_errors());

        // PLAYER ipbm_count ↔ IPBM.DAT length
        errors.extend(self.ipbm_count_length_errors_current_known());

        // Player/planet table lengths
        errors.extend(self.record_count_errors());

        // Fleet owner validation
        errors.extend(self.fleet_owner_errors());

        // Fleet block structure (for initialized scenarios)
        errors.extend(self.current_known_initialized_fleet_block_errors());

        // Planet owner bounds
        errors.extend(self.current_known_planet_owner_slot_errors());

        // Base owner bounds
        errors.extend(self.current_known_base_owner_empire_errors());

        // Base link word validity
        errors.extend(self.base_link_word_errors());

        errors
    }

    /// Validate CONQUEST.DAT header fields.
    /// ECMAINT checks: year in valid range, player_count plausible
    fn conquest_header_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let year = self.conquest.game_year();

        // Year should be in a reasonable range (3000-3100 based on game context)
        if year < 3000 || year > 3100 {
            errors.push(format!(
                "CONQUEST.DAT.game_year {} out of expected range (3000-3100)",
                year
            ));
        }

        let player_count = self.conquest.player_count();
        if player_count == 0 || player_count > 25 {
            errors.push(format!(
                "CONQUEST.DAT.player_count {} out of range (1-25)",
                player_count
            ));
        }

        errors
    }

    fn record_count_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_planets = player_count.saturating_mul(5);

        if self.player.records.len() != player_count {
            errors.push(format!(
                "PLAYER.DAT record count expected {}, got {}",
                player_count,
                self.player.records.len()
            ));
        }
        if self.planets.records.len() != expected_planets {
            errors.push(format!(
                "PLANETS.DAT record count expected {}, got {}",
                expected_planets,
                self.planets.records.len()
            ));
        }

        errors
    }

    /// Validate SETUP.DAT header.
    /// ECMAINT checks: version tag matches expected
    fn setup_header_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.setup.version_tag() != b"EC151" {
            errors.push(format!(
                "SETUP.DAT.version_tag expected EC151, got {:?}",
                String::from_utf8_lossy(self.setup.version_tag())
            ));
        }

        errors
    }

    /// Validate PLAYER starbase_count matches BASES.DAT records.
    /// ECMAINT at 2000:5EE4: PLAYER[0x44] used as base record selector
    fn player_starbase_bases_linkage_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (player_idx, player) in self.player.records.iter().enumerate() {
            let expected_count = player.starbase_count_raw() as usize;
            // Count actual bases owned by this player
            let actual_count = self
                .bases
                .records
                .iter()
                .filter(|b| b.owner_empire_raw() == (player_idx + 1) as u8)
                .count();

            if actual_count != expected_count {
                errors.push(format!(
                    "PLAYER[{}].starbase_count ({}) doesn't match owned BASES records ({})",
                    player_idx + 1,
                    expected_count,
                    actual_count
                ));
            }
        }

        errors
    }

    /// Validate fleet owner bytes match expected player indices.
    /// ECMAINT at 2000:6040..6368: validates fleet owner bytes against player index
    fn fleet_owner_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // For each fleet block (player_count * 4 records)
        let player_count = self.conquest.player_count() as usize;
        let expected_fleets = player_count * 4;

        for (fleet_idx, fleet) in self.fleets.records.iter().enumerate() {
            let owner = fleet.owner_empire_raw() as usize;

            // Determine expected owner from fleet index
            let expected_owner = if fleet_idx < expected_fleets {
                (fleet_idx / 4) + 1
            } else {
                0 // Extra fleets should have owner 0
            };

            if owner != expected_owner && owner != 0 {
                errors.push(format!(
                    "FLEET[{}].owner_empire expected {} or 0, got {}",
                    fleet_idx, expected_owner, owner
                ));
            }
        }

        errors
    }

    /// Validate BASES.DAT link words (offset 0x05..0x06).
    /// ECMAINT: BASES[0x05..0x06] = 0x0001 or 0x0101 triggers abort
    fn base_link_word_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (base_idx, base) in self.bases.records.iter().enumerate() {
            let link_word = base.link_word_raw();

            // Dangerous patterns that trigger ECMAINT abort
            if link_word == 0x0001 || link_word == 0x0101 {
                errors.push(format!(
                    "BASES[{}].link_word = 0x{:04X} triggers ECMAINT integrity abort",
                    base_idx, link_word
                ));
            }
        }

        errors
    }
}

fn build_guard_starbase_base_record(
    coords: [u8; 2],
    base_id: u8,
    summary_word: u16,
    chain_word: u16,
    owner_empire: u8,
    tuple_a: [u8; 5],
    tuple_b: [u8; 5],
    tuple_c: [u8; 5],
) -> BaseRecord {
    let mut record = BaseRecord::new_zeroed();
    record.set_local_slot_raw(base_id);
    record.set_summary_word_raw(summary_word);
    record.set_base_id_raw(base_id);
    record.set_link_word_raw(0x0000);
    record.set_chain_word_raw(chain_word);
    record.set_coords_raw(coords);
    record.set_tuple_a_payload_raw(tuple_a);
    record.set_tuple_b_payload_raw(tuple_b);
    record.set_tuple_c_payload_raw(tuple_c);
    record.set_trailing_coords_raw(coords);
    record.set_owner_empire_raw(owner_empire);
    record
}
