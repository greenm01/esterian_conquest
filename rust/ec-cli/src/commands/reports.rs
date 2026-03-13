use std::fs;
use std::path::Path;

use ec_data::{CoreGameData, DatabaseDat, MaintenanceEvents, PlanetDat};

const RESULTS_RECORD_SIZE: usize = 84;

/// Regenerate DATABASE.DAT from current PLANETS.DAT and CONQUEST.DAT year.
///
/// `pre_maint_planets` is the planet state before maintenance ran, used to detect
/// which planets had active build queues (which affects certain DATABASE fields).
pub(crate) fn regenerate_database_dat(
    dir: &Path,
    game_data: &CoreGameData,
    pre_maint_planets: &PlanetDat,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    let template_path = dir.join("DATABASE.DAT");
    let template = if template_path.exists() {
        let bytes = fs::read(&template_path)?;
        DatabaseDat::parse(&bytes).ok()
    } else {
        None
    };

    let planet_names: Vec<String> = game_data
        .planets
        .records
        .iter()
        .map(|p| {
            let name = p.planet_name();
            if name.eq_ignore_ascii_case("unowned") || name.eq_ignore_ascii_case("not named yet") {
                "UNKNOWN".to_string()
            } else {
                name
            }
        })
        .collect();

    let year = game_data.conquest.game_year();
    let discovery_year = year - 1;
    let mut new_database =
        DatabaseDat::generate_from_planets_and_year(&planet_names, year, template.as_ref());

    if let Some(ref template_db) = template {
        let year_bytes = discovery_year.to_le_bytes();

        for player in 0..4usize {
            for planet in 0..20usize {
                let record_idx = player * 20 + planet;
                let template_record = &template_db.records[record_idx];
                let scan_marker = template_record.raw[0x15];
                let is_orbit_record =
                    scan_marker >= 0x01 && scan_marker <= 0x04 && template_record.raw[0x00] == 0;

                let planet_owner = if planet < game_data.planets.records.len() {
                    game_data.planets.records[planet].owner_empire_slot_raw() as usize
                } else {
                    0
                };
                let is_owned_unknown = scan_marker == 0xff && planet_owner == player + 1;

                if is_orbit_record {
                    new_database.records[record_idx].set_planet_name("Not Named Yet");
                    new_database.records[record_idx].raw[0x16] = year_bytes[0];
                    new_database.records[record_idx].raw[0x17] = year_bytes[1];
                    new_database.records[record_idx].raw[0x18] = year_bytes[0];
                    new_database.records[record_idx].raw[0x19] = year_bytes[1];
                    new_database.records[record_idx].raw[0x27] = year_bytes[0];
                    new_database.records[record_idx].raw[0x28] = year_bytes[1];

                    if planet < pre_maint_planets.records.len() {
                        let had_build_queue = (0..10).any(|slot| {
                            pre_maint_planets.records[planet].build_count_raw(slot) > 0
                        });
                        if had_build_queue {
                            new_database.records[record_idx].raw[0x1e] = 0x00;
                        }
                    }

                    if planet < game_data.planets.records.len()
                        && game_data.planets.records[planet].raw[0x03] == 0x87
                        && planet_owner > 0
                        && planet_owner == player + 1
                    {
                        let player_mode = game_data.player.records[player].raw[0x00];
                        let autopilot = game_data.player.records[player].raw[0x6D];
                        let ai_ran =
                            player_mode == 0xff || (player_mode == 0x01 && autopilot == 0x01);
                        if ai_ran {
                            let owner_slot = planet_owner as u8;
                            let armies = game_data.planets.records[planet].army_count_raw();
                            new_database.records[record_idx].raw[0x1e] = 0x40 + owner_slot;
                            new_database.records[record_idx].raw[0x23] = armies;
                        }
                    }
                } else if is_owned_unknown {
                    let owner_slot = planet_owner as u8;
                    let planet_name = if planet < game_data.planets.records.len() {
                        game_data.planets.records[planet].planet_name()
                    } else {
                        String::new()
                    };
                    let is_new_colony = planet_name.eq_ignore_ascii_case("not named yet");

                    new_database.records[record_idx].set_planet_name(&planet_name);
                    new_database.records[record_idx].raw[0x15] =
                        if is_new_colony { 0x01 } else { owner_slot };
                    new_database.records[record_idx].raw[0x16] = year_bytes[0];
                    new_database.records[record_idx].raw[0x17] = year_bytes[1];
                    new_database.records[record_idx].raw[0x18] = year_bytes[0];
                    new_database.records[record_idx].raw[0x19] = year_bytes[1];
                    new_database.records[record_idx].raw[0x27] = year_bytes[0];
                    new_database.records[record_idx].raw[0x28] = year_bytes[1];

                    if planet < game_data.planets.records.len() {
                        let p = &game_data.planets.records[planet];
                        let pot_prod_lo = p.raw[0x02];
                        let armies = p.army_count_raw();
                        let batteries = p.ground_batteries_raw();

                        new_database.records[record_idx].raw[0x1c] = pot_prod_lo;
                        new_database.records[record_idx].raw[0x1d] = if is_new_colony {
                            owner_slot
                        } else {
                            pot_prod_lo
                        };
                        new_database.records[record_idx].raw[0x1e] = if is_new_colony {
                            0x00
                        } else {
                            0x40 + owner_slot
                        };
                        new_database.records[record_idx].raw[0x1f] = 0x00;
                        new_database.records[record_idx].raw[0x23] = armies;
                        new_database.records[record_idx].raw[0x24] = 0x00;
                        new_database.records[record_idx].raw[0x25] = batteries;
                        new_database.records[record_idx].raw[0x26] = 0x00;
                    }
                }
            }
        }
    }

    if let Some(ref _template_db) = template {
        let year_bytes = discovery_year.to_le_bytes();
        for event in &events.planet_intel_events {
            let planet_idx = event.planet_idx;
            if planet_idx >= game_data.planets.records.len() {
                continue;
            }
            let planet = &game_data.planets.records[planet_idx];
            let owner_slot = planet.owner_empire_slot_raw();
            let pot_prod_lo = planet.raw[0x02];
            let armies = planet.army_count_raw();
            let batteries = planet.ground_batteries_raw();
            let name_len = planet.raw[0x0F];
            let planet_name: String = planet.raw[0x10..0x10 + name_len.min(13) as usize]
                .iter()
                .map(|&b| b as char)
                .collect();

            let viewer_player = event.viewer_empire_raw.saturating_sub(1) as usize;
            let update_record = |new_database: &mut DatabaseDat, record_idx: usize| {
                new_database.records[record_idx].set_planet_name(&planet_name);
                new_database.records[record_idx].raw[0x15] = owner_slot;
                new_database.records[record_idx].raw[0x16] = year_bytes[0];
                new_database.records[record_idx].raw[0x17] = year_bytes[1];
                new_database.records[record_idx].raw[0x18] = year_bytes[0];
                new_database.records[record_idx].raw[0x19] = year_bytes[1];
                new_database.records[record_idx].raw[0x1c] = pot_prod_lo;
                new_database.records[record_idx].raw[0x1d] = pot_prod_lo;
                new_database.records[record_idx].raw[0x1e] = 0x23;
                new_database.records[record_idx].raw[0x1f] = 0x00;
                new_database.records[record_idx].raw[0x23] = armies;
                new_database.records[record_idx].raw[0x24] = 0x00;
                new_database.records[record_idx].raw[0x25] = batteries;
                new_database.records[record_idx].raw[0x26] = 0x00;
                new_database.records[record_idx].raw[0x27] = year_bytes[0];
                new_database.records[record_idx].raw[0x28] = year_bytes[1];
            };

            let record_idx = viewer_player * 20 + planet_idx;
            if record_idx < new_database.records.len() {
                update_record(&mut new_database, record_idx);
            }
        }
    }

    fs::write(template_path, new_database.to_bytes())?;
    Ok(())
}

