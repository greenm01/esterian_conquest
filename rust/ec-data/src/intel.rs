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
    let planet_count = game_data.planets.records.len();
    let viewer_index = viewer_empire_id.saturating_sub(1) as usize;
    let viewer_is_active = game_data
        .player
        .records
        .get(viewer_index)
        .map(|player| player.occupied_flag() != 0)
        .unwrap_or(false);
    compat_worlds(game_data, database, viewer_empire_id)
        .into_iter()
        .map(|world| {
            let planet_record_index_1_based = world.planet_record_index_1_based;
            let previous_snapshot =
                previous.and_then(|rows| rows.get(&planet_record_index_1_based));
            let record_index = DatabaseDat::record_index(
                planet_record_index_1_based - 1,
                viewer_index,
                planet_count,
            );
            let fallback_record = DatabaseRecord::new_zeroed();
            let db_record = database
                .records
                .get(record_index)
                .unwrap_or(&fallback_record);
            let mut current_snapshot =
                snapshot_from_compat_world(&world, viewer_empire_id, db_record);
            if current_snapshot.compat_is_orbit_seed
                && viewer_is_active
                && current_snapshot.intel_tier == IntelTier::Owned
            {
                if let Some(planet) = game_data
                    .planets
                    .records
                    .get(planet_record_index_1_based - 1)
                {
                    current_snapshot.known_name = Some(planet.status_or_name_summary());
                }
                current_snapshot.last_intel_year = Some(year);
            } else if !current_snapshot.compat_is_orbit_seed
                && current_snapshot.intel_tier != IntelTier::Unknown
                && current_snapshot.last_intel_year == Some(0)
            {
                current_snapshot.last_intel_year = Some(year);
            }
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
            let decoded_name = decode_known_name(db_record);
            let known_name = decoded_name.clone();
            let known_owner_empire_id =
                decode_known_owner_empire_id(db_record, game_data, decoded_name.is_some());
            let known_owner_empire_name = known_owner_empire_id.and_then(|empire_id| {
                (empire_id >= 1).then(|| {
                    game_data.player.records[empire_id as usize - 1]
                        .controlled_empire_name_summary()
                })
            });

            PlayerStarmapWorld {
                planet_record_index_1_based: planet_index + 1,
                coords: planet.coords_raw(),
                known_name,
                known_owner_empire_id,
                known_owner_empire_name,
                known_potential_production: decode_known_u16(db_record.raw[0x1c]),
                known_armies: decode_known_u8(db_record.raw[0x23]),
                known_ground_batteries: decode_known_u8(db_record.raw[0x25]),
                known_current_production: decode_known_u8(db_record.raw[0x1d]),
                known_stored_points: decode_known_u16_word(db_record.word_at(0x1e)),
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
            let current_turn_grant =
                current_turn_grants.and_then(|rows| rows.get(&planet_record_index_1_based));
            let current_snapshot = snapshot_from_runtime(
                planet_record_index_1_based,
                planet,
                viewer_empire_id,
                year,
                current_turn_grant,
            );
            let mut merged_snapshot =
                merge_snapshot(previous_snapshot, &current_snapshot, viewer_empire_id, year);
            if previous_snapshot
                .map(|snapshot| {
                    snapshot.compat_is_orbit_seed
                        && snapshot.seen_year == Some(0)
                        && snapshot.scout_year == Some(0)
                })
                .unwrap_or(false)
                && current_turn_grant.is_none()
                && year == 3000
            {
                merged_snapshot.compat_is_orbit_seed = true;
                merged_snapshot.last_intel_year = Some(0);
                merged_snapshot.seen_year = Some(0);
                merged_snapshot.scout_year = Some(0);
            } else if previous_snapshot
                .map(|snapshot| {
                    snapshot.compat_is_orbit_seed
                        && snapshot.seen_year == Some(0)
                        && snapshot.scout_year == Some(0)
                })
                .unwrap_or(false)
                && current_turn_grant.is_none()
                && merged_snapshot.intel_tier == IntelTier::Owned
                && year > 3000
            {
                let compat_year = year.saturating_sub(1);
                merged_snapshot.seen_year = Some(compat_year);
                merged_snapshot.scout_year = Some(compat_year);
            }
            if current_turn_grant.is_some() || merged_snapshot.intel_tier != IntelTier::Owned {
                merged_snapshot.compat_is_orbit_seed = false;
            }
            if current_turn_grant.is_none()
                && previous_snapshot.is_some()
                && viewer_has_fleet_presence(game_data, viewer_empire_id, planet.coords_raw())
            {
                merged_snapshot =
                    refresh_visible_snapshot_from_runtime(&merged_snapshot, planet, year);
            }
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
        compat_is_orbit_seed: false,
        last_intel_year,
        seen_year: last_intel_year,
        scout_year: last_intel_year,
        known_name: world.known_name.clone(),
        known_owner_empire_id: world.known_owner_empire_id,
        known_potential_production: world.known_potential_production,
        known_armies: world.known_armies,
        known_ground_batteries: world.known_ground_batteries,
        known_current_production: world.known_current_production,
        known_stored_points: world.known_stored_points,
        compat_word_1e: world.known_stored_points,
    }
}

fn snapshot_from_compat_world(
    world: &PlayerStarmapWorld,
    viewer_empire_id: u8,
    record: &DatabaseRecord,
) -> PlanetIntelSnapshot {
    let mut snapshot = snapshot_from_world(world, viewer_empire_id, None);
    snapshot.compat_is_orbit_seed = record.is_compat_orbit_seed_for_viewer(viewer_empire_id);
    if !matches!(snapshot.intel_tier, IntelTier::Unknown) {
        snapshot.last_intel_year = Some(record.word_at(0x16));
        snapshot.seen_year = Some(record.word_at(0x16));
        snapshot.scout_year = Some(record.word_at(0x27));
        snapshot.compat_word_1e = decode_known_u16_word(record.word_at(0x1e));
    } else {
        snapshot.compat_word_1e = None;
    }
    snapshot
}

fn snapshot_from_runtime(
    planet_record_index_1_based: usize,
    planet: &crate::PlanetRecord,
    viewer_empire_id: u8,
    year: u16,
    current_turn_grant: Option<&PlanetIntelSource>,
) -> PlanetIntelSnapshot {
    let owner_empire_id = planet.owner_empire_slot_raw();
    if owner_empire_id == viewer_empire_id {
        return PlanetIntelSnapshot {
            planet_record_index_1_based,
            intel_tier: IntelTier::Owned,
            compat_is_orbit_seed: false,
            last_intel_year: None,
            seen_year: None,
            scout_year: None,
            known_name: Some(planet.status_or_name_summary()),
            known_owner_empire_id: Some(viewer_empire_id),
            known_potential_production: Some(planet.potential_production_points()),
            known_armies: Some(planet.army_count_raw()),
            known_ground_batteries: Some(planet.ground_batteries_raw()),
            known_current_production: planet
                .present_production_points()
                .map(|value| value.min(u16::from(u8::MAX)) as u8),
            known_stored_points: Some(planet.stored_goods_raw().min(u32::from(u16::MAX)) as u16),
            compat_word_1e: None,
        };
    }

    let mut snapshot = PlanetIntelSnapshot {
        planet_record_index_1_based,
        intel_tier: IntelTier::Unknown,
        compat_is_orbit_seed: false,
        last_intel_year: None,
        seen_year: None,
        scout_year: None,
        known_name: None,
        known_owner_empire_id: None,
        known_potential_production: None,
        known_armies: None,
        known_ground_batteries: None,
        known_current_production: None,
        known_stored_points: None,
        compat_word_1e: None,
    };

    if let Some(source) = current_turn_grant.copied() {
        snapshot = snapshot_from_runtime_grant(snapshot, planet, source, year);
    }

    snapshot.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &snapshot);
    snapshot
}

