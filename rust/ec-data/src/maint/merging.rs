use crate::{CoreGameData, Order};
use super::{
    ColonizationEvent, ColonizationResolvedEvent, FleetMergeEvent, JoinMissionHostEvent,
    Mission, MissionEvent, MissionOutcome,
};

/// Apply colonization events to PLANETS.DAT and PLAYER.DAT.
///
/// When a ColonizeWorld fleet arrives at an unowned planet:
/// - Planet name set to "Not Named Yet"
/// - Planet ownership_status set to 2 (owned)
/// - Planet owner_empire_slot set to colonizing empire
/// - Planet army_count set to 1 (colonist armies)
/// - Planet raw[0x03] set to 0x81 (colonization flag in potential_production high byte)
/// - PLAYER record planet_count incremented
/// - PLAYER record raw[0x52] incremented (confirmed from fleet fixture)
///
/// Confirmed from fleet-scenario fixture: fleet 0 ColonizeWorld arrives at (15,13),
/// planet 13 colonized by empire 1, player 0 record updated.
pub(super) fn process_colonizations(
    game_data: &mut CoreGameData,
    events: &[ColonizationEvent],
) -> Result<Vec<ColonizationResolvedEvent>, Box<dyn std::error::Error>> {
    let mut resolved = Vec::new();
    for event in events {
        let [cx, cy] = event.coords;

        // Find planet at colonization coordinates
        let planet_idx = game_data.planets.records.iter().position(|p| {
            let [px, py] = p.coords_raw();
            px == cx && py == cy
        });

        if let Some(idx) = planet_idx {
            let planet = &mut game_data.planets.records[idx];

            // Only colonize if currently unowned (name "Unowned" or empty owner)
            let is_unowned = planet.owner_empire_slot_raw() == 0;
            if is_unowned {
                // Set name to "Not Named Yet"
                planet.set_planet_name("Not Named Yet");

                // Set ownership
                planet.set_ownership_status_raw(2);
                planet.set_owner_empire_slot_raw(event.owner_empire);

                // Set colonist armies (1 army for new colony)
                planet.set_army_count_raw(1);

                // Set colonization flag in raw[0x03] (high byte of potential_production pair)
                // 0x81 observed in fixture: bit 7 (0x80) + bit 0 (0x01)
                planet.raw[0x03] = 0x81;

                // Update PLAYER.DAT for the colonizing empire
                // Empire index is 1-based in fleet records, 0-based in player records
                let player_idx = (event.owner_empire as usize).saturating_sub(1);
                if player_idx < game_data.player.records.len() {
                    // Increment planet count at raw[0x50]
                    let current_count = game_data.player.records[player_idx].raw[0x50];
                    game_data.player.records[player_idx].raw[0x50] =
                        current_count.saturating_add(1);

                    // Increment score/economic field at raw[0x52]
                    let current_score = game_data.player.records[player_idx].raw[0x52];
                    game_data.player.records[player_idx].raw[0x52] =
                        current_score.saturating_add(1);
                }

                resolved.push(ColonizationResolvedEvent::Succeeded {
                    fleet_idx: event.fleet_idx,
                    planet_idx: idx,
                    colonizer_empire_raw: event.owner_empire,
                    stardate_week: None,
                });
            } else {
                resolved.push(ColonizationResolvedEvent::BlockedByOwner {
                    fleet_idx: event.fleet_idx,
                    planet_idx: idx,
                    colonizer_empire_raw: event.owner_empire,
                    owner_empire_raw: planet.owner_empire_slot_raw(),
                    stardate_week: None,
                });
            }
        }
    }

    Ok(resolved)
}

