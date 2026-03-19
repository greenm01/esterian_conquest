use std::collections::BTreeMap;

use crate::storage::{IntelTier, PlanetIntelSnapshot};
use crate::{CoreGameData, DatabaseDat, PlayerStarmapWorld, build_player_starmap_projection};

pub fn merge_player_intel_from_compat(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    viewer_empire_id: u8,
    year: u16,
    previous: Option<&BTreeMap<usize, PlanetIntelSnapshot>>,
) -> BTreeMap<usize, PlanetIntelSnapshot> {
    let projection = build_player_starmap_projection(game_data, database, viewer_empire_id);
    projection
        .worlds
        .into_iter()
        .map(|world| {
            let planet_record_index_1_based = world.planet_record_index_1_based;
            let previous_snapshot =
                previous.and_then(|rows| rows.get(&planet_record_index_1_based));
            let current_snapshot = snapshot_from_world(&world, viewer_empire_id, None);
            let merged_snapshot =
                merge_snapshot(previous_snapshot, &current_snapshot, viewer_empire_id, year);
            (planet_record_index_1_based, merged_snapshot)
        })
        .collect()
}

pub(crate) fn infer_intel_tier(viewer_empire_id: u8, world: &PlayerStarmapWorld) -> IntelTier {
    if world.known_owner_empire_id == Some(viewer_empire_id) {
        IntelTier::Owned
    } else if world.known_armies.is_some() || world.known_ground_batteries.is_some() {
        IntelTier::Full
    } else if world.known_name.is_some()
        || world.known_owner_empire_id.is_some()
        || world.known_potential_production.is_some()
    {
        IntelTier::Partial
    } else {
        IntelTier::Unknown
    }
}

pub(crate) fn infer_intel_tier_from_snapshot(
    viewer_empire_id: u8,
    snapshot: &PlanetIntelSnapshot,
) -> IntelTier {
    if snapshot.known_owner_empire_id == Some(viewer_empire_id) {
        IntelTier::Owned
    } else if snapshot.known_armies.is_some() || snapshot.known_ground_batteries.is_some() {
        IntelTier::Full
    } else if snapshot.known_name.is_some()
        || snapshot.known_owner_empire_id.is_some()
        || snapshot.known_potential_production.is_some()
    {
        IntelTier::Partial
    } else {
        IntelTier::Unknown
    }
}

fn snapshot_from_world(
    world: &PlayerStarmapWorld,
    viewer_empire_id: u8,
    last_intel_year: Option<u16>,
) -> PlanetIntelSnapshot {
    PlanetIntelSnapshot {
        planet_record_index_1_based: world.planet_record_index_1_based,
        intel_tier: infer_intel_tier(viewer_empire_id, world),
        last_intel_year,
        known_name: world.known_name.clone(),
        known_owner_empire_id: world.known_owner_empire_id,
        known_potential_production: world.known_potential_production,
        known_armies: world.known_armies,
        known_ground_batteries: world.known_ground_batteries,
    }
}

fn merge_snapshot(
    previous: Option<&PlanetIntelSnapshot>,
    current: &PlanetIntelSnapshot,
    viewer_empire_id: u8,
    year: u16,
) -> PlanetIntelSnapshot {
    let mut merged = current.clone();
    if let Some(previous) = previous {
        if merged.known_name.is_none() {
            merged.known_name = previous.known_name.clone();
        }
        if merged.known_owner_empire_id.is_none() {
            merged.known_owner_empire_id = previous.known_owner_empire_id;
        }
        if merged.known_potential_production.is_none() {
            merged.known_potential_production = previous.known_potential_production;
        }
        if merged.known_armies.is_none() {
            merged.known_armies = previous.known_armies;
        }
        if merged.known_ground_batteries.is_none() {
            merged.known_ground_batteries = previous.known_ground_batteries;
        }
    }

    merged.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &merged);
    merged.last_intel_year = match merged.intel_tier {
        IntelTier::Unknown => None,
        IntelTier::Owned => Some(year),
        _ => {
            let previous_year = previous.and_then(|snapshot| snapshot.last_intel_year);
            if previous
                .map(|snapshot| snapshot_fingerprint(snapshot) == snapshot_fingerprint(&merged))
                .unwrap_or(false)
            {
                previous_year.or(Some(year))
            } else {
                Some(year)
            }
        }
    };
    merged
}

fn snapshot_fingerprint(
    snapshot: &PlanetIntelSnapshot,
) -> (
    IntelTier,
    Option<&str>,
    Option<u8>,
    Option<u16>,
    Option<u8>,
    Option<u8>,
) {
    (
        snapshot.intel_tier,
        snapshot.known_name.as_deref(),
        snapshot.known_owner_empire_id,
        snapshot.known_potential_production,
        snapshot.known_armies,
        snapshot.known_ground_batteries,
    )
}