fn snapshot_from_runtime_grant(
    mut snapshot: PlanetIntelSnapshot,
    planet: &crate::PlanetRecord,
    source: PlanetIntelSource,
    year: u16,
) -> PlanetIntelSnapshot {
    let compat_year = year.saturating_sub(1);

    snapshot.known_name = Some(planet.status_or_name_summary());
    snapshot.known_owner_empire_id = Some(planet.owner_empire_slot_raw());
    snapshot.known_potential_production = Some(planet.potential_production_points());

    match source {
        PlanetIntelSource::ScoutSolarSystem => {
            snapshot.known_armies = Some(planet.army_count_raw());
            snapshot.known_ground_batteries = Some(planet.ground_batteries_raw());
            snapshot.known_current_production = planet
                .present_production_points()
                .map(|value| value.min(u16::from(u8::MAX)) as u8);
            snapshot.known_stored_points =
                Some(planet.stored_goods_raw().min(u32::from(u16::MAX)) as u16);
            snapshot.compat_word_1e = Some(0x23);
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::AssaultSuccess => {
            snapshot.known_armies = Some(planet.army_count_raw());
            snapshot.known_ground_batteries = Some(planet.ground_batteries_raw());
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::ViewWorld => {
            snapshot.last_intel_year = Some(compat_year);
            snapshot.seen_year = Some(compat_year);
            snapshot.scout_year = Some(compat_year);
        }
        PlanetIntelSource::AssaultFailure => {
            snapshot.last_intel_year = Some(year);
            snapshot.seen_year = Some(year);
            snapshot.scout_year = Some(0);
        }
    }

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
        if merged.compat_word_1e.is_none() {
            merged.compat_word_1e = previous.compat_word_1e;
        }
        if !merged.compat_is_orbit_seed {
            merged.compat_is_orbit_seed = previous.compat_is_orbit_seed;
        }
        if merged.seen_year.is_none() {
            merged.seen_year = previous.seen_year;
        }
        if merged.scout_year.is_none() {
            merged.scout_year = previous.scout_year;
        }
    }

    merged.intel_tier = infer_intel_tier_from_snapshot(viewer_empire_id, &merged);
    let compat_year = year.saturating_sub(1);
    merged.last_intel_year = match merged.intel_tier {
        IntelTier::Unknown => None,
        IntelTier::Owned => merged.last_intel_year.or_else(|| {
            previous
                .and_then(|snapshot| snapshot.last_intel_year)
                .map(|_| compat_year)
        }),
        _ => {
            let previous_year = previous.and_then(|snapshot| snapshot.last_intel_year);
            if merged.last_intel_year.is_some() {
                merged.last_intel_year
            } else if previous
                .map(|snapshot| snapshot_fingerprint(snapshot) == snapshot_fingerprint(&merged))
                .unwrap_or(false)
            {
                previous_year.or(Some(compat_year))
            } else {
                Some(compat_year)
            }
        }
    };
    if !matches!(merged.intel_tier, IntelTier::Unknown) {
        merged.seen_year = merged.seen_year.or(match merged.intel_tier {
            IntelTier::Owned => previous
                .and_then(|snapshot| snapshot.seen_year)
                .map(|_| compat_year),
            _ => Some(merged.last_intel_year.unwrap_or(compat_year)),
        });
    }
    if !matches!(merged.intel_tier, IntelTier::Unknown) {
        merged.scout_year = merged.scout_year.or(match merged.intel_tier {
            IntelTier::Owned => previous
                .and_then(|snapshot| snapshot.scout_year)
                .map(|_| compat_year),
            _ => merged.seen_year,
        });
    }

    merged
}

fn viewer_has_fleet_presence(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    coords: [u8; 2],
) -> bool {
    game_data.fleets.records.iter().any(|fleet| {
        fleet.owner_empire_raw() == viewer_empire_id
            && fleet.current_location_coords_raw() == coords
            && fleet_has_any_force(fleet)
    })
}

fn refresh_visible_snapshot_from_runtime(
    snapshot: &PlanetIntelSnapshot,
    planet: &crate::PlanetRecord,
    year: u16,
) -> PlanetIntelSnapshot {
    if matches!(snapshot.intel_tier, IntelTier::Unknown | IntelTier::Owned) {
        return snapshot.clone();
    }

    let compat_year = year.saturating_sub(1);
    let mut refreshed = snapshot.clone();
    refreshed.last_intel_year = Some(compat_year);
    refreshed.seen_year = Some(compat_year);
    if snapshot.scout_year.is_some() && snapshot.scout_year != Some(0) {
        refreshed.scout_year = Some(compat_year);
    }
    refreshed.known_name = Some(planet.status_or_name_summary());
    refreshed.known_owner_empire_id = Some(planet.owner_empire_slot_raw());
    refreshed.known_potential_production = Some(planet.potential_production_points());

    if refreshed.intel_tier == IntelTier::Full {
        refreshed.known_armies = Some(planet.army_count_raw());
        refreshed.known_ground_batteries = Some(planet.ground_batteries_raw());
        refreshed.known_current_production = planet
            .present_production_points()
            .map(|value| value.min(u16::from(u8::MAX)) as u8);
        refreshed.known_stored_points =
            Some(planet.stored_goods_raw().min(u32::from(u16::MAX)) as u16);
    }

    refreshed
}

fn fleet_has_any_force(fleet: &crate::FleetRecord) -> bool {
    fleet.scout_count() > 0
        || fleet.battleship_count() > 0
        || fleet.cruiser_count() > 0
        || fleet.destroyer_count() > 0
        || fleet.troop_transport_count() > 0
        || fleet.army_count() > 0
        || fleet.etac_count() > 0
}

fn snapshot_fingerprint(
    snapshot: &PlanetIntelSnapshot,
) -> (
    IntelTier,
    bool,
    Option<&str>,
    Option<u8>,
    Option<u16>,
    Option<u8>,
    Option<u8>,
    Option<u8>,
    Option<u16>,
    Option<u16>,
    Option<u16>,
    Option<u16>,
) {
    (
        snapshot.intel_tier,
        snapshot.compat_is_orbit_seed,
        snapshot.known_name.as_deref(),
        snapshot.known_owner_empire_id,
        snapshot.known_potential_production,
        snapshot.known_armies,
        snapshot.known_ground_batteries,
        snapshot.known_current_production,
        snapshot.known_stored_points,
        snapshot.seen_year,
        snapshot.scout_year,
        snapshot.compat_word_1e,
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

fn decode_known_owner_empire_id(
    record: &DatabaseRecord,
    game_data: &CoreGameData,
    has_visible_name: bool,
) -> Option<u8> {
    let raw = record.raw[0x15];
    if raw == 0 && has_visible_name {
        Some(0)
    } else if raw >= 1 && raw <= game_data.conquest.player_count() {
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
