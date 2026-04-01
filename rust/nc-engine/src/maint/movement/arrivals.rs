use super::super::{
    ColonizationEvent, DiplomaticEscalationEvent, Mission, MissionEvent, MissionOutcome,
    MovementEvents, PlanetIntelEvent, PlanetIntelSource,
};
use super::salvage::queue_salvage_resolution;
use super::stepper::set_fleet_to_deep_space_hold;
use crate::{CoreGameData, Order};

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

pub(super) fn handle_fleet_arrival(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
    to_remove: &mut [bool],
    fleet_idx: usize,
    order_kind: Order,
    owner_empire: u8,
    coords: [u8; 2],
) -> Result<(), Box<dyn std::error::Error>> {
    match order_kind {
        Order::ColonizeWorld => {
            movement_events.colonization_events.push(ColonizationEvent {
                fleet_idx,
                coords,
                owner_empire,
            });
        }
        Order::ScoutSector => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::ScoutSector,
                outcome: MissionOutcome::Succeeded,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::ScoutSolarSystem => {
            if let Some(planet_idx) = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords)
            {
                movement_events.planet_intel_events.push(PlanetIntelEvent {
                    planet_idx,
                    viewer_empire_raw: owner_empire,
                    source: PlanetIntelSource::ScoutSolarSystem,
                });
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::ScoutSolarSystem,
                outcome: MissionOutcome::Succeeded,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::ViewWorld => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            if let Some(planet_idx) = planet_idx {
                movement_events.planet_intel_events.push(PlanetIntelEvent {
                    planet_idx,
                    viewer_empire_raw: owner_empire,
                    source: PlanetIntelSource::ViewWorld,
                });
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::ViewWorld,
                outcome: if planet_idx.is_some() {
                    MissionOutcome::Succeeded
                } else {
                    MissionOutcome::Failed
                },
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
            set_fleet_to_deep_space_hold(&mut game_data.fleets.records[fleet_idx]);
        }
        Order::Salvage => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            queue_salvage_resolution(
                game_data,
                movement_events,
                to_remove,
                fleet_idx,
                owner_empire,
                planet_idx,
                coords,
            )?;
        }
        Order::GuardStarbase => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::GuardStarbase,
                outcome: MissionOutcome::Arrived,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::GuardBlockadeWorld => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            if let Some(planet_idx) = planet_idx {
                let defender_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::GuardBlockadeWorld,
                outcome: MissionOutcome::Arrived,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::RendezvousSector => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::RendezvousSector,
                outcome: MissionOutcome::Arrived,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::MoveOnly => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::MoveOnly,
                outcome: MissionOutcome::Succeeded,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::PatrolSector => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::PatrolSector,
                outcome: MissionOutcome::Arrived,
                planet_idx: None,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::SeekHome => {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::SeekHome,
                outcome: MissionOutcome::Succeeded,
                planet_idx: game_data
                    .planets
                    .records
                    .iter()
                    .position(|planet| planet.coords_raw() == coords),
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::BombardWorld => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            if let Some(planet_idx) = planet_idx {
                let defender_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::BombardWorld,
                outcome: MissionOutcome::Arrived,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::InvadeWorld => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            if let Some(planet_idx) = planet_idx {
                let defender_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::InvadeWorld,
                outcome: MissionOutcome::Arrived,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        Order::BlitzWorld => {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == coords);
            if let Some(planet_idx) = planet_idx {
                let defender_empire = game_data.planets.records[planet_idx].owner_empire_slot_raw();
                queue_local_intrusion_escalation(movement_events, owner_empire, defender_empire);
            }
            movement_events.mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: owner_empire,
                kind: Mission::BlitzWorld,
                outcome: MissionOutcome::Arrived,
                planet_idx,
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week: None,
            });
        }
        _ => {}
    }

    Ok(())
}
