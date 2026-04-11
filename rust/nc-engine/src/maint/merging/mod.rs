mod colonization;
mod consolidation;
mod helpers;
mod mission;

use super::{ColonizationEvent, ColonizationResolvedEvent, FleetMergeEvent, JoinMissionHostEvent};
use crate::{CoreGameData, maint::FleetRemovalRemapInfo};

pub(super) fn process_colonizations(
    game_data: &mut CoreGameData,
    events: &[ColonizationEvent],
) -> Result<Vec<ColonizationResolvedEvent>, Box<dyn std::error::Error>> {
    colonization::process_colonizations(game_data, events)
}

pub(super) fn process_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<(Vec<FleetMergeEvent>, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    consolidation::process_fleet_merging(game_data)
}

pub(super) fn process_mission_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<(Vec<FleetMergeEvent>, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    mission::process_mission_fleet_merging(game_data)
}

pub(super) fn process_join_host_updates(
    game_data: &mut CoreGameData,
    merge_events: &[FleetMergeEvent],
    fleet_number_by_id: &std::collections::HashMap<u8, u8>,
    destroyed_join_host_fleet_numbers: &std::collections::HashMap<u8, u8>,
    prior_join_host_ids: &std::collections::HashMap<u8, u8>,
) -> Vec<JoinMissionHostEvent> {
    mission::process_join_host_updates(
        game_data,
        merge_events,
        fleet_number_by_id,
        destroyed_join_host_fleet_numbers,
        prior_join_host_ids,
    )
}
