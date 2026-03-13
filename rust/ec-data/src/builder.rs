use crate::{
    BaseDat, ConquestDat, CoreGameData, DatabaseDat, FleetDat, FleetRecord, GameStateMutationError,
    IpbmDat, IpbmRecord, PlanetDat, PlanetRecord, PlayerDat, PlayerRecord, SetupDat,
};
use std::path::Path;

/// Builder for creating arbitrary ECMAINT-compliant gamestate directories.
///
/// This builder allows constructing gamestates from scratch with configurable
/// parameters, ensuring all cross-file linkage rules are satisfied.
///
/// # Example
/// ```
/// use ec_data::GameStateBuilder;
///
/// let gamestate = GameStateBuilder::new()
///     .with_player_count(4)
///     .with_year(3001)
///     .build_initialized_baseline()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct GameStateBuilder {
    player_count: u8,
    game_year: u16,
    homeworld_coords: Vec<[u8; 2]>,
    fleet_orders: Vec<FleetOrderSpec>,
    planet_builds: Vec<PlanetBuildSpec>,
    guard_starbase_orders: Vec<GuardStarbaseSpec>,
    ipbm_count: u16,
}

/// Canonical manual-faithful 4-player setup parameters for the current Rust
/// initializer tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalFourPlayerSetup {
    pub year: u16,
    pub homeworld_coords: [[u8; 2]; 4],
}

impl Default for CanonicalFourPlayerSetup {
    fn default() -> Self {
        Self {
            year: 3000,
            homeworld_coords: [[16, 13], [30, 6], [2, 25], [26, 26]],
        }
    }
}

/// Specification for a fleet order.
#[derive(Debug, Clone)]
pub struct FleetOrderSpec {
    pub fleet_index_1_based: usize,
    pub speed: u8,
    pub order_code: u8,
    pub target: [u8; 2],
    pub aux: [u8; 2],
}

/// Specification for a planet build order.
#[derive(Debug, Clone)]
pub struct PlanetBuildSpec {
    pub planet_index_1_based: usize,
    pub slot: u8,
    pub kind: u8,
}

/// Specification for a guard starbase order.
#[derive(Debug, Clone)]
pub struct GuardStarbaseSpec {
    pub player_index_1_based: usize,
    pub fleet_index_1_based: usize,
    pub target: [u8; 2],
    pub base_id: u8,
}

impl Default for GameStateBuilder {
    fn default() -> Self {
        Self {
            player_count: 4,
            game_year: 3001,
            homeworld_coords: vec![
                [16, 13], // Player 1 homeworld
                [30, 6],  // Player 2 homeworld
                [2, 25],  // Player 3 homeworld
                [26, 26], // Player 4 homeworld
            ],
            fleet_orders: Vec::new(),
            planet_builds: Vec::new(),
            guard_starbase_orders: Vec::new(),
            ipbm_count: 0,
        }
    }
}

impl GameStateBuilder {
    /// Generate a name buffer for a player's homeworld.
    fn name_buffer_for_player(player_idx: usize) -> [u8; 13] {
        let name = format!("Player {} HW", player_idx + 1);
        let bytes = name.as_bytes();
        let mut buffer = [0u8; 13];
        let len = bytes.len().min(13);
        buffer[..len].copy_from_slice(&bytes[..len]);
        buffer
    }

    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the current canonical manual-faithful 4-player game start.
    ///
    /// This is intentionally separate from the generic compatibility builder.
    pub fn build_canonical_four_player_start(
        setup: CanonicalFourPlayerSetup,
    ) -> Result<CoreGameData, GameStateMutationError> {
        GameStateBuilder::new()
            .with_player_count(4)
            .with_year(setup.year)
            .with_homeworld_coords(setup.homeworld_coords.to_vec())
            .build_initialized_baseline()
    }

