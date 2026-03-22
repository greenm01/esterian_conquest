use super::support::*;
use super::*;

impl CoreGameData {
    pub fn commission_planet_stardock_slot(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        slot_0_based: usize,
    ) -> Result<CommissionResult, GameStateMutationError> {
        self.commission_planet_stardock_slots(
            player_index_1_based,
            planet_record_index_1_based,
            &[slot_0_based],
        )
    }

    pub fn commission_planet_stardock_slots(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        slot_0_based_list: &[usize],
    ) -> Result<CommissionResult, GameStateMutationError> {
        if slot_0_based_list.is_empty() {
            return Err(GameStateMutationError::InvalidCommissionSelection);
        }

        let owner_empire = player_index_1_based as u8;
        let player = self
            .player
            .records
            .get_mut(player_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_index_1_based,
            })?;
        let planet = self
            .planets
            .records
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        if planet.owner_empire_slot_raw() != owner_empire {
            return Err(GameStateMutationError::PlanetOwnershipMismatch {
                player_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let coords = planet.coords_raw();
        let mut selected = Vec::with_capacity(slot_0_based_list.len());
        for &slot_0_based in slot_0_based_list {
            if slot_0_based >= crate::STARDOCK_SLOT_COUNT
                || planet.stardock_kind_raw(slot_0_based) == 0
                || planet.stardock_count_raw(slot_0_based) == 0
            {
                return Err(GameStateMutationError::EmptyStardockSlot {
                    planet_index_1_based: planet_record_index_1_based,
                    slot_0_based,
                });
            }
            selected.push((
                slot_0_based,
                planet.stardock_kind_raw(slot_0_based),
                planet.stardock_count_raw(slot_0_based),
            ));
        }

        let starbase_count = selected
            .iter()
            .filter(|(_, kind_raw, _)| *kind_raw == 9)
            .count();
        if starbase_count > 1 || (starbase_count == 1 && selected.len() > 1) {
            return Err(GameStateMutationError::InvalidCommissionSelection);
        }

        let result = if starbase_count == 1 {
            let next_base_id = player.starbase_count_raw().saturating_add(1);
            player.set_starbase_count_raw(next_base_id);
            self.bases.records.push(build_guard_starbase_base_record(
                coords,
                next_base_id as u8,
                next_base_id,
                next_base_id,
                owner_empire,
                [0x80, 0, 0, 0, 0],
                [0x80, 0, 0, 0, 0],
                [0x81, 0, 0, 0, 0],
            ));
            CommissionResult::Starbase {
                base_record_index_1_based: self.bases.records.len(),
            }
        } else {
            let mut destroyers = 0u16;
            let mut cruisers = 0u16;
            let mut battleships = 0u16;
            let mut scouts = 0u16;
            let mut transports = 0u16;
            let mut etacs = 0u16;

            for (_, kind_raw, count) in &selected {
                match ProductionItemKind::from_raw(*kind_raw) {
                    ProductionItemKind::Destroyer => {
                        destroyers = destroyers.saturating_add(*count);
                    }
                    ProductionItemKind::Cruiser => cruisers = cruisers.saturating_add(*count),
                    ProductionItemKind::Battleship => {
                        battleships = battleships.saturating_add(*count);
                    }
                    ProductionItemKind::Scout => scouts = scouts.saturating_add(*count),
                    ProductionItemKind::Transport => transports = transports.saturating_add(*count),
                    ProductionItemKind::Etac => etacs = etacs.saturating_add(*count),
                    _ => return Err(GameStateMutationError::InvalidCommissionSelection),
                }
            }

            let fleet_id = next_available_global_fleet_id(&self.fleets.records);
            let local_slot =
                next_available_owned_fleet_local_slot(&self.fleets.records, owner_empire);
            let mut owned_fleets = self
                .fleets
                .records
                .iter()
                .enumerate()
                .filter(|(_, fleet)| fleet.owner_empire_raw() == owner_empire)
                .map(|(idx, fleet)| (idx, fleet.local_slot_word_raw(), fleet.fleet_id_word_raw()))
                .collect::<Vec<_>>();
            owned_fleets.sort_unstable_by_key(|(_, slot, _)| *slot);

            let predecessor = owned_fleets
                .iter()
                .copied()
                .rev()
                .find(|(_, slot, _)| *slot < local_slot);
            let successor = owned_fleets
                .iter()
                .copied()
                .find(|(_, slot, _)| *slot > local_slot);
            let previous_fleet_id = predecessor
                .map(|(_, _, predecessor_fleet_id)| predecessor_fleet_id as u8)
                .unwrap_or(0);
            let next_fleet_id = successor
                .map(|(_, _, successor_fleet_id)| successor_fleet_id)
                .unwrap_or(0);

            if let Some((idx, _, _)) = predecessor {
                self.fleets.records[idx].set_next_fleet_link_word_raw(fleet_id);
            } else {
                player.set_fleet_chain_head_raw(fleet_id);
            }
            if let Some((idx, _, _)) = successor {
                self.fleets.records[idx].set_previous_fleet_id(fleet_id as u8);
            }

            let mut fleet = FleetRecord::new_zeroed();
            fleet.set_local_slot_word_raw(local_slot);
            fleet.set_owner_empire_raw(owner_empire);
            fleet.set_next_fleet_link_word_raw(next_fleet_id);
            fleet.set_fleet_id_word_raw(fleet_id);
            fleet.set_previous_fleet_id(previous_fleet_id);
            fleet.set_current_speed(0);
            fleet.set_current_location_coords_raw(coords);
            fleet.set_tuple_a_payload_raw([0x80, 0, 0, 0, 0]);
            fleet.set_tuple_b_payload_raw([0x80, 0, 0, 0, 0]);
            fleet.set_tuple_c_payload_raw([0x81, 0, 0, 0, 0]);
            fleet.set_standing_order_kind(Order::HoldPosition);
            fleet.set_standing_order_target_coords_raw(coords);
            fleet.set_mission_aux_bytes([0, 0]);
            fleet.set_rules_of_engagement(6);
            fleet.set_destroyer_count(destroyers);
            fleet.set_cruiser_count(cruisers);
            fleet.set_battleship_count(battleships);
            fleet.set_scout_count(scouts.min(u16::from(u8::MAX)) as u8);
            fleet.set_troop_transport_count(transports);
            fleet.set_etac_count(etacs);
            fleet.recompute_max_speed_from_composition();

            self.fleets.records.push(fleet);
            CommissionResult::Fleet {
                fleet_record_index_1_based: self.fleets.records.len(),
            }
        };

        for (slot_0_based, _, _) in selected {
            planet.set_stardock_count_raw(slot_0_based, 0);
            planet.set_stardock_kind_raw(slot_0_based, 0);
        }
        Ok(result)
    }

    pub fn auto_commission_all_stardock_units(
        &mut self,
        player_index_1_based: usize,
    ) -> Result<AutoCommissionSummary, GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let mut summary = AutoCommissionSummary::default();
        let planet_indices: Vec<usize> = self
            .planets
            .records
            .iter()
            .enumerate()
            .filter(|(_, planet)| planet.owner_empire_slot_raw() == owner_empire)
            .map(|(idx, _)| idx + 1)
            .collect();

        for planet_index_1_based in planet_indices {
            let Some(planet) = self.planets.records.get(planet_index_1_based - 1) else {
                return Err(GameStateMutationError::MissingPlanetRecord {
                    index_1_based: planet_index_1_based,
                });
            };
            let mut ship_slots = Vec::new();
            let mut starbase_slots = Vec::new();
            let mut ship_count = 0u32;
            for slot in 0..crate::STARDOCK_SLOT_COUNT {
                let count = u32::from(planet.stardock_count_raw(slot));
                if count == 0 {
                    continue;
                }
                match ProductionItemKind::from_raw(planet.stardock_kind_raw(slot)) {
                    ProductionItemKind::Destroyer
                    | ProductionItemKind::Cruiser
                    | ProductionItemKind::Battleship
                    | ProductionItemKind::Scout
                    | ProductionItemKind::Transport
                    | ProductionItemKind::Etac => {
                        ship_slots.push(slot);
                        ship_count = ship_count.saturating_add(count);
                    }
                    ProductionItemKind::Starbase => starbase_slots.push(slot),
                    _ => {}
                }
            }
            if ship_slots.is_empty() && starbase_slots.is_empty() {
                continue;
            }

            summary.planets_used += 1;

            if !ship_slots.is_empty() {
                match self.commission_planet_stardock_slots(
                    player_index_1_based,
                    planet_index_1_based,
                    &ship_slots,
                )? {
                    CommissionResult::Fleet { .. } => {
                        summary.fleets_created += 1;
                        summary.ships_commissioned =
                            summary.ships_commissioned.saturating_add(ship_count);
                    }
                    CommissionResult::Starbase { .. } => {
                        return Err(GameStateMutationError::InvalidCommissionSelection);
                    }
                }
            }

            for slot in starbase_slots {
                match self.commission_planet_stardock_slot(
                    player_index_1_based,
                    planet_index_1_based,
                    slot,
                )? {
                    CommissionResult::Starbase { .. } => summary.starbases_commissioned += 1,
                    CommissionResult::Fleet { .. } => {
                        return Err(GameStateMutationError::InvalidCommissionSelection);
                    }
                }
            }
        }

        Ok(summary)
    }
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
        reset_motion_state_for_new_orders(fleet);
        fleet.set_standing_order_kind(Order::GuardStarbase);
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
}