/// Merge co-located friendly fleets for players flagged for combat consolidation.
///
/// **Trigger:** only players whose `PLAYER.DAT raw[0x00] == 0xff` have their
/// fleets merged.  This byte is a combat-engagement flag set by ECGAME when the
/// player has declared war or been flagged as a rogue aggressor.  Values
/// `0x00`, `0x01`, `0x02`, etc. leave fleets untouched.
///
/// Confirmed by black-box oracle testing (econ/fleet-battle/invade fixtures):
/// - Setting player 1 raw[0x00] to `0x00` prevents the merge entirely.
/// - Only `0xff` triggers co-location merging.
///
/// **Merge rules (confirmed from econ-pre/post fixture pair):**
/// - All fleets belonging to the flagged player at the same coordinates are
///   merged into the lowest-indexed fleet at that location (the survivor).
/// - Ship counts (BB, CA, DD, TT, ARMY, ET, scouts) are summed.
///   (Confirmed: econ post CA=52 = sum of 4 fleets with CA=1+1+50+0.)
/// - Surviving fleet's ROE is set to 10 (maximum aggression).
/// - Surviving fleet's next_fleet_id (raw[0x03]) and prev_fleet_id (raw[0x07])
///   chain links are cleared to 0x00.
/// - Merged (removed) fleet records are deleted from the array.
/// - After deletion the global fleet-ID fields are remapped:
///   fleet_id (raw[0x05]), next_fleet_id (raw[0x03]), prev_fleet_id (raw[0x07])
///   are decremented by the count of removed slots before each position.
/// - Surviving local fleet numbers (raw[0x00]) are preserved per empire; gaps
///   remain and can be reused by later commissioning.
/// - PLAYER.DAT first_fleet_id (raw[0x40]) and last_fleet_id (raw[0x42]) are
///   updated for all players to reflect the remapped IDs.
pub(super) fn process_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<Vec<FleetMergeEvent>, Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    if fleet_count == 0 {
        return Ok(Vec::new());
    }

    // Build a list of which fleets should be removed (merged into another).
    let mut to_remove: Vec<bool> = vec![false; fleet_count];
    let mut merge_events = Vec::new();
    let mut players_with_merges = vec![false; game_data.player.records.len()];

    let player_count = game_data.player.records.len();
    for player_idx in 0..player_count {
        // Only merge fleets for players flagged with the combat-engagement byte 0xff.
        if game_data.player.records[player_idx].raw[0x00] != 0xff {
            continue;
        }

        let owner = (player_idx + 1) as u8;

        // Collect fleet indices for this player, in order.
        let player_fleet_indices: Vec<usize> = (0..fleet_count)
            .filter(|&i| game_data.fleets.records[i].owner_empire_raw() == owner)
            .collect();

        // Group by coords: for each coord pair, the first fleet is the survivor.
        let mut coord_to_survivor: std::collections::HashMap<[u8; 2], usize> =
            std::collections::HashMap::new();

        for &fi in &player_fleet_indices {
            let coords = game_data.fleets.records[fi].current_location_coords_raw();
            if let Some(&survivor_idx) = coord_to_survivor.get(&coords) {
                // This fleet duplicates an existing location → merge into survivor.
                to_remove[fi] = true;

                let merging_order = game_data.fleets.records[fi].standing_order_kind();
                let merge_kind = match merging_order {
                    Order::JoinAnotherFleet => Some(Mission::JoinAnotherFleet),
                    Order::RendezvousSector => Some(Mission::RendezvousSector),
                    _ => None,
                };
                if let Some(kind) = merge_kind {
                    merge_events.push(FleetMergeEvent {
                        fleet_idx: fi,
                        owner_empire_raw: owner,
                        kind,
                        host_fleet_id: game_data.fleets.records[survivor_idx].fleet_id(),
                        absorbed_fleet_id: game_data.fleets.records[fi].fleet_id(),
                        coords,
                        survivor_side: false,
                        stardate_week: None,
                    });
                    if kind == Mission::RendezvousSector {
                        merge_events.push(FleetMergeEvent {
                            fleet_idx: survivor_idx,
                            owner_empire_raw: owner,
                            kind,
                            host_fleet_id: game_data.fleets.records[survivor_idx].fleet_id(),
                            absorbed_fleet_id: game_data.fleets.records[fi].fleet_id(),
                            coords,
                            survivor_side: true,
                            stardate_week: None,
                        });
                    }
                }

                // Sum ship counts into survivor.
                let bb = game_data.fleets.records[fi].battleship_count();
                let ca = game_data.fleets.records[fi].cruiser_count();
                let dd = game_data.fleets.records[fi].destroyer_count();
                let tt = game_data.fleets.records[fi].troop_transport_count();
                let army = game_data.fleets.records[fi].army_count();
                let et = game_data.fleets.records[fi].etac_count();
                let sc = game_data.fleets.records[fi].scout_count();

                let s = &mut game_data.fleets.records[survivor_idx];
                s.set_battleship_count(s.battleship_count().saturating_add(bb));
                s.set_cruiser_count(s.cruiser_count().saturating_add(ca));
                s.set_destroyer_count(s.destroyer_count().saturating_add(dd));
                s.set_troop_transport_count(s.troop_transport_count().saturating_add(tt));
                s.set_army_count(s.army_count().saturating_add(army));
                s.set_etac_count(s.etac_count().saturating_add(et));
                s.set_scout_count(s.scout_count().saturating_add(sc));
                s.recompute_max_speed_from_composition();
            } else {
                coord_to_survivor.insert(coords, fi);
            }
        }

        // Set ROE=10 and clear chain links on any survivor that absorbed other fleets.
        for (&coords, &fi) in &coord_to_survivor {
            let had_merges = player_fleet_indices.iter().any(|&other| {
                other != fi
                    && game_data.fleets.records[other].current_location_coords_raw() == coords
            });

            if had_merges {
                game_data.fleets.records[fi].raw[0x03] = 0x00; // next_fleet_id
                game_data.fleets.records[fi].raw[0x07] = 0x00; // prev_fleet_id
                game_data.fleets.records[fi].set_rules_of_engagement(10);
                players_with_merges[player_idx] = true;
            }
        }
    }
    super::apply_fleet_removal_remap(game_data, &to_remove);

    // raw[0x51]: set to 0x41 for players whose fleets were merged this turn.
    // Observed consistently across econ/fleet-battle/invade post-fixtures.
    for (player_idx, had_merge) in players_with_merges.into_iter().enumerate() {
        if had_merge && game_data.player.records[player_idx].raw[0x00] == 0xff {
            game_data.player.records[player_idx].raw[0x51] = 0x41;
        }
    }

    Ok(merge_events)
}