    /// Set the number of players (1-4).
    pub fn with_player_count(mut self, count: u8) -> Self {
        self.player_count = count.clamp(1, 4);
        // Adjust homeworld coords to match player count
        while self.homeworld_coords.len() < self.player_count as usize {
            self.homeworld_coords.push([0, 0]);
        }
        self.homeworld_coords.truncate(self.player_count as usize);
        self
    }

    /// Set the game year.
    pub fn with_year(mut self, year: u16) -> Self {
        self.game_year = year;
        self
    }

    /// Set homeworld coordinates for all players.
    pub fn with_homeworld_coords(mut self, coords: Vec<[u8; 2]>) -> Self {
        self.homeworld_coords = coords;
        self.player_count = self.homeworld_coords.len().clamp(1, 4) as u8;
        self
    }

    /// Add a fleet order.
    pub fn with_fleet_order(
        mut self,
        fleet_index_1_based: usize,
        speed: u8,
        order_code: u8,
        target: [u8; 2],
        aux: [u8; 2],
    ) -> Self {
        self.fleet_orders.push(FleetOrderSpec {
            fleet_index_1_based,
            speed,
            order_code,
            target,
            aux,
        });
        self
    }

    /// Add a planet build order.
    pub fn with_planet_build(mut self, planet_index_1_based: usize, slot: u8, kind: u8) -> Self {
        self.planet_builds.push(PlanetBuildSpec {
            planet_index_1_based,
            slot,
            kind,
        });
        self
    }

    /// Add a guard starbase order.
    pub fn with_guard_starbase(
        mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        target: [u8; 2],
        base_id: u8,
    ) -> Self {
        self.guard_starbase_orders.push(GuardStarbaseSpec {
            player_index_1_based,
            fleet_index_1_based,
            target,
            base_id,
        });
        self
    }

    /// Set IPBM count (for all players - currently supports count=0 best).
    pub fn with_ipbm_count(mut self, count: u16) -> Self {
        self.ipbm_count = count;
        self
    }

    /// Build an initialized baseline gamestate.
    ///
    /// This creates a clean post-maint state with:
    /// - Proper fleet blocks for each player
    /// - Homeworld planets configured
    /// - Setup and Conquest headers set
    /// - Empty auxiliary files (BASES.DAT, IPBM.DAT)
    pub fn build_initialized_baseline(&self) -> Result<CoreGameData, GameStateMutationError> {
        // Create zeroed records for fixed-size arrays
        let player_records: [PlayerRecord; 4] = std::array::from_fn(|_| PlayerRecord::new_zeroed());
        let planet_records: [PlanetRecord; 20] =
            std::array::from_fn(|_| PlanetRecord::new_zeroed());

        let mut data = CoreGameData {
            player: PlayerDat {
                records: player_records,
            },
            planets: PlanetDat {
                records: planet_records,
            },
            fleets: FleetDat { records: vec![] },
            bases: BaseDat { records: vec![] },
            ipbm: IpbmDat { records: vec![] },
            setup: SetupDat {
                raw: [0; crate::SETUP_DAT_SIZE],
            },
            conquest: ConquestDat {
                raw: [0; crate::CONQUEST_DAT_SIZE],
            },
        };

        // Configure conquest header
        data.conquest.set_game_year(self.game_year);
        data.conquest.set_player_count(self.player_count);

        // Configure setup header
        data.setup.raw[..5].copy_from_slice(b"EC151");
        data.setup.raw[5..13].copy_from_slice(&[4, 3, 4, 3, 1, 1, 1, 1]); // option_prefix
        data.setup.set_snoop_enabled(true);
        data.setup.set_max_time_between_keys_minutes_raw(10);
        data.setup.set_remote_timeout_enabled(true);
        data.setup.set_local_timeout_enabled(false);

        // Configure player records
        for (idx, player) in data.player.records.iter_mut().enumerate() {
            if idx < self.player_count as usize {
                // Set flag at offset 0 to indicate player is present
                player.set_owner_empire_raw((idx + 1) as u8);
                player.set_tax_rate_raw(0);
                player.set_ipbm_count_raw(self.ipbm_count);
                // Set autopilot based on player index (player 1 = on, others = off)
                player.set_autopilot_flag(if idx == 0 { 1 } else { 0 });
            }
        }

        // Configure homeworld planets
        for (player_idx, coords) in self.homeworld_coords.iter().enumerate() {
            // Use planets 0-3 for players 1-4 homeworlds
            if let Some(planet) = data.planets.records.get_mut(player_idx) {
                planet.set_as_owned_target_world(
                    *coords,
                    [100, 135],            // potential_production (default)
                    [0, 0, 0, 0, 72, 134], // factories (default)
                    0,                     // tax_rate
                    14,                    // name_len ("Player X HW")
                    Self::name_buffer_for_player(player_idx),
                    [0; 7],                 // name_suffix_raw
                    1,                      // army_count
                    1,                      // ground_batteries
                    2,                      // ownership_status
                    (player_idx + 1) as u8, // owner_empire_slot
                );
            }
        }

        // Build fleet blocks
        self.build_fleet_blocks(&mut data)?;

        // Setup empty auxiliary state
        data.bases = BaseDat { records: vec![] };
        if self.ipbm_count > 0 {
            data.ipbm = IpbmDat {
                records: (0..self.ipbm_count)
                    .map(|_| IpbmRecord::new_zeroed())
                    .collect(),
            };
        }

        // Apply any additional orders
        self.apply_orders(&mut data)?;

        Ok(data)
    }

