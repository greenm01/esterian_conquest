use super::super::{
    Mission, MissionEvent, MissionOutcome, MovementEvents, SalvageFailureReason,
    SalvageResolvedEvent,
};
use super::stepper::set_fleet_to_local_hold;
use crate::{CoreGameData, ProductionItemKind};

pub(super) fn queue_salvage_resolution(
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
        set_fleet_to_local_hold(fleet);
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
        set_fleet_to_local_hold(fleet);
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

pub(super) fn remap_movement_event_fleet_indices_after_removal(
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