pub(super) fn process_mission_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<Vec<FleetMergeEvent>, Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    if fleet_count == 0 {
        return Ok(Vec::new());
    }

    let mut to_remove = vec![false; fleet_count];
    let mut merge_events = Vec::new();

    let fleet_lookup: std::collections::HashMap<u8, usize> = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .map(|(idx, fleet)| (fleet.fleet_id(), idx))
        .collect();

    for joiner_idx in 0..fleet_count {
        if to_remove[joiner_idx] {
            continue;
        }
        let joiner = &game_data.fleets.records[joiner_idx];
        if joiner.standing_order_kind() != Order::JoinAnotherFleet {
            continue;
        }
        let host_id = joiner.join_host_fleet_id_raw();
        let joiner_owner = joiner.owner_empire_raw();
        let joiner_fleet_id = joiner.fleet_id();
        let joiner_coords = joiner.current_location_coords_raw();
        let Some(&host_idx) = fleet_lookup.get(&host_id) else {
            continue;
        };
        if host_idx == joiner_idx || to_remove[host_idx] {
            continue;
        }
        let same_owner = game_data.fleets.records[host_idx].owner_empire_raw() == joiner_owner;
        let same_coords =
            game_data.fleets.records[host_idx].current_location_coords_raw() == joiner_coords;
        if !same_owner || !same_coords {
            continue;
        }

        merge_one_fleet_into_host(game_data, host_idx, joiner_idx);
        to_remove[joiner_idx] = true;
        merge_events.push(FleetMergeEvent {
            fleet_idx: joiner_idx,
            owner_empire_raw: joiner_owner,
            kind: Mission::JoinAnotherFleet,
            host_fleet_id: game_data.fleets.records[host_idx].fleet_id(),
            absorbed_fleet_id: joiner_fleet_id,
            coords: joiner_coords,
            survivor_side: false,
            stardate_week: None,
        });
    }

    let mut rendezvous_groups: std::collections::BTreeMap<(u8, [u8; 2]), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if to_remove[idx] || fleet.standing_order_kind() != Order::RendezvousSector {
            continue;
        }
        rendezvous_groups
            .entry((
                fleet.owner_empire_raw(),
                fleet.current_location_coords_raw(),
            ))
            .or_default()
            .push(idx);
    }

    for ((_owner, coords), mut group) in rendezvous_groups {
        if group.len() < 2 {
            continue;
        }
        group.sort_by_key(|idx| game_data.fleets.records[*idx].fleet_id());
        let survivor_idx = group[0];
        for &absorbed_idx in group.iter().skip(1) {
            if to_remove[absorbed_idx] {
                continue;
            }
            let absorbed_id = game_data.fleets.records[absorbed_idx].fleet_id();
            let owner_empire_raw = game_data.fleets.records[absorbed_idx].owner_empire_raw();
            merge_one_fleet_into_host(game_data, survivor_idx, absorbed_idx);
            to_remove[absorbed_idx] = true;
            merge_events.push(FleetMergeEvent {
                fleet_idx: absorbed_idx,
                owner_empire_raw,
                kind: Mission::RendezvousSector,
                host_fleet_id: game_data.fleets.records[survivor_idx].fleet_id(),
                absorbed_fleet_id: absorbed_id,
                coords,
                survivor_side: false,
                stardate_week: None,
            });
            merge_events.push(FleetMergeEvent {
                fleet_idx: survivor_idx,
                owner_empire_raw,
                kind: Mission::RendezvousSector,
                host_fleet_id: game_data.fleets.records[survivor_idx].fleet_id(),
                absorbed_fleet_id: absorbed_id,
                coords,
                survivor_side: true,
                stardate_week: None,
            });
        }
    }

    if to_remove.iter().any(|remove| *remove) {
        super::apply_fleet_removal_remap(game_data, &to_remove);
    }

    Ok(merge_events)
}

