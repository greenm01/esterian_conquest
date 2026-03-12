use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    BaseDat, BaseRecord, ConquestDat, FleetDat, IpbmDat, IpbmRecord, ParseError, PlanetDat,
    PlayerDat, SetupDat, IPBM_RECORD_SIZE,
};

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
    MissingFleetRecord {
        index_1_based: usize,
    },
    MissingIpbmRecord {
        index_1_based: usize,
    },
    MissingPlanetRecord {
        index_1_based: usize,
    },
    MissingPlayerRecord {
        index_1_based: usize,
    },
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

    pub fn player1_ipbm_count_current_known(&self) -> usize {
        self.player
            .records
            .first()
            .map(|record| record.ipbm_count_raw() as usize)
            .unwrap_or(0)
    }

    pub fn current_known_core_state_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let expected_bases = self.player1_starbase_count_current_known();
        let expected_ipbm = self.player1_ipbm_count_current_known();

        if self.bases.records.len() != expected_bases {
            errors.push(format!(
                "BASES.DAT record count expected {}, got {}",
                expected_bases,
                self.bases.records.len()
            ));
        }

        if self.ipbm.records.len() != expected_ipbm {
            errors.push(format!(
                "IPBM.DAT record count expected {}, got {}",
                expected_ipbm,
                self.ipbm.records.len()
            ));
        }

        errors
    }

    pub fn sync_player1_current_known_counts(&mut self) {
        if let Some(player1) = self.player.records.first_mut() {
            player1.set_starbase_count_raw(self.bases.records.len() as u16);
            player1.set_ipbm_count_raw(self.ipbm.records.len() as u16);
        }
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

    pub fn set_guard_starbase_onebase(
        &mut self,
        target: [u8; 2],
    ) -> Result<(), GameStateMutationError> {
        let player1 = self
            .player
            .records
            .first_mut()
            .ok_or(GameStateMutationError::MissingPlayerRecord { index_1_based: 1 })?;
        player1.set_starbase_count_raw(1);

        let fleet1 = self
            .fleets
            .records
            .first_mut()
            .ok_or(GameStateMutationError::MissingFleetRecord { index_1_based: 1 })?;
        fleet1.set_standing_order_code_raw(0x04);
        fleet1.set_standing_order_target_coords_raw(target);
        fleet1.set_mission_aux_bytes([0x01, 0x01]);

        let base_summary_word = fleet1.local_slot_word_raw();
        let base_chain_word = fleet1.fleet_id_word_raw();
        let tuple_a = fleet1.tuple_a_payload_raw();
        let tuple_b = fleet1.tuple_b_payload_raw();
        let tuple_c = fleet1.tuple_c_payload_raw();

        self.bases = BaseDat {
            records: vec![build_guard_starbase_base_record(
                target,
                0x01,
                base_summary_word,
                base_chain_word,
                0x01,
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
        let record = self
            .ipbm
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingIpbmRecord {
                index_1_based: record_index_1_based,
            })?;
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

        let summary =
            match self.guard_starbase_linkage_summary_current_known(
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
                summary.guard_index,
                expected_owner_empire,
                summary.selected_base_owner_empire
            ));
        }

        errors
    }

    pub fn guarding_fleet_record_indexes_current_known(&self) -> Vec<usize> {
        self.fleets
            .records
            .iter()
            .enumerate()
            .filter_map(|(idx, fleet)| {
                (fleet.standing_order_code_raw() == 0x04).then_some(idx + 1)
            })
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
            errors.extend(
                self.guard_starbase_linkage_errors_current_known(
                    player_record_index_1_based,
                    fleet_record_index_1_based,
                ),
            );
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
            player_starbase_count: player1.map(|record| record.starbase_count_raw()).unwrap_or(0),
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
