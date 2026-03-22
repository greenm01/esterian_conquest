use super::super::{FleetMergeEvent, Mission};
use super::helpers::merge_one_fleet_into_host;
use crate::{CoreGameData, Order};

/// Merge co-located friendly fleets for players flagged for combat consolidation.
///
/// **Trigger:** only players whose `PLAYER.DAT raw[0x00] == 0xff` have their
/// fleets merged. This byte is a combat-engagement flag set by ECGAME when the
/// player has declared war or been flagged as a rogue aggressor. Values
/// `0x00`, `0x01`, `0x02`, etc. leave fleets untouched.
///
/// Confirmed by black-box oracle testing (econ/fleet-battle/invade fixtures):
/// - Setting player 1 raw[0x00] to `0x00` prevents the merge entirely.
/// - Only `0xff` triggers co-location merging.
///
/// **Merge rules (confirmed from econ-pre/post fixture pair):**
/// - All fleets belonging to the flagged player at the same coordinates are
///   merged into the lowest-indexed fleet at that location (the survivor).
/// - Ship counts are summed.
/// - Surviving fleet's ROE is set to 10 (maximum aggression).
/// - Surviving fleet's next_fleet_id (raw[0x03]) and prev_fleet_id (raw[0x07])
///   chain links are cleared to 0x00.
/// - Merged fleet records are deleted from the array.
/// - Fleet ID links and player first/last fleet IDs are remapped afterward.
pub(super) fn process_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<Vec<FleetMergeEvent>, Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    if fleet_count == 0 {
        return Ok(Vec::new());
    }

    let mut to_remove = vec![false; fleet_count];
    let mut merge_events = Vec::new();
    let mut players_with_merges = vec![false; game_data.player.records.len()];

    for (player_idx, player_merged) in players_with_merges.iter_mut().enumerate() {
        if game_data.player.records[player_idx].raw[0x00] != 0xff {
            continue;
        }

        let owner = (player_idx + 1) as u8;
        let player_fleet_indices: Vec<usize> = (0..fleet_count)
            .filter(|&i| game_data.fleets.records[i].owner_empire_raw() == owner)
            .collect();

        let mut coord_to_survivor: std::collections::HashMap<[u8; 2], usize> =
            std::collections::HashMap::new();

        for &fi in &player_fleet_indices {
            let coords = game_data.fleets.records[fi].current_location_coords_raw();
            if let Some(&survivor_idx) = coord_to_survivor.get(&coords) {
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

                merge_one_fleet_into_host(game_data, survivor_idx, fi);
            } else {
                coord_to_survivor.insert(coords, fi);
            }
        }

        for (&coords, &fi) in &coord_to_survivor {
            let had_merges = player_fleet_indices.iter().any(|&other| {
                other != fi
                    && game_data.fleets.records[other].current_location_coords_raw() == coords
            });

            if had_merges {
                game_data.fleets.records[fi].raw[0x03] = 0x00;
                game_data.fleets.records[fi].raw[0x07] = 0x00;
                game_data.fleets.records[fi].set_rules_of_engagement(10);
                *player_merged = true;
            }
        }
    }
    super::super::apply_fleet_removal_remap(game_data, &to_remove);

    for (player_idx, had_merge) in players_with_merges.into_iter().enumerate() {
        if had_merge && game_data.player.records[player_idx].raw[0x00] == 0xff {
            game_data.player.records[player_idx].raw[0x51] = 0x41;
        }
    }

    Ok(merge_events)
}