fn merge_one_fleet_into_host(game_data: &mut CoreGameData, host_idx: usize, donor_idx: usize) {
    let bb = game_data.fleets.records[donor_idx].battleship_count();
    let ca = game_data.fleets.records[donor_idx].cruiser_count();
    let dd = game_data.fleets.records[donor_idx].destroyer_count();
    let tt = game_data.fleets.records[donor_idx].troop_transport_count();
    let army = game_data.fleets.records[donor_idx].army_count();
    let et = game_data.fleets.records[donor_idx].etac_count();
    let sc = game_data.fleets.records[donor_idx].scout_count();

    let host = &mut game_data.fleets.records[host_idx];
    host.set_battleship_count(host.battleship_count().saturating_add(bb));
    host.set_cruiser_count(host.cruiser_count().saturating_add(ca));
    host.set_destroyer_count(host.destroyer_count().saturating_add(dd));
    host.set_troop_transport_count(host.troop_transport_count().saturating_add(tt));
    host.set_army_count(host.army_count().saturating_add(army));
    host.set_etac_count(host.etac_count().saturating_add(et));
    host.set_scout_count(host.scout_count().saturating_add(sc));
    host.recompute_max_speed_from_composition();
}

pub(super) fn process_join_host_updates(
    game_data: &mut CoreGameData,
    merge_events: &[FleetMergeEvent],
) -> Vec<JoinMissionHostEvent> {
    let mut absorbed_to_host = std::collections::HashMap::new();
    for event in merge_events {
        if event.absorbed_fleet_id != 0 && event.absorbed_fleet_id != event.host_fleet_id {
            absorbed_to_host.insert(event.absorbed_fleet_id, event.host_fleet_id);
        }
    }

    let current_fleet_ids: std::collections::HashSet<u8> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| fleet.fleet_id())
        .collect();
    let current_host_viability: std::collections::HashMap<u8, bool> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| {
            let viable = fleet.destroyer_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.battleship_count() > 0
                || fleet.scout_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.etac_count() > 0;
            (fleet.fleet_id(), viable)
        })
        .collect();
    let current_fleet_coords: std::collections::HashMap<u8, [u8; 2]> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| (fleet.fleet_id(), fleet.current_location_coords_raw()))
        .collect();

    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.records.iter_mut().enumerate() {
        if fleet.standing_order_kind() != Order::JoinAnotherFleet {
            continue;
        }

        let host_id = fleet.join_host_fleet_id_raw();
        if host_id == 0 || host_id == fleet.fleet_id() {
            continue;
        }

        if let Some(&new_host_id) = absorbed_to_host.get(&host_id) {
            fleet.set_join_host_fleet_id_raw(new_host_id);
            if let Some(coords) = current_fleet_coords.get(&new_host_id).copied() {
                fleet.set_standing_order_target_coords_raw(coords);
            }
            events.push(JoinMissionHostEvent::Retargeted {
                fleet_idx,
                owner_empire_raw: fleet.owner_empire_raw(),
                previous_host_fleet_id: host_id,
                new_host_fleet_id: new_host_id,
                coords: fleet.current_location_coords_raw(),
            });
            continue;
        }

        let host_exists = current_fleet_ids.contains(&host_id);
        let host_viable = current_host_viability
            .get(&host_id)
            .copied()
            .unwrap_or(false);
        if !host_exists || !host_viable {
            let coords = fleet.current_location_coords_raw();
            fleet.set_standing_order_kind(Order::HoldPosition);
            fleet.set_current_speed(0);
            fleet.set_standing_order_target_coords_raw(coords);
            fleet.set_join_host_fleet_id_raw(0);
            events.push(JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire_raw: fleet.owner_empire_raw(),
                destroyed_host_fleet_id: host_id,
                coords,
            });
        }
    }

    events
}
