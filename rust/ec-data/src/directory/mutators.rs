use super::*;
use crate::fleet_motion_state::reset_motion_state_for_new_orders;
use crate::{
    BaseDat, BaseRecord, DiplomaticRelation, FleetRecord, IPBM_RECORD_SIZE, IpbmDat, IpbmRecord,
    Order, PlayerRecord,
};

impl CoreGameData {
    pub fn join_player(
        &mut self,
        player_record_index_1_based: usize,
        empire_name: &str,
    ) -> Result<(), GameStateMutationError> {
        let player = self
            .player
            .records
            .get_mut(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        let tax_rate = player.tax_rate();
        let ipbm_count = player.ipbm_count_raw();
        let homeworld_planet_index_1_based =
            if player.homeworld_planet_index_1_based_raw() as usize != 0 {
                player.homeworld_planet_index_1_based_raw() as usize
            } else {
                self.planets
                    .records
                    .iter()
                    .position(|planet| {
                        planet.owner_empire_slot_raw() as usize == player_record_index_1_based
                            && planet.is_homeworld_seed_ignoring_name()
                    })
                    .map(|idx| idx + 1)
                    .unwrap_or(0)
            };
        player.set_owner_empire_raw(player_record_index_1_based as u8);
        player.set_controlled_empire_name_raw(empire_name);
        player.set_tax_rate_raw(tax_rate);
        player.set_ipbm_count_raw(ipbm_count);
        player.set_autopilot_flag(0);
        player.set_last_run_year_raw(self.conquest.game_year());
        if homeworld_planet_index_1_based != 0 {
            player.set_homeworld_planet_index_1_based_raw(homeworld_planet_index_1_based as u8);
            if let Some(planet) = self
                .planets
                .records
                .get_mut(homeworld_planet_index_1_based - 1)
            {
                let revenue = crate::yearly_tax_revenue(
                    planet.present_production_points().unwrap_or(0),
                    tax_rate,
                );
                planet.set_stored_production_points(revenue);
            }
        }
        Ok(())
    }

    pub fn rename_player_homeworld(
        &mut self,
        player_record_index_1_based: usize,
        homeworld_name: &str,
    ) -> Result<usize, GameStateMutationError> {
        let raw_homeworld_planet_index_1_based =
            self.player
                .records
                .get(player_record_index_1_based - 1)
                .ok_or(GameStateMutationError::MissingPlayerRecord {
                    index_1_based: player_record_index_1_based,
                })?
                .homeworld_planet_index_1_based_raw() as usize;
        let homeworld_planet_index_1_based = if raw_homeworld_planet_index_1_based != 0 {
            raw_homeworld_planet_index_1_based
        } else {
            self.planets
                .records
                .iter()
                .position(|planet| {
                    planet.owner_empire_slot_raw() as usize == player_record_index_1_based
                        && planet.is_homeworld_seed_ignoring_name()
                })
                .map(|idx| idx + 1)
                .ok_or(GameStateMutationError::MissingPlanetRecord { index_1_based: 0 })?
        };
        let planet = self
            .planets
            .records
            .get_mut(homeworld_planet_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: homeworld_planet_index_1_based,
            })?;
        planet.set_planet_name(homeworld_name);
        Ok(homeworld_planet_index_1_based)
    }

    pub fn rename_owned_planet(
        &mut self,
        player_record_index_1_based: usize,
        planet_record_index_1_based: usize,
        planet_name: &str,
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_record_index_1_based as u8;
        let planet = self
            .planets
            .records
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        if planet.owner_empire_slot_raw() != owner_empire {
            return Err(GameStateMutationError::PlanetOwnershipMismatch {
                player_index_1_based: player_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }
        planet.set_planet_name(planet_name);
        Ok(())
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
        self.validate_fleet_order_payload(record_index_1_based, order_code, target, aux0, aux1)
            .map_err(|reason| GameStateMutationError::InvalidFleetOrder {
                fleet_index_1_based: record_index_1_based,
                reason,
            })?;
        let max_speed = self
            .fleets
            .records
            .get(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: record_index_1_based,
            })?
            .max_speed();
        if speed > max_speed {
            return Err(GameStateMutationError::InvalidFleetSpeed {
                fleet_index_1_based: record_index_1_based,
                requested: speed,
                max: max_speed,
            });
        }
        let record = self
            .fleets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: record_index_1_based,
            })?;
        reset_motion_state_for_new_orders(record);
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