fn empire_label(game_data: &CoreGameData, empire_raw: u8) -> String {
    let idx = empire_raw.saturating_sub(1) as usize;
    let Some(player) = game_data.player.records.get(idx) else {
        return format!("Empire #{empire_raw}");
    };
    let empire = player.controlled_empire_name_summary();
    let handle = player.assigned_player_handle_summary();
    let legacy = player.legacy_status_name_summary();
    if !empire.is_empty() {
        format!("Empire #{empire_raw} \"{empire}\"")
    } else if !handle.is_empty() {
        format!("Empire #{empire_raw} \"{handle}\"")
    } else if !legacy.is_empty() {
        format!("Empire #{empire_raw} \"{legacy}\"")
    } else {
        format!("Empire #{empire_raw}")
    }
}

fn push_results_line(data: &mut Vec<u8>, line: &str) {
    let mut record = [0u8; RESULTS_RECORD_SIZE];
    let bytes = line.as_bytes();
    let len = bytes.len().min(RESULTS_RECORD_SIZE - 1);
    record[0] = 0x08;
    record[1..1 + len].copy_from_slice(&bytes[..len]);
    data.extend_from_slice(&record);
}

pub(crate) fn regenerate_results_dat(
    dir: &Path,
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    for event in &events.bombard_events {
        if let Some(planet) = game_data.planets.records.get(event.planet_idx) {
            let [x, y] = planet.coords_raw();
            push_results_line(
                &mut results,
                &format!(
                    "Bombardment at System({x},{y}) against planet \"{}\" by {}.",
                    planet.planet_name(),
                    empire_label(game_data, event.attacker_empire_raw)
                ),
            );
        }
    }

    for event in &events.fleet_battle_events {
        let participants = event
            .participant_empires_raw
            .iter()
            .map(|empire| empire_label(game_data, *empire))
            .collect::<Vec<_>>()
            .join(", ");
        let [x, y] = event.coords;
        let outcome = match event.winner_empire_raw {
            Some(empire) => format!("winner {}", empire_label(game_data, empire)),
            None => "no clear winner".to_string(),
        };
        push_results_line(
            &mut results,
            &format!("Fleet battle at System({x},{y}): {participants}; {outcome}."),
        );
    }

    for event in &events.ownership_change_events {
        if let Some(planet) = game_data.planets.records.get(event.planet_idx) {
            let [x, y] = planet.coords_raw();
            let from = if event.previous_owner_empire_raw == 0 {
                "unowned world".to_string()
            } else {
                empire_label(game_data, event.previous_owner_empire_raw)
            };
            push_results_line(
                &mut results,
                &format!(
                    "Planet \"{}\" in System({x},{y}) captured by {} from {}.",
                    planet.planet_name(),
                    empire_label(game_data, event.new_owner_empire_raw),
                    from
                ),
            );
        }
    }

    fs::write(dir.join("RESULTS.DAT"), results)?;
    Ok(())
}
