use super::{
    ColonizationEvent, DiplomaticEscalationEvent, Mission, MissionEvent, MissionOutcome,
    MovementEvents, PlanetIntelEvent, PlanetIntelSource, SalvageFailureReason,
    SalvageResolvedEvent,
};
use crate::navigation::{
    advance_exact_position, plan_route_with_intel, rounded_coords_from_exact,
    visible_hazard_intel_is_empty,
};
use crate::{CoreGameData, Order, ProductionItemKind, VisibleHazardIntel};
use ec_data::fleet_motion_state::{
    decode_exact_position, reset_motion_state_for_new_orders, store_exact_position,
};

fn queue_local_intrusion_escalation(
    movement_events: &mut MovementEvents,
    owner_empire_raw: u8,
    defender_empire_raw: u8,
) {
    if owner_empire_raw != 0 && defender_empire_raw != 0 && owner_empire_raw != defender_empire_raw
    {
        movement_events
            .diplomatic_escalation_events
            .push(DiplomaticEscalationEvent {
                left_empire_raw: owner_empire_raw,
                right_empire_raw: defender_empire_raw,
                stardate_week: None,
            });
    }
}

/// Process fleet movement for all fleets with active movement.
///
/// Based on docs/dev/archive/RE_NOTES.md section "Fleet Movement: Speed and Distance":
/// - Distance per turn = speed / 1.5 (approximately)
/// - Any order kind with speed > 0 and target ≠ current position triggers movement
/// - Coordinates stored at FLEETS.DAT[0x0B..0x0C] (x, y)
///
/// Returns a list of colonization events for fleets that arrived with ColonizeWorld orders.
pub(super) fn process_fleet_movement(
    game_data: &mut CoreGameData,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<MovementEvents, Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    let mut movement_events = MovementEvents::default();
    let mut to_remove = vec![false; fleet_count];

    for i in 0..fleet_count {
        let (target_x, target_y, current_x, current_y, speed, order_kind, owner_empire) = {
            let fleet = &game_data.fleets.records[i];
            (
                fleet.standing_order_target_coords_raw()[0],
                fleet.standing_order_target_coords_raw()[1],
                fleet.current_location_coords_raw()[0],
                fleet.current_location_coords_raw()[1],
                fleet.current_speed(),
                fleet.standing_order_kind(),
                fleet.owner_empire_raw(),
            )
        };
        if matches!(order_kind, Order::Salvage) && target_x == current_x && target_y == current_y {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == [target_x, target_y]);
            queue_salvage_resolution(
                game_data,
                &mut movement_events,
                &mut to_remove,
                i,
                owner_empire,
                planet_idx,
                [target_x, target_y],
            )?;
            continue;
        }
        // A fleet moves when it has a non-HoldPosition order, speed > 0,
        // and hasn't reached its target yet.
        // order_code 0x00 = HoldPosition — fleet stays put even if speed > 0
        // and target != current.
        // Note: BombardWorld/InvadeWorld fleets also move to their target before executing;
        // they are allowed here — arrival handling preserves their order/speed.
        let order_code = game_data.fleets.records[i].standing_order_code_raw();
        let should_move =
            speed > 0 && order_code != 0x00 && (target_x != current_x || target_y != current_y);

        if should_move {
            let arrived = process_single_fleet_movement(game_data, i, visible_hazards_by_empire)?;

            // If a ColonizeWorld fleet arrived, queue a colonization event
            if arrived {
                match order_kind {
                    Order::ColonizeWorld => {
                        movement_events.colonization_events.push(ColonizationEvent {
                            fleet_idx: i,
                            coords: [target_x, target_y],
                            owner_empire,
                        });
                    }
                    Order::ScoutSector => {
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::ScoutSector,
                            outcome: MissionOutcome::Succeeded,
                            planet_idx: None,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::ScoutSolarSystem => {
                        if let Some(planet_idx) = game_data
                            .planets
                            .records
                            .iter()
                            .position(|planet| planet.coords_raw() == [target_x, target_y])
                        {
                            movement_events.planet_intel_events.push(PlanetIntelEvent {
                                planet_idx,
                                viewer_empire_raw: owner_empire,
                                source: PlanetIntelSource::ScoutSolarSystem,
                            });
                        }
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::ScoutSolarSystem,
                            outcome: MissionOutcome::Succeeded,
                            planet_idx: None,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::ViewWorld => {
                        let planet_idx = game_data
                            .planets
                            .records
                            .iter()
                            .position(|planet| planet.coords_raw() == [target_x, target_y]);
                        if let Some(planet_idx) = planet_idx {
                            movement_events.planet_intel_events.push(PlanetIntelEvent {
                                planet_idx,
                                viewer_empire_raw: owner_empire,
                                source: PlanetIntelSource::ViewWorld,
                            });
                        }
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::ViewWorld,
                            outcome: if planet_idx.is_some() {
                                MissionOutcome::Succeeded
                            } else {
                                MissionOutcome::Failed
                            },
                            planet_idx,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                        set_fleet_to_deep_space_hold(&mut game_data.fleets.records[i]);
                    }
                    Order::Salvage => {
                        let planet_idx = game_data
                            .planets
                            .records
                            .iter()
                            .position(|planet| planet.coords_raw() == [target_x, target_y]);
                        queue_salvage_resolution(
                            game_data,
                            &mut movement_events,
                            &mut to_remove,
                            i,
                            owner_empire,
                            planet_idx,
                            [target_x, target_y],
                        )?;
                    }
                    Order::GuardStarbase => {
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::GuardStarbase,
                            outcome: MissionOutcome::Arrived,
                            planet_idx: None,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::GuardBlockadeWorld => {
                        let planet_idx = game_data
                            .planets
                            .records
                            .iter()
                            .position(|planet| planet.coords_raw() == [target_x, target_y]);
                        if let Some(planet_idx) = planet_idx {
                            let defender_empire =
                                game_data.planets.records[planet_idx].owner_empire_slot_raw();
                            queue_local_intrusion_escalation(
                                &mut movement_events,
                                owner_empire,
                                defender_empire,
                            );
                        }
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::GuardBlockadeWorld,
                            outcome: MissionOutcome::Arrived,
                            planet_idx,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::RendezvousSector => {
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::RendezvousSector,
                            outcome: MissionOutcome::Arrived,
                            planet_idx: None,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::MoveOnly => {
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::MoveOnly,
                            outcome: MissionOutcome::Succeeded,
                            planet_idx: None,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::PatrolSector => {
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::PatrolSector,
                            outcome: MissionOutcome::Arrived,
                            planet_idx: None,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::SeekHome => {
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::SeekHome,
                            outcome: MissionOutcome::Succeeded,
                            planet_idx: game_data
                                .planets
                                .records
                                .iter()
                                .position(|planet| planet.coords_raw() == [target_x, target_y]),
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::BombardWorld => {
                        if let Some(planet_idx) = game_data
                            .planets
                            .records
                            .iter()
                            .position(|planet| planet.coords_raw() == [target_x, target_y])
                        {
                            let defender_empire =
                                game_data.planets.records[planet_idx].owner_empire_slot_raw();
                            queue_local_intrusion_escalation(
                                &mut movement_events,
                                owner_empire,
                                defender_empire,
                            );
                        }
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::BombardWorld,
                            outcome: MissionOutcome::Arrived,
                            planet_idx: game_data
                                .planets
                                .records
                                .iter()
                                .position(|planet| planet.coords_raw() == [target_x, target_y]),
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::InvadeWorld => {
                        if let Some(planet_idx) = game_data
                            .planets
                            .records
                            .iter()
                            .position(|planet| planet.coords_raw() == [target_x, target_y])
                        {
                            let defender_empire =
                                game_data.planets.records[planet_idx].owner_empire_slot_raw();
                            queue_local_intrusion_escalation(
                                &mut movement_events,
                                owner_empire,
                                defender_empire,
                            );
                        }
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::InvadeWorld,
                            outcome: MissionOutcome::Arrived,
                            planet_idx: game_data
                                .planets
                                .records
                                .iter()
                                .position(|planet| planet.coords_raw() == [target_x, target_y]),
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    Order::BlitzWorld => {
                        let planet_idx = game_data
                            .planets
                            .records
                            .iter()
                            .position(|planet| planet.coords_raw() == [target_x, target_y]);
                        if let Some(planet_idx) = planet_idx {
                            let defender_empire =
                                game_data.planets.records[planet_idx].owner_empire_slot_raw();
                            queue_local_intrusion_escalation(
                                &mut movement_events,
                                owner_empire,
                                defender_empire,
                            );
                        }
                        movement_events.mission_events.push(MissionEvent {
                            fleet_idx: i,
                            owner_empire_raw: owner_empire,
                            kind: Mission::BlitzWorld,
                            outcome: MissionOutcome::Arrived,
                            planet_idx,
                            location_coords: Some([target_x, target_y]),
                            target_coords: Some([target_x, target_y]),
                            stardate_week: None,
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    if to_remove.iter().any(|remove| *remove) {
        remap_movement_event_fleet_indices_after_removal(&mut movement_events, &to_remove);
        super::remove_selected_fleets(game_data, &to_remove);
    }

    Ok(movement_events)
}

fn queue_salvage_resolution(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
    to_remove: &mut [bool],
    fleet_idx: usize,
    owner_empire_raw: u8,
    planet_idx: Option<usize>,
    coords: [u8; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    let salvage_event =
        resolve_salvage_arrival(game_data, fleet_idx, owner_empire_raw, planet_idx)?;
    match salvage_event {
        SalvageResolvedEvent::Succeeded {
            recovered_points, ..
        } => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw,
                kind: Mission::Salvage,
                outcome: MissionOutcome::Succeeded,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
            movement_events.salvage_events.push(salvage_event);
            if recovered_points > 0 {
                to_remove[fleet_idx] = true;
            }
        }
        SalvageResolvedEvent::Failed { .. } => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw,
                kind: Mission::Salvage,
                outcome: MissionOutcome::Failed,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
            movement_events.salvage_events.push(salvage_event);
        }
    }

    Ok(())
}

fn resolve_salvage_arrival(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    owner_empire_raw: u8,
    planet_idx: Option<usize>,
) -> Result<SalvageResolvedEvent, Box<dyn std::error::Error>> {
    let coords = game_data.fleets.records[fleet_idx].current_location_coords_raw();
    let Some(planet_idx) = planet_idx else {
        let fleet = &mut game_data.fleets.records[fleet_idx];
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(coords);
        reset_motion_state_for_new_orders(fleet);
        return Ok(SalvageResolvedEvent::Failed {
            fleet_idx,
            owner_empire_raw,
            planet_idx: None,
            coords,
            reason: SalvageFailureReason::NoPlanetAtTarget,
            stardate_week: None,
        });
    };

    if game_data.planets.records[planet_idx].owner_empire_slot_raw() != owner_empire_raw {
        let fleet = &mut game_data.fleets.records[fleet_idx];
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(coords);
        reset_motion_state_for_new_orders(fleet);
        return Ok(SalvageResolvedEvent::Failed {
            fleet_idx,
            owner_empire_raw,
            planet_idx: Some(planet_idx),
            coords,
            reason: SalvageFailureReason::PlanetNotOwned,
            stardate_week: None,
        });
    }

    let recovered_points = fleet_salvage_value(&game_data.fleets.records[fleet_idx]);
    let current_stored = game_data.planets.records[planet_idx].stored_production_points();
    game_data.planets.records[planet_idx]
        .set_stored_production_points(current_stored.saturating_add(recovered_points));

    Ok(SalvageResolvedEvent::Succeeded {
        fleet_idx,
        owner_empire_raw,
        planet_idx,
        coords,
        recovered_points,
        stardate_week: None,
    })
}

fn set_fleet_to_deep_space_hold(fleet: &mut ec_data::FleetRecord) {
    let coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
    reset_motion_state_for_new_orders(fleet);
}

fn order_preserves_state_on_arrival(order: Order) -> bool {
    matches!(
        order,
        Order::PatrolSector
            | Order::GuardStarbase
            | Order::GuardBlockadeWorld
            | Order::JoinAnotherFleet
            | Order::RendezvousSector
            | Order::BombardWorld
            | Order::InvadeWorld
            | Order::BlitzWorld
    )
}

fn remap_movement_event_fleet_indices_after_removal(
    movement_events: &mut MovementEvents,
    to_remove: &[bool],
) {
    let removed_before: Vec<usize> = {
        let mut removed = 0usize;
        to_remove
            .iter()
            .map(|remove| {
                let current = removed;
                if *remove {
                    removed += 1;
                }
                current
            })
            .collect()
    };

    let remap = |fleet_idx: usize| -> Option<usize> {
        if to_remove.get(fleet_idx).copied().unwrap_or(false) {
            None
        } else {
            Some(fleet_idx.saturating_sub(removed_before.get(fleet_idx).copied().unwrap_or(0)))
        }
    };

    movement_events
        .colonization_events
        .retain_mut(|event| match remap(event.fleet_idx) {
            Some(new_idx) => {
                event.fleet_idx = new_idx;
                true
            }
            None => false,
        });
    movement_events
        .mission_events
        .retain_mut(|event| match remap(event.fleet_idx) {
            Some(new_idx) => {
                event.fleet_idx = new_idx;
                true
            }
            None => false,
        });
}

fn fleet_salvage_value(fleet: &ec_data::FleetRecord) -> u32 {
    let total_cost = u32::from(fleet.destroyer_count())
        * purchase_cost(ProductionItemKind::Destroyer)
        + u32::from(fleet.cruiser_count()) * purchase_cost(ProductionItemKind::Cruiser)
        + u32::from(fleet.battleship_count()) * purchase_cost(ProductionItemKind::Battleship)
        + u32::from(fleet.scout_count()) * purchase_cost(ProductionItemKind::Scout)
        + u32::from(fleet.troop_transport_count()) * purchase_cost(ProductionItemKind::Transport)
        + u32::from(fleet.etac_count()) * purchase_cost(ProductionItemKind::Etac)
        + u32::from(fleet.army_count()) * purchase_cost(ProductionItemKind::Army);
    total_cost / 2
}

fn purchase_cost(kind: ProductionItemKind) -> u32 {
    match kind {
        ProductionItemKind::Destroyer => 5,
        ProductionItemKind::Cruiser => 15,
        ProductionItemKind::Battleship => 45,
        ProductionItemKind::Scout => 15,
        ProductionItemKind::Transport => 5,
        ProductionItemKind::Etac => 20,
        ProductionItemKind::GroundBattery => 20,
        ProductionItemKind::Army => 2,
        ProductionItemKind::Starbase => 50,
        ProductionItemKind::Unknown(_) => 0,
    }
}

/// Process movement for a single fleet using the ECMAINT movement formula.
///
/// Movement formula (confirmed from move-scenario fixture, speed=3, horizontal move):
/// - Uses a sub-grid of 9 sub-units per grid cell.
/// - Each turn: sub_acc += speed * 8; integer_move = sub_acc / 9; sub_acc %= 9.
/// - The fleet advances from its exact in-transit position toward its target
///   by integer_move movement units and only rounds when writing visible
///   sector coordinates.
/// - This is equivalent to distance_per_turn ≈ speed * 8/9.
///
/// The fractional accumulator is persisted in raw[0x0f] between turns.
/// Encoding (confirmed for speed=3): raw[0x0f] as i8 = (sub_acc - 9) * 2 / 3
/// (Generalised to: the sub_acc is always a multiple of 3 for speed=3 with denominator 9.)
///
/// When a fleet starts moving from rest (raw[0x0d] == 0x80):
/// - raw[0x0d] → 0x7f (transit tag byte)
/// - raw[0x0e] → 0xc0 (fixed constant during transit)
/// - raw[0x10..0x12] → [0xff, 0xff, 0x7f] (fixed constants during transit)
/// - raw[0x19] → 0x00 (clear departure flag)
///
/// On arrival (position reaches target):
/// - completion orders clear current_speed and fall back to HoldPosition
/// - persistent standing and delayed hostile orders remain armed
/// - tuple_c_payload set to [0x80, 0xb9, 0xff, 0xff, 0xff]
/// - raw[0x1e] set to 0x7f
///
/// Confirmed from fleet-scenario fixture: fleet 0 ColonizeWorld, speed=3,
/// pos=(16,13) → (15,13) (arrived), all above changes observed.
/// Confirmed from move-scenario fixture: fleet 0 MoveOnly, speed=3,
/// pos=(16,13) → (24,13) after 3 turns, position and 0x0f encoding verified.
///
/// Returns `true` if the fleet arrived at its target this turn.
fn process_single_fleet_movement(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<bool, Box<dyn std::error::Error>> {
    // Get fleet data first, then release the borrow
    let (current_x, current_y, target_x, target_y, speed, is_at_rest, raw_0f, owner_empire_raw) = {
        let fleet = &game_data.fleets.records[fleet_idx];
        (
            fleet.current_location_coords_raw()[0],
            fleet.current_location_coords_raw()[1],
            fleet.standing_order_target_coords_raw()[0],
            fleet.standing_order_target_coords_raw()[1],
            fleet.current_speed(),
            fleet.raw[0x0d] == 0x80, // 0x80 = at rest, 0x7f = in transit
            fleet.raw[0x0f],
            fleet.owner_empire_raw(),
        )
    };

    if speed == 0 {
        return Ok(false);
    }

    let dx_total = target_x as i32 - current_x as i32;
    let dy_total = target_y as i32 - current_y as i32;

    if dx_total == 0 && dy_total == 0 {
        // Already at target - clear speed and order
        let fleet = &mut game_data.fleets.records[fleet_idx];
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        reset_motion_state_for_new_orders(fleet);
        return Ok(true);
    }

    // Reconstruct the fractional sub-accumulator from raw[0x0f].
    // Encoding (confirmed, speed=3): sub_acc = 9 + (raw[0x0f] as i8) * 3 / 2
    // When the fleet is at rest (0x0d == 0x80), sub_acc starts at 0.
    let sub_acc_prev: u32 = if is_at_rest {
        0
    } else {
        // Decode from raw[0x0f]: sub_acc = 9 + (i8_val * 3 / 2)
        let i8_val = raw_0f as i8;
        (9i32 + i8_val as i32 * 3 / 2) as u32
    };

    // ECMAINT movement formula: sub-grid of 9 units per cell.
    // Each turn: sub_acc += speed * 8, integer_move = sub_acc / 9, sub_acc %= 9.
    let sub_acc_new = sub_acc_prev + (speed as u32) * 8;
    let sub_acc_after = sub_acc_new % 9;

    let int_move = (sub_acc_new / 9) as f64;
    let hazard_intel = visible_hazards_by_empire
        .get(owner_empire_raw.saturating_sub(1) as usize)
        .cloned()
        .unwrap_or_default();
    let exact_start = {
        let fleet = &game_data.fleets.records[fleet_idx];
        if is_at_rest {
            [f64::from(current_x), f64::from(current_y)]
        } else {
            decode_exact_position(fleet).unwrap_or([f64::from(current_x), f64::from(current_y)])
        }
    };
    let use_route_geometry = !visible_hazard_intel_is_empty(&hazard_intel);
    let route = if use_route_geometry {
        plan_route_with_intel(game_data, fleet_idx, &hazard_intel)
    } else {
        None
    };
    let exact_end = advance_exact_position(
        exact_start,
        [target_x, target_y],
        int_move,
        route.as_ref(),
        use_route_geometry,
    );
    let [new_x, new_y] = rounded_coords_from_exact(exact_end, [target_x, target_y]);

    // Update fleet position
    game_data.fleets.records[fleet_idx].set_current_location_coords_raw([new_x, new_y]);

    // Check if arrived at target
    if new_x == target_x && new_y == target_y {
        // Completion orders fall back to Hold on arrival. Persistent standing
        // orders and delayed hostile-world orders remain armed after reaching
        // their target so later merge/assault phases can still see them.
        let order_code_on_arrival = game_data.fleets.records[fleet_idx].standing_order_code_raw();
        let preserves_order_on_arrival =
            order_preserves_state_on_arrival(Order::from_raw(order_code_on_arrival));

        if !preserves_order_on_arrival {
            // Arrivals that execute and complete: clear speed and order immediately.
            game_data.fleets.records[fleet_idx].set_current_speed(0);
            game_data.fleets.records[fleet_idx].set_standing_order_kind(Order::HoldPosition);
        }
        // Orders that preserve state on arrival remain armed for their ongoing or
        // delayed-resolution behavior.

        // Set tuple_c_payload and raw[0x1e] on arrival (confirmed from fleet fixture).
        // raw[0x19]: 0x81 -> 0x80 on arrival (NOT 0x00).
        // raw[0x0d] and raw[0x0f] are NOT changed on arrival (confirmed: stay at 0x80/0x00).
        game_data.fleets.records[fleet_idx].raw[0x19] = 0x80;
        game_data.fleets.records[fleet_idx].raw[0x1a] = 0xb9;
        game_data.fleets.records[fleet_idx].raw[0x1b] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x1c] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x1d] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x1e] = 0x7f;

        return Ok(true);
    }

    // Fleet is still in transit (did not arrive this turn).
    // Set transit flag bytes on first turn of movement.
    if is_at_rest {
        game_data.fleets.records[fleet_idx].raw[0x0d] = 0x7f;
        game_data.fleets.records[fleet_idx].raw[0x0e] = 0xc0;
        game_data.fleets.records[fleet_idx].raw[0x10] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x11] = 0xff;
        game_data.fleets.records[fleet_idx].raw[0x12] = 0x7f;
        // Clear departure flag in raw[0x19] when fleet starts moving but does not arrive
        game_data.fleets.records[fleet_idx].raw[0x19] = 0x00;
    }

    store_exact_position(&mut game_data.fleets.records[fleet_idx], exact_end);

    // Update fractional accumulator in raw[0x0f].
    // Encoding: raw[0x0f] as i8 = (sub_acc_after - 9) * 2 / 3
    let new_0f = ((sub_acc_after as i32 - 9) * 2 / 3) as i8;
    game_data.fleets.records[fleet_idx].raw[0x0f] = new_0f as u8;

    Ok(false)
}