    /// Build fleet blocks for all players.
    fn build_fleet_blocks(&self, data: &mut CoreGameData) -> Result<(), GameStateMutationError> {
        let player_count = self.player_count as usize;
        let expected_fleet_count = player_count * 4;

        let mut records = Vec::with_capacity(expected_fleet_count);

        for block_idx in 0..player_count {
            let coords = self
                .homeworld_coords
                .get(block_idx)
                .copied()
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
                record.set_standing_order_code_raw(5); // Default: no order
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

        data.fleets = FleetDat { records };
        Ok(())
    }

    /// Apply all configured orders to the gamestate.
    fn apply_orders(&self, data: &mut CoreGameData) -> Result<(), GameStateMutationError> {
        // Apply fleet orders
        for order in &self.fleet_orders {
            data.set_fleet_order(
                order.fleet_index_1_based,
                order.speed,
                order.order_code,
                order.target,
                Some(order.aux[0]),
                Some(order.aux[1]),
            )?;
        }

        // Apply planet builds
        for build in &self.planet_builds {
            data.set_planet_build(build.planet_index_1_based, build.slot, build.kind)?;
        }

        // Apply guard starbase orders
        for guard in &self.guard_starbase_orders {
            data.set_guard_starbase(
                guard.player_index_1_based,
                guard.fleet_index_1_based,
                guard.target,
                guard.base_id,
                guard.player_index_1_based as u8,
            )?;
        }

        Ok(())
    }

    /// Build and save a complete gamestate directory.
    ///
    /// This creates all necessary files including generated DATABASE.DAT.
    pub fn build_and_save(&self, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;

        fs::create_dir_all(target)?;

        // Build the gamestate
        let data = self.build_initialized_baseline()?;

        // Save core files
        data.save(target)?;

        // Generate and save DATABASE.DAT
        let planet_names: Vec<String> = data
            .planets
            .records
            .iter()
            .map(|p| p.planet_name())
            .collect();

        // Load template from init fixture (we'll create a default one if needed)
        let database = DatabaseDat::generate_from_planets_and_year(
            &planet_names,
            self.game_year,
            None, // Use default template
        );
        fs::write(target.join("DATABASE.DAT"), database.to_bytes())?;

        // Ensure auxiliary files exist
        for name in ["MESSAGES.DAT", "RESULTS.DAT"] {
            let path = target.join(name);
            if !path.exists() {
                fs::write(path, [])?;
            }
        }

        Ok(())
    }
}
