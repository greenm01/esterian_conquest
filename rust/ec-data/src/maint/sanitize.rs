use crate::{
    CoreGameData, FleetOrderValidationError, FleetPlayerInputValidationError, Order,
    PlanetPlayerInputValidationError, PlayerDiplomacyValidationError, ProductionItemKind,
};
use super::InvalidPlayerStateEvent;

pub(super) fn sanitize_invalid_player_inputs(game_data: &mut CoreGameData) -> Vec<InvalidPlayerStateEvent> {
    let mut events = Vec::new();

    for fleet_idx in 0..game_data.fleets.records.len() {
        let mut rescan = true;
        while rescan {
            rescan = false;
            let fleet = &game_data.fleets.records[fleet_idx];
            let order_code = fleet.standing_order_code_raw();
            let target = fleet.standing_order_target_coords_raw();
            let aux = fleet.mission_aux_bytes();
            let owner_empire_raw = fleet.owner_empire_raw();
            let coords = fleet.current_location_coords_raw();
            if let Err(reason) = game_data.validate_fleet_player_inputs(
                fleet_idx + 1,
                order_code,
                target,
                Some(aux[0]),
                Some(aux[1]),
            ) {
                match reason {
                    FleetPlayerInputValidationError::InvalidOrder(reason) => {
                        let should_sanitize = matches!(
                            reason,
                            FleetOrderValidationError::UnknownOrderCode(_)
                                | FleetOrderValidationError::MissingCombatShips
                                | FleetOrderValidationError::MissingScoutShip
                                | FleetOrderValidationError::MissingEtac
                                | FleetOrderValidationError::MissingLoadedTroopTransports
                                | FleetOrderValidationError::InvalidJoinHost
                                | FleetOrderValidationError::TargetOwnedByFleetEmpire
                        );
                        if !should_sanitize {
                            break;
                        }
                        let fleet = &mut game_data.fleets.records[fleet_idx];
                        fleet.set_standing_order_kind(Order::HoldPosition);
                        fleet.set_current_speed(0);
                        fleet.set_standing_order_target_coords_raw(coords);
                        fleet.set_join_host_fleet_id_raw(0);
                        fleet.set_mission_aux_bytes([0, 0]);
                        events.push(InvalidPlayerStateEvent::FleetMission {
                            fleet_idx,
                            owner_empire_raw,
                            order_code_raw: order_code,
                            coords,
                            reason,
                        });
                    }
                    FleetPlayerInputValidationError::LoadedArmiesExceedTransportCapacity {
                        transports,
                        ..
                    } => {
                        game_data.fleets.records[fleet_idx].set_army_count(transports);
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire_raw,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                    FleetPlayerInputValidationError::SpeedExceedsMaximum { max, .. } => {
                        game_data.fleets.records[fleet_idx].set_current_speed(max);
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire_raw,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                    FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { .. } => {
                        game_data.fleets.records[fleet_idx].set_rules_of_engagement(10);
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire_raw,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                    FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { .. } => {
                        game_data.fleets.records[fleet_idx].set_rules_of_engagement(0);
                        events.push(InvalidPlayerStateEvent::FleetInput {
                            fleet_idx,
                            owner_empire_raw,
                            coords,
                            reason,
                        });
                        rescan = true;
                    }
                }
            }
        }
    }

    for planet_idx in 0..game_data.planets.records.len() {
        let (owner_empire_raw, coords, reason_opt) = {
            let planet = &game_data.planets.records[planet_idx];
            (
                planet.owner_empire_slot_raw(),
                planet.coords_raw(),
                game_data
                    .validate_planet_player_inputs(planet_idx + 1)
                    .err(),
            )
        };
        if let Some(reason) = reason_opt {
            let planet = &mut game_data.planets.records[planet_idx];
            for slot in 0..10 {
                let build_count = planet.build_count_raw(slot);
                let build_kind = planet.build_kind_raw(slot);
                if build_count == 0 && build_kind != 0
                    || build_count != 0
                        && matches!(
                            ProductionItemKind::from_raw(build_kind),
                            ProductionItemKind::Unknown(_)
                        )
                {
                    planet.set_build_count_raw(slot, 0);
                    planet.set_build_kind_raw(slot, 0);
                }
                let stardock_count = planet.stardock_count_raw(slot);
                let stardock_kind = planet.stardock_kind_raw(slot);
                if stardock_count == 0 && stardock_kind != 0
                    || stardock_count != 0
                        && matches!(
                            ProductionItemKind::from_raw(stardock_kind),
                            ProductionItemKind::Unknown(_)
                        )
                {
                    planet.set_stardock_count_raw(slot, 0);
                    planet.set_stardock_kind_raw(slot, 0);
                }
            }
            events.push(InvalidPlayerStateEvent::PlanetInput {
                planet_idx,
                owner_empire_raw,
                coords,
                reason,
            });
        }
    }

    for player_idx in 0..game_data.player.records.len() {
        let tax_rate = game_data.player.records[player_idx].tax_rate();
        if tax_rate > 100 {
            game_data.player.records[player_idx].set_tax_rate_raw(100);
            events.push(InvalidPlayerStateEvent::PlayerTaxRate {
                player_idx,
                owner_empire_raw: (player_idx + 1) as u8,
                tax_rate,
            });
        }
        let player_count = game_data.player.records.len() as u8;
        for target_empire_raw in 1..=player_count {
            let reason = {
                let player = &game_data.player.records[player_idx];
                let raw = player.raw[0x54 + target_empire_raw as usize - 1];
                let empire_raw = (player_idx + 1) as u8;
                if target_empire_raw == empire_raw {
                    (raw != 0).then_some(PlayerDiplomacyValidationError::SelfTarget { empire_raw })
                } else if raw != 0x00 && raw != 0x01 {
                    Some(PlayerDiplomacyValidationError::InvalidStoredRelationByte {
                        target_empire_raw,
                        raw,
                    })
                } else {
                    None
                }
            };
            if let Some(reason) = reason {
                game_data.player.records[player_idx].raw[0x54 + target_empire_raw as usize - 1] =
                    0x00;
                events.push(InvalidPlayerStateEvent::DiplomacyInput {
                    player_idx,
                    owner_empire_raw: (player_idx + 1) as u8,
                    reason,
                });
            }
        }
    }

    events
}
