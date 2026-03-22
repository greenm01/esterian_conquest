use std::collections::BTreeMap;

use crate::maint::PlanetIntelSource;
use crate::storage::{IntelTier, PlanetIntelSnapshot};
use crate::{CoreGameData, DatabaseDat, DatabaseRecord, PlayerStarmapWorld};

pub fn merge_player_intel_from_compat(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    viewer_empire_id: u8,
    year: u16,
    previous: Option<&BTreeMap<usize, PlanetIntelSnapshot>>,
) -> BTreeMap<usize, PlanetIntelSnapshot> {
    compat_worlds(game_data, database, viewer_empire_id)
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

fn compat_worlds(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    viewer_empire_id: u8,
) -> Vec<PlayerStarmapWorld> {
    let planet_count = game_data.planets.records.len();
    let viewer_index = viewer_empire_id.saturating_sub(1) as usize;
    game_data
        .planets
        .records
        .iter()
        .enumerate()
        .map(|(planet_index, planet)| {
            let record_index = DatabaseDat::record_index(planet_index, viewer_index, planet_count);
            let fallback_record = DatabaseRecord::new_zeroed();
            let db_record = database
                .records
                .get(record_index)
                .unwrap_or(&fallback_record);
            let actual_owner_empire_id = planet.owner_empire_slot_raw();
            let is_owned_world = actual_owner_empire_id == viewer_empire_id;
            let known_name = if is_owned_world {
                Some(planet.status_or_name_summary())
            } else {
                decode_known_name(db_record)
            };
            let known_owner_empire_id = if is_owned_world {
                Some(viewer_empire_id)
            } else {
                decode_known_owner_empire_id(db_record, game_data)
            };
            let known_owner_empire_name = known_owner_empire_id.map(|empire_id| {
                game_data.player.records[empire_id as usize - 1].controlled_empire_name_summary()
            });

            PlayerStarmapWorld {
                planet_record_index_1_based: planet_index + 1,
                coords: planet.coords_raw(),
                known_name,
                known_owner_empire_id,
                known_owner_empire_name,
                known_potential_production: if is_owned_world {
                    Some(planet.potential_production_points())
                } else {
                    decode_known_u16(db_record.raw[0x1c])
                },
                known_armies: if is_owned_world {
                    Some(planet.army_count_raw())
                } else {
                    decode_known_u8(db_record.raw[0x23])
                },
                known_ground_batteries: if is_owned_world {
                    Some(planet.ground_batteries_raw())
                } else {
                    decode_known_u8(db_record.raw[0x25])
                },
                known_current_production: if is_owned_world {
                    planet.present_production_points().map(|v| v as u8)
                } else {
                    decode_known_u8(db_record.raw[0x1d])
                },
                known_stored_points: if is_owned_world {
                    Some(planet.stored_goods_raw() as u16)
                } else {
                    decode_known_u16_word(db_record.word_at(0x1e))
                },
            }
        })
        .collect()
}

pub fn merge_player_intel_from_runtime(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    year: u16,
    previous: Option<&BTreeMap<usize, PlanetIntelSnapshot>>,
    current_turn_grants: Option<&BTreeMap<usize, PlanetIntelSource>>,
) -> BTreeMap<usize, PlanetIntelSnapshot> {
    game_data
        .planets
        .records
        .iter()
        .enumerate()
        .map(|(planet_index, planet)| {
            let planet_record_index_1_based = planet_index + 1;
            let previous_snapshot =
                previous.and_then(|rows| rows.get(&planet_record_index_1_based));
            let current_snapshot = snapshot_from_runtime(
                planet_record_index_1_based,
                planet,
                viewer_empire_id,
                current_turn_grants.and_then(|rows| rows.get(&planet_record_index_1_based)),
            );
            let merged_snapshot =
                merge_snapshot(previous_snapshot, &current_snapshot, viewer_empire_id, year);
            (planet_record_index_1_based, merged_snapshot)
        })
        .collect()
}

pub fn extract_player_intel_from_compat_database(
    game_data: &CoreGameData,
    database: &DatabaseDat,
    year: u16,
) -> Vec<BTreeMap<usize, PlanetIntelSnapshot>> {
    (1..=game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            merge_player_intel_from_compat(game_data, database, viewer_empire_id, year, None)
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
        known_current_production: world.known_current_production,
        known_stored_points: world.known_stored_points,
    }
}

fn snapshot_from_runtime(
    planet_record_index_1_based: usize,
    planet: &crate::PlanetRecord,
    viewer_empire_id: u8,
    current_turn_grant: Option<&PlanetIntelSource>,
) -> PlanetIntelSnapshot {
    let owner_empire_id = planet.owner_empire_slot_raw();
    if owner_empire_id == viewer_empire_id {
        return PlanetIntelSnapshot {
            planet_record_index_1_based,
            intel_tier: IntelTier::Owned,
            last_intel_year: None,
            known_name: Some(planet.status_or_name_summary()),
            known_owner_empire_id: Some(viewer_empire_id),
            known_potential_production: Some(planet.potential_production_points()),
            known_armies: Some(planet.army_count_raw()),
            known_ground_batteries: Some(planet.ground_batteries_raw()),
            known_current_production: planet
                .present_production_points()
                .map(|value| value.min(u16::from(u8::MAX)) as u8),
            known_stored_points: Some(planet.stored_goods_raw().min(u32::from(u16::MAX)) as u16),
        };
    }

    let mut snapshot = PlanetIntelSnapshot {
        planet_record_index_1_based,
        intel_tier: IntelTier::Unknown,
        last_intel_year: None,
        known_name: None,
        known_owner_empire_id: None,
        known_potential_production: None,
        known_armies: None,
        known_ground_batteries: None,
        known_current_production: None,
        known_stored_points: None,
    };

    let Some(source) = current_turn_grant.copied() else {
        return snapshot;
    };

    snapshot.known_name = Some(planet.status_or_name_summary());
    snapshot.known_owner_empire_id = (owner_empire_id != 0).then_some(owner_empire_id);
    snapshot.known_potential_production = Some(planet.potential_production_points());

    match source {
        PlanetIntelSource::ScoutSolarSystem | PlanetIntelSource::AssaultSuccess => {
            snapshot.known_armies = Some(planet.army_count_raw());
            snapshot.known_ground_batteries = Some(planet.ground_batteries_raw());
            snapshot.known_current_production = planet
                .present_production_points()
                .map(|value| value.min(u16::from(u8::MAX)) as u8);
            snapshot.known_stored_points =
                Some(planet.stored_goods_raw().min(u32::from(u16::MAX)) as u16);
        }
        PlanetIntelSource::ViewWorld | PlanetIntelSource::AssaultFailure => {}
    }

    snapshot.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &snapshot);
    snapshot
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
        if merged.known_current_production.is_none() {
            merged.known_current_production = previous.known_current_production;
        }
        if merged.known_stored_points.is_none() {
            merged.known_stored_points = previous.known_stored_points;
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
    Option<u8>,
    Option<u16>,
) {
    (
        snapshot.intel_tier,
        snapshot.known_name.as_deref(),
        snapshot.known_owner_empire_id,
        snapshot.known_potential_production,
        snapshot.known_armies,
        snapshot.known_ground_batteries,
        snapshot.known_current_production,
        snapshot.known_stored_points,
    )
}

fn decode_known_name(record: &DatabaseRecord) -> Option<String> {
    let name = String::from_utf8_lossy(record.planet_name_bytes())
        .trim()
        .to_string();
    if name.is_empty() || name.eq_ignore_ascii_case("unknown") {
        None
    } else {
        Some(name)
    }
}

fn decode_known_owner_empire_id(record: &DatabaseRecord, game_data: &CoreGameData) -> Option<u8> {
    let raw = record.raw[0x15];
    if raw >= 1 && raw <= game_data.conquest.player_count() {
        Some(raw)
    } else {
        None
    }
}

fn decode_known_u16(raw: u8) -> Option<u16> {
    if raw == 0 || raw == 0xff {
        None
    } else {
        Some(raw as u16)
    }
}

fn decode_known_u8(raw: u8) -> Option<u8> {
    if raw == 0 || raw == 0xff {
        None
    } else {
        Some(raw)
    }
}

fn decode_known_u16_word(raw: u16) -> Option<u16> {
    if raw == 0 || raw == 0xffff {
        None
    } else {
        Some(raw)
    }
}