    pub fn validate_fleet_order_payload(
        &self,
        record_index_1_based: usize,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<(), FleetOrderValidationError> {
        let Some(fleet) = self.fleets.records.get(record_index_1_based - 1) else {
            return Ok(());
        };
        let order = Order::from_raw(order_code);
        if matches!(order, Order::Unknown(_)) {
            return Err(FleetOrderValidationError::UnknownOrderCode(order_code));
        }
        let owner = fleet.owner_empire_raw();
        let has_combat = fleet_has_combat_ships(fleet);
        let has_scout = fleet.scout_count() > 0;
        let has_etac = fleet.etac_count() > 0;
        let has_loaded_troops =
            fleet.troop_transport_count() > 0 && u16::from(fleet.army_count()) > 0;
        let planet_owner = self
            .planets
            .records
            .iter()
            .find(|planet| planet.coords_raw() == target)
            .map(|planet| planet.owner_empire_slot_raw());

        match order {
            Order::GuardStarbase => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                let base_id = aux0.unwrap_or_else(|| fleet.guard_starbase_index_raw());
                let enabled = aux1.unwrap_or_else(|| fleet.guard_starbase_enable_raw());
                if enabled == 0
                    || base_id == 0
                    || !self.bases.records.iter().any(|base| {
                        base.owner_empire_raw() == owner && base.base_id_raw() == base_id
                    })
                {
                    return Err(FleetOrderValidationError::InvalidGuardStarbase);
                }
            }
            Order::GuardBlockadeWorld => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                if planet_owner.is_none() {
                    return Err(FleetOrderValidationError::MissingPlanetTarget);
                }
            }
            Order::BombardWorld => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                match planet_owner {
                    None => return Err(FleetOrderValidationError::MissingPlanetTarget),
                    Some(owner_empire) if owner_empire == owner => {
                        return Err(FleetOrderValidationError::TargetOwnedByFleetEmpire);
                    }
                    _ => {}
                }
            }
            Order::InvadeWorld => {
                if !has_combat {
                    return Err(FleetOrderValidationError::MissingCombatShips);
                }
                if !has_loaded_troops {
                    return Err(FleetOrderValidationError::MissingLoadedTroopTransports);
                }
                match planet_owner {
                    None => return Err(FleetOrderValidationError::MissingPlanetTarget),
                    Some(owner_empire) if owner_empire == owner => {
                        return Err(FleetOrderValidationError::TargetOwnedByFleetEmpire);
                    }
                    _ => {}
                }
            }
            Order::BlitzWorld => {
                if !has_loaded_troops {
                    return Err(FleetOrderValidationError::MissingLoadedTroopTransports);
                }
                match planet_owner {
                    None => return Err(FleetOrderValidationError::MissingPlanetTarget),
                    Some(owner_empire) if owner_empire == owner => {
                        return Err(FleetOrderValidationError::TargetOwnedByFleetEmpire);
                    }
                    _ => {}
                }
            }
            Order::ViewWorld | Order::ScoutSolarSystem | Order::Salvage => {
                if matches!(order, Order::ScoutSolarSystem) && !has_scout {
                    return Err(FleetOrderValidationError::MissingScoutShip);
                }
                let Some(owner_empire) = planet_owner else {
                    return Err(FleetOrderValidationError::MissingPlanetTarget);
                };
                if matches!(order, Order::Salvage) && owner_empire != owner {
                    return Err(FleetOrderValidationError::TargetNotOwnedByFleetEmpire);
                }
            }
            Order::ScoutSector => {
                if !has_scout {
                    return Err(FleetOrderValidationError::MissingScoutShip);
                }
            }
            Order::ColonizeWorld => {
                if !has_etac {
                    return Err(FleetOrderValidationError::MissingEtac);
                }
                if planet_owner.is_none() {
                    return Err(FleetOrderValidationError::MissingPlanetTarget);
                }
            }
            Order::JoinAnotherFleet => {
                let _ = aux1;
                let host_id = aux0.unwrap_or_else(|| fleet.join_host_fleet_id_raw());
                if host_id == 0
                    || host_id == fleet.fleet_id()
                    || !self
                        .fleets
                        .records
                        .iter()
                        .any(|host| host.fleet_id() == host_id && host.owner_empire_raw() == owner)
                {
                    return Err(FleetOrderValidationError::InvalidJoinHost);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn validate_fleet_player_inputs(
        &self,
        record_index_1_based: usize,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Result<(), FleetPlayerInputValidationError> {
        let Some(fleet) = self.fleets.records.get(record_index_1_based - 1) else {
            return Ok(());
        };
        if fleet.army_count() > fleet.troop_transport_count() {
            return Err(
                FleetPlayerInputValidationError::LoadedArmiesExceedTransportCapacity {
                    loaded_armies: fleet.army_count(),
                    transports: fleet.troop_transport_count(),
                },
            );
        }
        if fleet.current_speed() > fleet.max_speed() {
            return Err(FleetPlayerInputValidationError::SpeedExceedsMaximum {
                speed: fleet.current_speed(),
                max: fleet.max_speed(),
            });
        }
        if !fleet_has_combat_ships(fleet) && fleet.rules_of_engagement() != 0 {
            return Err(
                FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe {
                    roe: fleet.rules_of_engagement(),
                },
            );
        }
        if fleet.rules_of_engagement() > 10 {
            return Err(
                FleetPlayerInputValidationError::RulesOfEngagementOutOfRange {
                    roe: fleet.rules_of_engagement(),
                },
            );
        }
        self.validate_fleet_order_payload(record_index_1_based, order_code, target, aux0, aux1)
            .map_err(FleetPlayerInputValidationError::InvalidOrder)
    }

    pub fn validate_planet_player_inputs(
        &self,
        planet_index_1_based: usize,
    ) -> Result<(), PlanetPlayerInputValidationError> {
        let Some(planet) = self.planets.records.get(planet_index_1_based - 1) else {
            return Ok(());
        };
        for slot in 0..10 {
            let build_count = planet.build_count_raw(slot);
            let build_kind = planet.build_kind_raw(slot);
            if build_count == 0 && build_kind != 0 {
                return Err(PlanetPlayerInputValidationError::MissingBuildCountForKind);
            }
            if build_count != 0 && build_kind == 0 {
                return Err(PlanetPlayerInputValidationError::MissingBuildKindForCount);
            }
            if build_kind != 0
                && matches!(
                    ProductionItemKind::from_raw(build_kind),
                    ProductionItemKind::Unknown(_)
                )
            {
                return Err(PlanetPlayerInputValidationError::InvalidBuildKind(
                    build_kind,
                ));
            }
        }
        for slot in 0..crate::STARDOCK_SLOT_COUNT {
            let stardock_count = planet.stardock_count_raw(slot);
            let stardock_kind = planet.stardock_kind_raw(slot);
            if stardock_count == 0 && stardock_kind != 0 {
                return Err(PlanetPlayerInputValidationError::MissingStardockCountForKind);
            }
            if stardock_count != 0 && stardock_kind == 0 {
                return Err(PlanetPlayerInputValidationError::MissingStardockKindForCount);
            }
            if stardock_kind != 0
                && matches!(
                    ProductionItemKind::from_raw(stardock_kind),
                    ProductionItemKind::Unknown(_)
                )
            {
                return Err(PlanetPlayerInputValidationError::InvalidStardockKind(
                    stardock_kind,
                ));
            }
        }
        Ok(())
    }

    pub fn validate_player_diplomacy_inputs(
        &self,
        player_index_1_based: usize,
    ) -> Vec<PlayerDiplomacyValidationError> {
        let mut errors = Vec::new();
        let Some(player) = self.player.records.get(player_index_1_based - 1) else {
            return errors;
        };
        let empire_raw = player_index_1_based as u8;
        let player_count = self.player.records.len() as u8;
        for target_empire_raw in 1..=player_count {
            let raw = player.raw[0x54 + target_empire_raw as usize - 1];
            if target_empire_raw == empire_raw {
                if raw != 0 {
                    errors.push(PlayerDiplomacyValidationError::SelfTarget { empire_raw });
                }
                continue;
            }
            if raw != 0x00 && raw != 0x01 {
                errors.push(PlayerDiplomacyValidationError::InvalidStoredRelationByte {
                    target_empire_raw,
                    raw,
                });
            }
        }
        errors
    }

    pub fn set_join_fleet_order(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        host_fleet_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        if fleet_index_1_based == host_fleet_index_1_based {
            return Err(GameStateMutationError::InvalidFleetMergeSelection {
                fleet_index_1_based,
                host_fleet_index_1_based,
            });
        }

        let owner_empire = player_index_1_based as u8;
        let host = self
            .fleets
            .records
            .get(host_fleet_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: host_fleet_index_1_based,
            })?;
        let fleet = self.fleets.records.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;

        if fleet.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        if host.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: host_fleet_index_1_based,
            });
        }

        let host_coords = host.current_location_coords_raw();
        let host_fleet_id = host.fleet_id();
        let fleet = self.fleets.records.get_mut(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        reset_motion_state_for_new_orders(fleet);
        fleet.set_standing_order_kind(Order::JoinAnotherFleet);
        fleet.set_standing_order_target_coords_raw(host_coords);
        fleet.set_join_host_fleet_id_raw(host_fleet_id);
        Ok(())
    }

    pub fn set_fleet_rules_of_engagement(
        &mut self,
        player_index_1_based: usize,
        fleet_index_1_based: usize,
        roe: u8,
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let fleet = self.fleets.records.get(fleet_index_1_based - 1).ok_or(
            GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_index_1_based,
            },
        )?;
        if fleet.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            });
        }
        if roe > 10 {
            return Err(GameStateMutationError::InvalidFleetPlayerInput {
                fleet_index_1_based,
                reason: FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { roe },
            });
        }
        if !fleet_has_combat_ships(fleet) && roe != 0 {
            return Err(GameStateMutationError::InvalidFleetPlayerInput {
                fleet_index_1_based,
                reason: FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { roe },
            });
        }

        self.fleets.records[fleet_index_1_based - 1].set_rules_of_engagement(roe);
        Ok(())
    }

    pub fn set_planet_build(
        &mut self,
        record_index_1_based: usize,
        slot_raw: u8,
        kind_raw: u8,
    ) -> Result<(), GameStateMutationError> {
        if kind_raw == 0 {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::MissingBuildKindForCount,
            });
        }
        if matches!(
            ProductionItemKind::from_raw(kind_raw),
            ProductionItemKind::Unknown(_)
        ) {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::InvalidBuildKind(kind_raw),
            });
        }
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

    pub fn clear_planet_build_queue(
        &mut self,
        record_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        for slot in 0..10 {
            record.set_build_count_raw(slot, 0);
            record.set_build_kind_raw(slot, 0);
        }
        Ok(())
    }

    pub fn clear_planet_build_orders_by_kind(
        &mut self,
        record_index_1_based: usize,
        kind: ProductionItemKind,
    ) -> Result<usize, GameStateMutationError> {
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        let mut cleared = 0usize;
        for slot in 0..10 {
            if ProductionItemKind::from_raw(record.build_kind_raw(slot)) == kind {
                record.set_build_count_raw(slot, 0);
                record.set_build_kind_raw(slot, 0);
                cleared += 1;
            }
        }
        Ok(cleared)
    }

    pub fn planet_free_stardock_slots(
        &self,
        record_index_1_based: usize,
    ) -> Result<usize, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;

        let occupied_stardock = (0..crate::STARDOCK_SLOT_COUNT)
            .filter(|&s| record.stardock_kind_raw(s) != 0)
            .count();

        let pending_ship_slots = (0..10)
            .filter(|&s| {
                let count = record.build_count_raw(s);
                let kind = record.build_kind_raw(s);
                if count == 0 || kind == 0 {
                    return false;
                }
                !matches!(
                    ProductionItemKind::from_raw(kind),
                    ProductionItemKind::Army | ProductionItemKind::GroundBattery
                )
            })
            .count();

        let reserved = occupied_stardock + pending_ship_slots;
        Ok(crate::STARDOCK_SLOT_COUNT.saturating_sub(reserved))
    }

    pub fn planet_open_stardock_slots_now(
        &self,
        record_index_1_based: usize,
    ) -> Result<usize, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;

        Ok((0..crate::STARDOCK_SLOT_COUNT)
            .filter(|&s| record.stardock_kind_raw(s) == 0)
            .count())
    }

    pub fn planet_free_army_capacity(
        &self,
        record_index_1_based: usize,
    ) -> Result<u16, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        Ok(u16::from(u8::MAX.saturating_sub(record.army_count_raw())))
    }

    pub fn planet_free_ground_battery_capacity(
        &self,
        record_index_1_based: usize,
    ) -> Result<u16, GameStateMutationError> {
        let record = self.planets.records.get(record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            },
        )?;
        Ok(u16::from(
            u8::MAX.saturating_sub(record.ground_batteries_raw()),
        ))
    }

    pub fn append_planet_build_order(
        &mut self,
        record_index_1_based: usize,
        points_remaining_raw: u8,
        kind_raw: u8,
    ) -> Result<(), GameStateMutationError> {
        if points_remaining_raw == 0 || kind_raw == 0 {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: if kind_raw == 0 {
                    PlanetPlayerInputValidationError::MissingBuildKindForCount
                } else {
                    PlanetPlayerInputValidationError::MissingBuildCountForKind
                },
            });
        }
        if matches!(
            ProductionItemKind::from_raw(kind_raw),
            ProductionItemKind::Unknown(_)
        ) {
            return Err(GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: record_index_1_based,
                reason: PlanetPlayerInputValidationError::InvalidBuildKind(kind_raw),
            });
        }
        let record = self
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: record_index_1_based,
            })?;
        let slot = (0..10)
            .find(|&s| record.build_count_raw(s) == 0 && record.build_kind_raw(s) == 0)
            .ok_or(GameStateMutationError::PlanetBuildQueueFull {
                index_1_based: record_index_1_based,
            })?;
        record.set_build_count_raw(slot, points_remaining_raw);
        record.set_build_kind_raw(slot, kind_raw);
        Ok(())
    }

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

    pub fn detach_ships_to_new_fleet(
        &mut self,
        player_index_1_based: usize,
        donor_fleet_record_index_1_based: usize,
        selection: FleetDetachSelection,
        donor_speed: Option<u8>,
        new_fleet_roe: u8,
    ) -> Result<FleetDetachResult, GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let donor = self
            .fleets
            .records
            .get(donor_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: donor_fleet_record_index_1_based,
            })?;
        if donor.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }
        if selection.total_ships() == 0 {
            return Err(GameStateMutationError::FleetDetachSelectionEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let available_full_transports = donor.army_count();
        let available_empty_transports = donor
            .troop_transport_count()
            .saturating_sub(donor.army_count());
        for (ship_kind, requested, available) in [
            (
                "battleships",
                selection.battleships,
                donor.battleship_count(),
            ),
            ("cruisers", selection.cruisers, donor.cruiser_count()),
            ("destroyers", selection.destroyers, donor.destroyer_count()),
            (
                "full transports",
                selection.full_transports,
                available_full_transports,
            ),
            (
                "empty transports",
                selection.empty_transports,
                available_empty_transports,
            ),
            (
                "scout ships",
                u16::from(selection.scouts),
                u16::from(donor.scout_count()),
            ),
            ("ETAC ships", selection.etacs, donor.etac_count()),
        ] {
            if requested > available {
                return Err(
                    GameStateMutationError::FleetDetachSelectionExceedsAvailable {
                        fleet_index_1_based: donor_fleet_record_index_1_based,
                        ship_kind,
                        requested,
                        available,
                    },
                );
            }
        }

        let remaining_ships = total_starships(donor).saturating_sub(selection.total_ships());
        if remaining_ships == 0 {
            return Err(GameStateMutationError::FleetDetachLeavesFleetEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let mut donor_after = donor.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(selection.destroyers),
        );
        donor_after.set_troop_transport_count(
            donor_after
                .troop_transport_count()
                .saturating_sub(selection.full_transports + selection.empty_transports),
        );
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(selection.full_transports),
        );
        donor_after.set_scout_count(donor_after.scout_count().saturating_sub(selection.scouts));
        donor_after.set_etac_count(donor_after.etac_count().saturating_sub(selection.etacs));
        donor_after.recompute_max_speed_from_composition();
        if donor_after.max_speed() > 0 && donor.current_speed() > donor_after.max_speed() {
            let requested = donor_speed.unwrap_or(donor_after.max_speed());
            if requested == 0 || requested > donor_after.max_speed() {
                return Err(GameStateMutationError::InvalidFleetSpeed {
                    fleet_index_1_based: donor_fleet_record_index_1_based,
                    requested,
                    max: donor_after.max_speed(),
                });
            }
            donor_after.set_current_speed(requested);
        }

        let mut new_fleet = FleetRecord::new_zeroed();
        new_fleet.set_owner_empire_raw(owner_empire);
        new_fleet.set_local_slot_word_raw(next_available_owned_fleet_local_slot(
            &self.fleets.records,
            owner_empire,
        ));
        new_fleet.set_fleet_id_word_raw(next_available_global_fleet_id(&self.fleets.records));
        new_fleet.set_current_location_coords_raw(donor.current_location_coords_raw());
        new_fleet.set_tuple_a_payload_raw(donor.tuple_a_payload_raw());
        new_fleet.set_tuple_b_payload_raw(donor.tuple_b_payload_raw());
        new_fleet.set_tuple_c_payload_raw(donor.tuple_c_payload_raw());
        new_fleet.set_standing_order_kind(Order::HoldPosition);
        new_fleet.set_standing_order_target_coords_raw(donor.current_location_coords_raw());
        new_fleet.set_mission_aux_bytes([0, 0]);
        new_fleet.set_rules_of_engagement(new_fleet_roe.min(10));
        new_fleet.set_battleship_count(selection.battleships);
        new_fleet.set_cruiser_count(selection.cruisers);
        new_fleet.set_destroyer_count(selection.destroyers);
        new_fleet.set_troop_transport_count(selection.full_transports + selection.empty_transports);
        new_fleet.set_army_count(selection.full_transports);
        new_fleet.set_scout_count(selection.scouts);
        new_fleet.set_etac_count(selection.etacs);
        new_fleet.recompute_max_speed_from_composition();
        new_fleet.set_current_speed(0);

        self.fleets.records[donor_fleet_record_index_1_based - 1] = donor_after;
        self.fleets.records.push(new_fleet);
        let new_fleet_record_index_1_based = self.fleets.records.len();

        let player = self
            .player
            .records
            .get_mut(player_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_index_1_based,
            })?;
        rebuild_owner_fleet_chain(&mut self.fleets.records, player, owner_empire);

        Ok(FleetDetachResult {
            donor_fleet_record_index_1_based,
            new_fleet_record_index_1_based,
        })
    }

    pub fn transfer_ships_between_fleets(
        &mut self,
        player_index_1_based: usize,
        donor_fleet_record_index_1_based: usize,
        host_fleet_record_index_1_based: usize,
        selection: FleetDetachSelection,
    ) -> Result<FleetTransferResult, GameStateMutationError> {
        if donor_fleet_record_index_1_based == host_fleet_record_index_1_based {
            return Err(GameStateMutationError::InvalidFleetMergeSelection {
                fleet_index_1_based: donor_fleet_record_index_1_based,
                host_fleet_index_1_based: host_fleet_record_index_1_based,
            });
        }

        let owner_empire = player_index_1_based as u8;
        let donor = self
            .fleets
            .records
            .get(donor_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: donor_fleet_record_index_1_based,
            })?
            .clone();
        let host = self
            .fleets
            .records
            .get(host_fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: host_fleet_record_index_1_based,
            })?
            .clone();

        if donor.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }
        if host.owner_empire_raw() != owner_empire {
            return Err(GameStateMutationError::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based: host_fleet_record_index_1_based,
            });
        }
        if donor.current_location_coords_raw() != host.current_location_coords_raw() {
            return Err(GameStateMutationError::InvalidFleetMergeSelection {
                fleet_index_1_based: donor_fleet_record_index_1_based,
                host_fleet_index_1_based: host_fleet_record_index_1_based,
            });
        }
        if selection.total_ships() == 0 {
            return Err(GameStateMutationError::FleetDetachSelectionEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let available_full_transports = donor.army_count();
        let available_empty_transports = donor
            .troop_transport_count()
            .saturating_sub(donor.army_count());
        for (ship_kind, requested, available) in [
            (
                "battleships",
                selection.battleships,
                donor.battleship_count(),
            ),
            ("cruisers", selection.cruisers, donor.cruiser_count()),
            ("destroyers", selection.destroyers, donor.destroyer_count()),
            (
                "full transports",
                selection.full_transports,
                available_full_transports,
            ),
            (
                "empty transports",
                selection.empty_transports,
                available_empty_transports,
            ),
            (
                "scout ships",
                u16::from(selection.scouts),
                u16::from(donor.scout_count()),
            ),
            ("ETAC ships", selection.etacs, donor.etac_count()),
        ] {
            if requested > available {
                return Err(
                    GameStateMutationError::FleetDetachSelectionExceedsAvailable {
                        fleet_index_1_based: donor_fleet_record_index_1_based,
                        ship_kind,
                        requested,
                        available,
                    },
                );
            }
        }

        let remaining_ships = total_starships(&donor).saturating_sub(selection.total_ships());
        if remaining_ships == 0 {
            return Err(GameStateMutationError::FleetDetachLeavesFleetEmpty {
                fleet_index_1_based: donor_fleet_record_index_1_based,
            });
        }

        let mut donor_after = donor.clone();
        donor_after.set_battleship_count(
            donor_after
                .battleship_count()
                .saturating_sub(selection.battleships),
        );
        donor_after.set_cruiser_count(
            donor_after
                .cruiser_count()
                .saturating_sub(selection.cruisers),
        );
        donor_after.set_destroyer_count(
            donor_after
                .destroyer_count()
                .saturating_sub(selection.destroyers),
        );
        donor_after.set_troop_transport_count(
            donor_after
                .troop_transport_count()
                .saturating_sub(selection.full_transports + selection.empty_transports),
        );
        donor_after.set_army_count(
            donor_after
                .army_count()
                .saturating_sub(selection.full_transports),
        );
        donor_after.set_scout_count(donor_after.scout_count().saturating_sub(selection.scouts));
        donor_after.set_etac_count(donor_after.etac_count().saturating_sub(selection.etacs));
        donor_after.recompute_max_speed_from_composition();
        if donor_after.current_speed() > donor_after.max_speed() {
            donor_after.set_current_speed(donor_after.max_speed());
        }

        let mut host_after = host.clone();
        host_after.set_battleship_count(
            host_after
                .battleship_count()
                .saturating_add(selection.battleships),
        );
        host_after.set_cruiser_count(
            host_after
                .cruiser_count()
                .saturating_add(selection.cruisers),
        );
        host_after.set_destroyer_count(
            host_after
                .destroyer_count()
                .saturating_add(selection.destroyers),
        );
        host_after.set_troop_transport_count(
            host_after
                .troop_transport_count()
                .saturating_add(selection.full_transports + selection.empty_transports),
        );
        host_after.set_army_count(
            host_after
                .army_count()
                .saturating_add(selection.full_transports),
        );
        host_after.set_scout_count(host_after.scout_count().saturating_add(selection.scouts));
        host_after.set_etac_count(host_after.etac_count().saturating_add(selection.etacs));
        host_after.recompute_max_speed_from_composition();
        if host_after.current_speed() > host_after.max_speed() {
            host_after.set_current_speed(host_after.max_speed());
        }

        self.fleets.records[donor_fleet_record_index_1_based - 1] = donor_after;
        self.fleets.records[host_fleet_record_index_1_based - 1] = host_after;

        Ok(FleetTransferResult {
            donor_fleet_record_index_1_based,
            host_fleet_record_index_1_based,
        })
    }

    pub fn load_planet_armies_onto_fleet(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
        qty: u16,
    ) -> Result<(), GameStateMutationError> {
        if qty == 0 {
            return Ok(());
        }
        let owner_empire = player_index_1_based as u8;
        let planet = self
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        if planet.owner_empire_slot_raw() != owner_empire
            || fleet.owner_empire_raw() != owner_empire
            || fleet.current_location_coords_raw() != planet.coords_raw()
        {
            return Err(GameStateMutationError::FleetNotAtPlanet {
                fleet_index_1_based: fleet_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let available_planet_armies = u16::from(planet.army_count_raw());
        if qty > available_planet_armies {
            return Err(GameStateMutationError::PlanetArmyShortage {
                planet_index_1_based: planet_record_index_1_based,
                requested: qty,
                available: available_planet_armies,
            });
        }

        let available_transport_capacity = fleet
            .troop_transport_count()
            .saturating_sub(fleet.army_count());
        if qty > available_transport_capacity {
            return Err(GameStateMutationError::TransportCapacityExceeded {
                fleet_index_1_based: fleet_record_index_1_based,
                requested: qty,
                available: available_transport_capacity,
            });
        }

        let planet = self
            .planets
            .records
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get_mut(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        planet.set_army_count_raw(planet.army_count_raw().saturating_sub(qty as u8));
        fleet.set_army_count(fleet.army_count().saturating_add(qty));
        Ok(())
    }

    pub fn unload_fleet_armies_to_planet(
        &mut self,
        player_index_1_based: usize,
        planet_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
        qty: u16,
    ) -> Result<(), GameStateMutationError> {
        if qty == 0 {
            return Ok(());
        }
        let owner_empire = player_index_1_based as u8;
        let planet = self
            .planets
            .records
            .get(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        if planet.owner_empire_slot_raw() != owner_empire
            || fleet.owner_empire_raw() != owner_empire
            || fleet.current_location_coords_raw() != planet.coords_raw()
        {
            return Err(GameStateMutationError::FleetNotAtPlanet {
                fleet_index_1_based: fleet_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }

        let available_loaded_armies = fleet.army_count();
        if qty > available_loaded_armies {
            return Err(GameStateMutationError::FleetArmyShortage {
                fleet_index_1_based: fleet_record_index_1_based,
                requested: qty,
                available: available_loaded_armies,
            });
        }
        let available_planet_capacity = u16::from(u8::MAX.saturating_sub(planet.army_count_raw()));
        if qty > available_planet_capacity {
            return Err(GameStateMutationError::PlanetArmyCapacityExceeded {
                planet_index_1_based: planet_record_index_1_based,
                requested: qty,
                available: available_planet_capacity,
            });
        }

        let planet = self
            .planets
            .records
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get_mut(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        planet.set_army_count_raw(planet.army_count_raw().saturating_add(qty as u8));
        fleet.set_army_count(fleet.army_count().saturating_sub(qty));
        Ok(())
    }

    pub fn replace_planet_build_queue_with_single_order(
        &mut self,
        record_index_1_based: usize,
        points_remaining_raw: u8,
        kind_raw: u8,
    ) -> Result<(), GameStateMutationError> {
        self.clear_planet_build_queue(record_index_1_based)?;
        self.set_planet_build(record_index_1_based, points_remaining_raw, kind_raw)
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

    pub fn set_player_tax_rate(
        &mut self,
        player_record_index_1_based: usize,
        tax_rate: u8,
    ) -> Result<(), GameStateMutationError> {
        if tax_rate > 100 {
            return Err(GameStateMutationError::InvalidPlayerTaxRate {
                player_index_1_based: player_record_index_1_based,
                tax_rate,
            });
        }
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
        let player_count = self.player.records.len() as u8;
        if to_empire_raw == 0 || to_empire_raw > player_count {
            return Err(GameStateMutationError::InvalidDiplomacyInput {
                player_index_1_based: from_empire_raw as usize,
                reason: PlayerDiplomacyValidationError::TargetOutOfRange {
                    target_empire_raw: to_empire_raw,
                },
            });
        }
        if from_empire_raw == 0 || from_empire_raw > player_count {
            return Err(GameStateMutationError::MissingPlayerRecord {
                index_1_based: from_empire_raw as usize,
            });
        }
        if from_empire_raw == to_empire_raw {
            return Err(GameStateMutationError::InvalidDiplomacyInput {
                player_index_1_based: from_empire_raw as usize,
                reason: PlayerDiplomacyValidationError::SelfTarget {
                    empire_raw: from_empire_raw,
                },
            });
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

fn next_available_owned_fleet_local_slot(records: &[FleetRecord], owner_empire: u8) -> u16 {
    let mut owned_slots = records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == owner_empire)
        .map(FleetRecord::local_slot_word_raw)
        .filter(|slot| *slot != 0)
        .collect::<Vec<_>>();
    owned_slots.sort_unstable();
    owned_slots.dedup();

    let mut next = 1u16;
    for slot in owned_slots {
        if slot == next {
            next = next.saturating_add(1);
        } else if slot > next {
            break;
        }
    }
    next
}

fn next_available_global_fleet_id(records: &[FleetRecord]) -> u16 {
    let mut fleet_ids = records
        .iter()
        .map(FleetRecord::fleet_id_word_raw)
        .filter(|fleet_id| *fleet_id != 0)
        .collect::<Vec<_>>();
    fleet_ids.sort_unstable();
    fleet_ids.dedup();

    let mut next = 1u16;
    for fleet_id in fleet_ids {
        if fleet_id == next {
            next = next.saturating_add(1);
        } else if fleet_id > next {
            break;
        }
    }
    next
}

fn total_starships(record: &FleetRecord) -> u32 {
    u32::from(record.battleship_count())
        + u32::from(record.cruiser_count())
        + u32::from(record.destroyer_count())
        + u32::from(record.troop_transport_count())
        + u32::from(record.scout_count())
        + u32::from(record.etac_count())
}

fn fleet_has_combat_ships(record: &FleetRecord) -> bool {
    record.destroyer_count() > 0 || record.cruiser_count() > 0 || record.battleship_count() > 0
}

fn rebuild_owner_fleet_chain(
    records: &mut [FleetRecord],
    player: &mut PlayerRecord,
    owner_empire: u8,
) {
    let mut owned = records
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner_empire_raw() == owner_empire)
        .map(|(idx, fleet)| (idx, fleet.local_slot_word_raw(), fleet.fleet_id_word_raw()))
        .collect::<Vec<_>>();
    owned.sort_unstable_by_key(|(_, local_slot, _)| *local_slot);

    player.set_fleet_chain_head_raw(owned.first().map(|(_, _, fleet_id)| *fleet_id).unwrap_or(0));

    for (position, (idx, _, _)) in owned.iter().enumerate() {
        let previous_id = position
            .checked_sub(1)
            .and_then(|prev| owned.get(prev))
            .map(|(_, _, fleet_id)| *fleet_id as u8)
            .unwrap_or(0);
        let next_id = owned
            .get(position + 1)
            .map(|(_, _, fleet_id)| *fleet_id)
            .unwrap_or(0);
        records[*idx].set_previous_fleet_id(previous_id);
        records[*idx].set_next_fleet_link_word_raw(next_id);
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
