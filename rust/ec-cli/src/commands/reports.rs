use ec_data::{
    ContactReportSource, CoreGameData, DatabaseDat, MaintenanceEvents, Mission, MissionOutcome,
    PlanetDat, QueuedPlayerMail, ShipLosses,
};

const RESULTS_RECORD_SIZE: usize = 84;
const RESULTS_TEXT_SIZE: usize = 75;
const RESULTS_TAIL_BOMBARD: [u8; 8] = [0, 0, 0, 0, 0, 0, 185, 11];
const RESULTS_TAIL_INVASION: [u8; 8] = [0, 0, 0, 0, 0, 0, 195, 11];
const RESULTS_TAIL_FLEET: [u8; 8] = [0, 0, 7, 0, 0, 0, 194, 11];
const RESULTS_TAIL_COLONIZATION: [u8; 8] = [0, 0, 0, 0, 0, 0, 184, 11];
const RESULTS_TAIL_SCOUTING: [u8; 8] = [0, 0, 0, 0, 0, 0, 186, 11];

/// Regenerate DATABASE.DAT from current PLANETS.DAT and CONQUEST.DAT year.
///
/// `pre_maint_planets` is the planet state before maintenance ran, used to detect
/// which planets had active build queues (which affects certain DATABASE fields).
pub(crate) fn build_database_dat(
    game_data: &CoreGameData,
    pre_maint_planets: &PlanetDat,
    events: &MaintenanceEvents,
    template: Option<&DatabaseDat>,
) -> DatabaseDat {
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
    let player_count = game_data.conquest.player_count() as usize;
    let planet_count = game_data.planets.records.len();
    let expected_record_count = player_count * planet_count;
    let template = template
        .filter(|db| db.records.len() == expected_record_count)
        .cloned();
    let template = template.as_ref();
    let mut new_database = DatabaseDat::generate_from_planets_and_year(
        &planet_names,
        year,
        player_count,
        template,
    );

    if let Some(template_db) = template {
        let year_bytes = discovery_year.to_le_bytes();

        for player in 0..player_count {
            for planet in 0..planet_count {
                let record_idx = player * planet_count + planet;
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

    if template.is_some() {
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

            let record_idx = DatabaseDat::record_index(planet_idx, viewer_player, planet_count);
            if record_idx < new_database.records.len() {
                update_record(&mut new_database, record_idx);
            }
        }
    }

    new_database
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

fn push_results_chunked(data: &mut Vec<u8>, kind: u8, tail: [u8; 8], text: &str) {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return;
    }
    for chunk in bytes.chunks(RESULTS_TEXT_SIZE) {
        let mut record = [0u8; RESULTS_RECORD_SIZE];
        record[0] = kind;
        record[1..1 + chunk.len()].copy_from_slice(chunk);
        record[76..84].copy_from_slice(&tail);
        data.extend_from_slice(&record);
    }
}

fn push_routed_message_chunked(
    data: &mut Vec<u8>,
    game_data: &CoreGameData,
    recipient_empire_raw: u8,
    kind: u8,
    tail: [u8; 8],
    text: &str,
) {
    if recipient_empire_raw == 0 {
        return;
    }
    let routed = format!(
        "For {}: {}",
        empire_label(game_data, recipient_empire_raw),
        text
    );
    push_results_chunked(data, kind, tail, &routed);
}

fn mission_location_phrase(kind: Mission, coords: [u8; 2]) -> String {
    let [x, y] = coords;
    match kind {
        Mission::ScoutSector | Mission::MoveOnly => {
            format!("Sector({x},{y})")
        }
        _ => format!("System({x},{y})"),
    }
}

fn mission_report_label(kind: Mission) -> &'static str {
    match kind {
        Mission::GuardStarbase => "Guard Starbase mission report",
        Mission::JoinAnotherFleet => "Join mission report",
        Mission::RendezvousSector => "Rendezvous mission report",
        Mission::GuardBlockadeWorld => "Guard/Blockade World mission report",
        Mission::ViewWorld => "Viewing mission report",
        _ => "Scouting mission report",
    }
}

fn contact_size_summary(event: &ec_data::ScoutContactEvent) -> String {
    match (
        event.large_vessels > 0,
        event.medium_vessels > 0,
        event.small_vessels > 0,
    ) {
        (true, true, true) => format!(
            "{} large, {} medium, and {} small vessel(s)",
            event.large_vessels, event.medium_vessels, event.small_vessels
        ),
        (true, true, false) => format!(
            "{} large and {} medium vessel(s)",
            event.large_vessels, event.medium_vessels
        ),
        (true, false, true) => format!(
            "{} large and {} small vessel(s)",
            event.large_vessels, event.small_vessels
        ),
        (false, true, true) => format!(
            "{} medium and {} small vessel(s)",
            event.medium_vessels, event.small_vessels
        ),
        (true, false, false) => format!("{} large vessel(s)", event.large_vessels),
        (false, true, false) => format!("{} medium vessel(s)", event.medium_vessels),
        (false, false, true) => format!("{} small vessel(s)", event.small_vessels),
        (false, false, false) => "no combat vessels".to_string(),
    }
}

fn coords_system_text(coords: [u8; 2]) -> String {
    let [x, y] = coords;
    format!("System({x},{y})")
}

fn nearest_owned_destination_text(
    game_data: &CoreGameData,
    empire_raw: u8,
    coords: [u8; 2],
) -> String {
    if let Some(planet) = game_data.planets.records.iter().find(|planet| {
        planet.coords_raw() == coords && planet.owner_empire_slot_raw() == empire_raw
    }) {
        format!(
            "planet \"{}\", located in {}",
            planet.planet_name(),
            coords_system_text(coords)
        )
    } else {
        coords_system_text(coords)
    }
}

fn ship_loss_summary(losses: ShipLosses) -> String {
    let mut parts = Vec::new();
    if losses.battleships > 0 {
        parts.push(format!("{} battleship(s)", losses.battleships));
    }
    if losses.cruisers > 0 {
        parts.push(format!("{} cruiser(s)", losses.cruisers));
    }
    if losses.destroyers > 0 {
        parts.push(format!("{} destroyer(s)", losses.destroyers));
    }
    if losses.scouts > 0 {
        parts.push(format!("{} scout ship(s)", losses.scouts));
    }
    if losses.transports > 0 {
        parts.push(format!("{} troop transport(s)", losses.transports));
    }
    if losses.etacs > 0 {
        parts.push(format!("{} ETAC(s)", losses.etacs));
    }
    if parts.is_empty() {
        "no ship losses".to_string()
    } else {
        parts.join(", ")
    }
}

pub(crate) fn build_results_dat(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<u8> {
    let mut results = Vec::new();

    for event in &events.bombard_events {
        if event.defender_empire_raw == 0 {
            continue;
        }
        if let Some(planet) = game_data.planets.records.get(event.planet_idx) {
            let [x, y] = planet.coords_raw();
            let text = format!(
                "From planet \"{}\" in System({x},{y}): Stardate 1/{}. We have been bombarded by {}. We observed losses of {} ground batteries and {} armies.",
                planet.planet_name(),
                game_data.conquest.game_year(),
                empire_label(game_data, event.attacker_empire_raw),
                event.defender_battery_losses,
                event.defender_army_losses,
            );
            push_results_chunked(&mut results, 0x08, RESULTS_TAIL_BOMBARD, &text);
        }
    }

    for event in &events.fleet_battle_events {
        let enemies = event
            .enemy_empires_raw
            .iter()
            .map(|empire| empire_label(game_data, *empire))
            .collect::<Vec<_>>()
            .join(", ");
        let [x, y] = event.coords;
        let outcome = if event.held_field {
            "We held the field.".to_string()
        } else {
            "We were forced to disengage.".to_string()
        };
        let text = format!(
            "From your fleet in System({x},{y}): Fleet battle report. We engaged hostile forces belonging to {enemies}. Friendly losses: {}. Observed enemy losses: {}. {outcome}",
            ship_loss_summary(event.friendly_losses),
            ship_loss_summary(event.enemy_losses),
        );
        push_results_chunked(&mut results, 0x06, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.fleet_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .map(|empire| empire_label(game_data, empire))
            .unwrap_or_else(|| "an alien fleet".to_string());
        let verb = if event.was_intercepting {
            "intercepted"
        } else {
            "was attacked by"
        };
        let text = format!(
            "From your Fleet Command Center: We lost all contact with the {}th Fleet shortly after it {} {} in System({x},{y}). Records show the fleet was composed of {} and carried {} armies. According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            event.fleet_id,
            verb,
            enemy,
            ship_loss_summary(event.friendly_initial),
            event.friendly_armies,
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.enemy_losses),
        );
        push_results_chunked(&mut results, 0x06, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.starbase_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .map(|empire| empire_label(game_data, empire))
            .unwrap_or_else(|| "an alien fleet".to_string());
        let text = format!(
            "From your Fleet Command Center: We lost all contact with Starbase {} shortly after it was attacked by {} in System({x},{y}). According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            event.starbase_id,
            enemy,
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.enemy_losses),
        );
        push_results_chunked(&mut results, 0x06, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.civil_disorder_events {
        let text = format!(
            "From your Fleet Command Center: With all of our controlled worlds lost and no immediate means of recovery, the empire of \"{}\" has fallen into civil disorder. Remaining forces are scattered and unreliable.",
            event.prior_label,
        );
        push_results_chunked(&mut results, 0x06, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.campaign_outlook_events {
        let text = format!(
            "From your Fleet Command Center: {} now stands as the sole remaining serious contender for the imperial throne. Other empires may still persist, but none currently appear capable of challenging our claim.",
            empire_label(game_data, event.empire_raw),
        );
        push_results_chunked(&mut results, 0x06, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.campaign_outcome_events {
        let text = format!(
            "From your Fleet Command Center: {} has now been recognized as Emperor. No other stable empire remains capable of contesting the throne.",
            empire_label(game_data, event.emperor_empire_raw),
        );
        push_results_chunked(&mut results, 0x06, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.fleet_defection_events {
        let text = format!(
            "From your Fleet Command Center: We have lost all contact with the {}th Fleet. In the chaos of civil disorder, the surviving crews have defected and no longer answer to central command.",
            event.fleet_id,
        );
        push_results_chunked(&mut results, 0x06, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.assault_report_events {
        let Some(planet) = game_data.planets.records.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.coords_raw();
        let ship_losses = ship_loss_summary(event.attacker_ship_losses);
        let transport_note = if event.transport_army_losses > 0 {
            format!(
                " {} troop(s) died in destroyed troop transports during the landing.",
                event.transport_army_losses
            )
        } else {
            String::new()
        };
        let blitz_cover_note = if event.defender_battery_losses > 0 {
            format!(
                " Our escorting ships briefly suppressed {} ground batteries before the descent.",
                event.defender_battery_losses
            )
        } else {
            " Our cover fire failed to suppress the defending batteries before the descent."
                .to_string()
        };
        let text = match (event.kind, event.outcome) {
            (Mission::InvadeWorld, MissionOutcome::Succeeded) => format!(
                "From your fleet in System({x},{y}): Invasion mission report: Our armies have captured planet \"{}\". Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet.planet_name(),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Failed) => format!(
                "From your fleet in System({x},{y}): Invasion mission report: The landing was repulsed. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Aborted) => format!(
                "From your fleet in System({x},{y}): Invasion mission report: Enemy ground batteries prevented a landing. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::BlitzWorld, MissionOutcome::Succeeded) => format!(
                "From your fleet in System({x},{y}): Blitz mission report: We have seized planet \"{}\" in a fast assault.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                planet.planet_name(),
                blitz_cover_note,
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
                transport_note,
            ),
            (Mission::BlitzWorld, MissionOutcome::Failed) => format!(
                "From your fleet in System({x},{y}): Blitz mission report: The blitz attack failed.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                blitz_cover_note,
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
                transport_note,
            ),
            _ => continue,
        };
        push_results_chunked(&mut results, 0x0c, RESULTS_TAIL_INVASION, &text);
    }

    for event in &events.scout_contact_events {
        let [x, y] = event.coords;
        let size_summary = contact_size_summary(event);
        match event.source {
            ContactReportSource::FleetMission(kind) => {
                let label = mission_report_label(kind);
                let contact_text = format!(
                    "From your fleet in System({x},{y}): {label}: Sensor contact shows an alien fleet in System({x},{y}) traveling at sublight speed. Closing to check it out..."
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &contact_text);

                let identified_text = format!(
                    "From your fleet in System({x},{y}): {label}: We have located and identified the alien fleet in System({x},{y}). It belongs to {}. Their fleet contains {size_summary} of unknown type. Ignoring alien fleet...",
                    empire_label(game_data, event.target_empire_raw),
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &identified_text);
            }
            ContactReportSource::Fleet(fleet_id) => {
                let identified_text = format!(
                    "From Fleet {fleet_id} in System({x},{y}): Contact report: We have encountered an alien fleet in System({x},{y}). It belongs to {}. Their fleet contains {size_summary} of unknown type.",
                    empire_label(game_data, event.target_empire_raw),
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &identified_text);
            }
            ContactReportSource::Starbase(starbase_id) => {
                let identified_text = format!(
                    "From Starbase {starbase_id}, located in System({x},{y}): We have located and identified an alien fleet in System({x},{y}). It is {}. Their fleet contains {size_summary} of unknown type. We are alerting all fleets in the area.",
                    empire_label(game_data, event.target_empire_raw),
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &identified_text);
            }
        }
    }

    for event in &events.ownership_change_events {
        if event.reporting_empire_raw == 0 {
            continue;
        }
        if let Some(planet) = game_data.planets.records.get(event.planet_idx) {
            let [x, y] = planet.coords_raw();
            let from = if event.previous_owner_empire_raw == 0 {
                "unowned world".to_string()
            } else {
                empire_label(game_data, event.previous_owner_empire_raw)
            };
            let text = format!(
                "From planet \"{}\" in System({x},{y}): We have been invaded and captured by {} from {}.",
                planet.planet_name(),
                empire_label(game_data, event.new_owner_empire_raw),
                from
            );
            push_results_chunked(&mut results, 0x0c, RESULTS_TAIL_INVASION, &text);
        }
    }

    for event in &events.colonization_events {
        match *event {
            ec_data::ColonizationResolvedEvent::Succeeded {
                planet_idx,
                colonizer_empire_raw,
                ..
            } => {
                if let Some(planet) = game_data.planets.records.get(planet_idx) {
                    let [x, y] = planet.coords_raw();
                    let text = format!(
                        "From colony mission in System({x},{y}): We have successfully established a colony on planet \"{}\" for {}.",
                        planet.planet_name(),
                        empire_label(game_data, colonizer_empire_raw),
                    );
                    push_results_chunked(&mut results, 0x09, RESULTS_TAIL_COLONIZATION, &text);
                }
            }
            ec_data::ColonizationResolvedEvent::BlockedByOwner {
                planet_idx,
                colonizer_empire_raw,
                owner_empire_raw,
                ..
            } => {
                if let Some(planet) = game_data.planets.records.get(planet_idx) {
                    let [x, y] = planet.coords_raw();
                    let text = format!(
                        "From colony mission in System({x},{y}): {} could not establish a colony on planet \"{}\" because it is already occupied by {}.",
                        empire_label(game_data, colonizer_empire_raw),
                        planet.planet_name(),
                        empire_label(game_data, owner_empire_raw),
                    );
                    push_results_chunked(&mut results, 0x09, RESULTS_TAIL_COLONIZATION, &text);
                }
            }
        }
    }

    for event in &events.mission_events {
        let Some(fleet) = game_data.fleets.records.get(event.fleet_idx) else {
            continue;
        };
        let coords = event
            .location_coords
            .unwrap_or_else(|| fleet.current_location_coords_raw());
        let [x, y] = coords;
        match (event.kind, event.outcome) {
            (Mission::MoveOnly, MissionOutcome::Succeeded) => {
                let text = format!(
                    "From your fleet in {}: Move mission report: We have arrived at our destination and await new orders.",
                    mission_location_phrase(event.kind, coords)
                );
                push_results_chunked(&mut results, 0x05, RESULTS_TAIL_FLEET, &text);
            }
            (Mission::RendezvousSector, MissionOutcome::Succeeded) => {
                let text = format!(
                    "From your fleet in Sector({x},{y}): Rendezvous mission report: We have arrived at the our rendezvous point and are waiting for more fleets to arrive."
                );
                push_results_chunked(&mut results, 0x05, RESULTS_TAIL_FLEET, &text);
            }
            (Mission::GuardStarbase, MissionOutcome::Succeeded) => {
                let starbase_text = game_data
                    .bases
                    .records
                    .iter()
                    .find(|base| {
                        base.coords_raw() == coords
                            && base.owner_empire_raw() == event.owner_empire_raw
                            && base.active_flag_raw() != 0
                    })
                    .map(|base| format!("Starbase {}", base.base_id_raw()))
                    .unwrap_or_else(|| "the assigned starbase".to_string());
                let text = format!(
                    "From your fleet in System({x},{y}): Guard Starbase mission report: We have arrived at {starbase_text} and are beginning our guard/escort mission."
                );
                push_results_chunked(&mut results, 0x05, RESULTS_TAIL_FLEET, &text);
            }
            (Mission::GuardBlockadeWorld, MissionOutcome::Succeeded) => {
                let text = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        format!(
                            "From your fleet in System({x},{y}): Guard/Blockade World mission report: We have arrived at planet \"{}\" in Sector({x},{y}) and are beginning our guarding/blockading assignment.",
                            planet.planet_name(),
                        )
                    } else {
                        format!(
                            "From your fleet in System({x},{y}): Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment."
                        )
                    }
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment."
                    )
                };
                push_results_chunked(&mut results, 0x05, RESULTS_TAIL_FLEET, &text);
            }
            (Mission::MoveOnly, MissionOutcome::Aborted) => {
                let destination = fleet.standing_order_target_coords_raw();
                let [dx, dy] = destination;
                let text = format!(
                    "From your fleet in {}: Move mission report: Hostile action forced us to abort our mission and seek safety in System({dx},{dy}).",
                    mission_location_phrase(event.kind, coords)
                );
                push_results_chunked(&mut results, 0x05, RESULTS_TAIL_FLEET, &text);
            }
            (Mission::ViewWorld, MissionOutcome::Succeeded) => {
                let text = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        let ownership = if planet.owner_empire_slot_raw() == 0 {
                            "unowned".to_string()
                        } else {
                            format!(
                                "owned by {}",
                                empire_label(game_data, planet.owner_empire_slot_raw())
                            )
                        };
                        format!(
                            "From your fleet in System({x},{y}): Viewing mission report: We have entered System({x},{y}) and completed a long range analysis of planet \"{}\". The world is {} and has a potential of {} points. Until ordered otherwise, we will be moving out of the solar system.",
                            planet.planet_name(),
                            ownership,
                            u16::from_le_bytes(planet.potential_production_raw()),
                        )
                    } else {
                        format!(
                            "From your fleet in System({x},{y}): Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis."
                        )
                    }
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis."
                    )
                };
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &text);
            }
            (Mission::ViewWorld, MissionOutcome::Failed) => {
                let text = format!(
                    "From your fleet in System({x},{y}): Viewing mission report: We found no world to analyze at the assigned destination."
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &text);
            }
            (Mission::ViewWorld, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| {
                        nearest_owned_destination_text(game_data, event.owner_empire_raw, coords)
                    })
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                let text = format!(
                    "From your fleet in System({x},{y}): Viewing mission report: We were attacked before the viewing mission could be completed. We are aborting our assignment and seeking safety at {retreat}."
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &text);
            }
            (Mission::BombardWorld, MissionOutcome::Succeeded) => {
                let bombard_event = events.bombard_events.iter().find(|bombard| {
                    bombard.planet_idx == event.planet_idx.unwrap_or(usize::MAX)
                        && bombard.attacker_empire_raw == event.owner_empire_raw
                });
                let text = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        format!(
                            "From your fleet in System({x},{y}): Bombardment mission report: We have concluded our bombing run against planet \"{}\". Friendly losses: {}. Observed enemy losses: {} ground batteries and {} armies.",
                            planet.planet_name(),
                            bombard_event
                                .map(|e| ship_loss_summary(e.attacker_losses))
                                .unwrap_or_else(|| "no ship losses".to_string()),
                            bombard_event
                                .map(|e| e.defender_battery_losses)
                                .unwrap_or(0),
                            bombard_event.map(|e| e.defender_army_losses).unwrap_or(0),
                        )
                    } else {
                        format!(
                            "From your fleet in System({x},{y}): Bombardment mission report: We have concluded our bombing run and are awaiting new orders."
                        )
                    }
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Bombardment mission report: We have concluded our bombing run and are awaiting new orders."
                    )
                };
                push_results_chunked(&mut results, 0x08, RESULTS_TAIL_BOMBARD, &text);
            }
            (Mission::InvadeWorld, _) | (Mission::BlitzWorld, _) => {}
            (Mission::ScoutSector, MissionOutcome::Succeeded) => {
                let text = format!(
                    "From your fleet in Sector({x},{y}): Scouting mission report: We have arrived at our destination and are beginning to scout this sector."
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &text);
            }
            (Mission::ScoutSector, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| {
                        nearest_owned_destination_text(game_data, event.owner_empire_raw, coords)
                    })
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                let text = format!(
                    "From your fleet in Sector({x},{y}): Scouting mission report: Hostile action forced us to abort our scouting mission and withdraw toward {retreat}."
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &text);
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Succeeded) => {
                let text = if let Some(planet) = game_data
                    .planets
                    .records
                    .iter()
                    .find(|planet| planet.coords_raw() == [x, y])
                {
                    let owner = if planet.owner_empire_slot_raw() == 0 {
                        "Unowned world".to_string()
                    } else {
                        empire_label(game_data, planet.owner_empire_slot_raw())
                    };
                    let stardock_summary =
                        if (0..10).any(|slot| planet.stardock_count_raw(slot) > 0) {
                            "The planet's stardock contains ships."
                        } else {
                            "The planet's stardock appears to be empty."
                        };
                    format!(
                        "From your fleet in System({x},{y}): Scouting mission report: We are in extended orbit around planet \"{}\". Owner: {}. Potential production: {} points. Stored goods: {} points. Armies: {}. Ground batteries: {}. {}",
                        planet.planet_name(),
                        owner,
                        planet.potential_production_raw()[0],
                        planet.stored_goods_raw(),
                        planet.army_count_raw(),
                        planet.ground_batteries_raw(),
                        stardock_summary,
                    )
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Scouting mission report: We have arrived at our destination and are beginning to scout this solar system."
                    )
                };
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &text);
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| {
                        nearest_owned_destination_text(game_data, event.owner_empire_raw, coords)
                    })
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                let text = format!(
                    "From your fleet in System({x},{y}): Scouting mission report: We were forced to break off our close reconnaissance and withdraw toward {retreat}."
                );
                push_results_chunked(&mut results, 0x07, RESULTS_TAIL_SCOUTING, &text);
            }
            _ => {}
        }
    }

    for event in &events.fleet_merge_events {
        let [x, y] = event.coords;
        let text = match event.kind {
            Mission::JoinAnotherFleet => format!(
                "From your fleet in System({x},{y}): Join mission report: We have joined the {}th Fleet and are now merging with them.",
                event.host_fleet_id
            ),
            Mission::RendezvousSector if event.survivor_side => format!(
                "From your fleet in Sector({x},{y}): Rendezvous mission report: We have arrived at the our rendezvous point and are absorbing the {}th Fleet.",
                event.absorbed_fleet_id
            ),
            Mission::RendezvousSector => format!(
                "From your fleet in Sector({x},{y}): Rendezvous mission report: We have arrived at the our rendezvous point and are merging with the {}th Fleet.",
                event.host_fleet_id
            ),
            _ => continue,
        };
        push_results_chunked(&mut results, 0x05, RESULTS_TAIL_FLEET, &text);
    }

    for event in &events.join_host_events {
        let text = match *event {
            ec_data::JoinMissionHostEvent::Retargeted {
                previous_host_fleet_id,
                new_host_fleet_id,
                coords,
                ..
            } => {
                let [x, y] = coords;
                format!(
                    "From your fleet in Sector({x},{y}): Join mission report: Since the {previous_host_fleet_id}th Fleet has merged with the {new_host_fleet_id}th Fleet, we are now attempting to join the {new_host_fleet_id}th Fleet."
                )
            }
            ec_data::JoinMissionHostEvent::HostDestroyed {
                destroyed_host_fleet_id,
                coords,
                ..
            } => {
                let [x, y] = coords;
                format!(
                    "From your fleet in Sector({x},{y}): Join mission report: In light of the destruction of the {destroyed_host_fleet_id}th Fleet, we are holding our current position in Sector({x},{y}) and are awaiting new orders."
                )
            }
        };
        push_results_chunked(&mut results, 0x05, RESULTS_TAIL_FLEET, &text);
    }

    results
}

pub(crate) fn build_messages_dat(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
    queued_mail: &[QueuedPlayerMail],
    existing_messages: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut messages = Vec::new();

    for event in &events.bombard_events {
        if event.defender_empire_raw == 0 {
            continue;
        }
        if let Some(planet) = game_data.planets.records.get(event.planet_idx) {
            let [x, y] = planet.coords_raw();
            let text = format!(
                "From planet \"{}\" in System({x},{y}): Stardate 1/{}. We have been bombarded by {}. We observed losses of {} ground batteries and {} armies.",
                planet.planet_name(),
                game_data.conquest.game_year(),
                empire_label(game_data, event.attacker_empire_raw),
                event.defender_battery_losses,
                event.defender_army_losses,
            );
            push_routed_message_chunked(
                &mut messages,
                game_data,
                event.defender_empire_raw,
                0x08,
                RESULTS_TAIL_BOMBARD,
                &text,
            );
        }
    }

    for event in &events.fleet_battle_events {
        let enemies = event
            .enemy_empires_raw
            .iter()
            .map(|empire| empire_label(game_data, *empire))
            .collect::<Vec<_>>()
            .join(", ");
        let [x, y] = event.coords;
        let outcome = if event.held_field {
            "We held the field.".to_string()
        } else {
            "We were forced to disengage.".to_string()
        };
        let text = format!(
            "From your fleet in System({x},{y}): Fleet battle report. We engaged hostile forces belonging to {enemies}. Friendly losses: {}. Observed enemy losses: {}. {outcome}",
            ship_loss_summary(event.friendly_losses),
            ship_loss_summary(event.enemy_losses),
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.reporting_empire_raw,
            0x06,
            RESULTS_TAIL_FLEET,
            &text,
        );
    }

    for event in &events.fleet_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .map(|empire| empire_label(game_data, empire))
            .unwrap_or_else(|| "an alien fleet".to_string());
        let verb = if event.was_intercepting {
            "intercepted"
        } else {
            "was attacked by"
        };
        let text = format!(
            "From your Fleet Command Center: We lost all contact with the {}th Fleet shortly after it {} {} in System({x},{y}). Records show the fleet was composed of {} and carried {} armies. According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            event.fleet_id,
            verb,
            enemy,
            ship_loss_summary(event.friendly_initial),
            event.friendly_armies,
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.enemy_losses),
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.reporting_empire_raw,
            0x06,
            RESULTS_TAIL_FLEET,
            &text,
        );
    }

    for event in &events.starbase_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .map(|empire| empire_label(game_data, empire))
            .unwrap_or_else(|| "an alien fleet".to_string());
        let text = format!(
            "From your Fleet Command Center: We lost all contact with Starbase {} shortly after it was attacked by {} in System({x},{y}). According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            event.starbase_id,
            enemy,
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.enemy_losses),
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.reporting_empire_raw,
            0x06,
            RESULTS_TAIL_FLEET,
            &text,
        );
    }

    for event in &events.civil_disorder_events {
        let text = format!(
            "From your Fleet Command Center: With all of our controlled worlds lost and no immediate means of recovery, the empire of \"{}\" has fallen into civil disorder. Remaining forces are scattered and unreliable.",
            event.prior_label,
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.reporting_empire_raw,
            0x06,
            RESULTS_TAIL_FLEET,
            &text,
        );
    }

    for event in &events.fleet_defection_events {
        let text = format!(
            "From your Fleet Command Center: We have lost all contact with the {}th Fleet. In the chaos of civil disorder, the surviving crews have defected and no longer answer to central command.",
            event.fleet_id,
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.reporting_empire_raw,
            0x06,
            RESULTS_TAIL_FLEET,
            &text,
        );
    }

    for event in &events.assault_report_events {
        let Some(planet) = game_data.planets.records.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.coords_raw();
        let ship_losses = ship_loss_summary(event.attacker_ship_losses);
        let transport_note = if event.transport_army_losses > 0 {
            format!(
                " {} troop(s) died in destroyed troop transports during the landing.",
                event.transport_army_losses
            )
        } else {
            String::new()
        };
        let blitz_cover_note = if event.defender_battery_losses > 0 {
            format!(
                " Our escorting ships briefly suppressed {} ground batteries before the descent.",
                event.defender_battery_losses
            )
        } else {
            " Our cover fire failed to suppress the defending batteries before the descent."
                .to_string()
        };
        let text = match (event.kind, event.outcome) {
            (Mission::InvadeWorld, MissionOutcome::Succeeded) => format!(
                "From your fleet in System({x},{y}): Invasion mission report: Our armies have captured planet \"{}\". Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet.planet_name(),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Failed) => format!(
                "From your fleet in System({x},{y}): Invasion mission report: The landing was repulsed. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Aborted) => format!(
                "From your fleet in System({x},{y}): Invasion mission report: Enemy ground batteries prevented a landing. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::BlitzWorld, MissionOutcome::Succeeded) => format!(
                "From your fleet in System({x},{y}): Blitz mission report: We have seized planet \"{}\" in a fast assault.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                planet.planet_name(),
                blitz_cover_note,
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
                transport_note,
            ),
            (Mission::BlitzWorld, MissionOutcome::Failed) => format!(
                "From your fleet in System({x},{y}): Blitz mission report: The blitz attack failed.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                blitz_cover_note,
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
                transport_note,
            ),
            _ => continue,
        };
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.attacker_empire_raw,
            0x0c,
            RESULTS_TAIL_INVASION,
            &text,
        );
    }

    for event in &events.scout_contact_events {
        let [x, y] = event.coords;
        let size_summary = contact_size_summary(event);
        match event.source {
            ContactReportSource::FleetMission(kind) => {
                let label = mission_report_label(kind);
                let contact_text = format!(
                    "From your fleet in System({x},{y}): {label}: Sensor contact shows an alien fleet in System({x},{y}) traveling at sublight speed. Closing to check it out..."
                );
                push_routed_message_chunked(
                    &mut messages,
                    game_data,
                    event.viewer_empire_raw,
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    &contact_text,
                );

                let identified_text = format!(
                    "From your fleet in System({x},{y}): {label}: We have located and identified the alien fleet in System({x},{y}). It belongs to {}. Their fleet contains {size_summary} of unknown type. Ignoring alien fleet...",
                    empire_label(game_data, event.target_empire_raw),
                );
                push_routed_message_chunked(
                    &mut messages,
                    game_data,
                    event.viewer_empire_raw,
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    &identified_text,
                );
            }
            ContactReportSource::Fleet(fleet_id) => {
                let identified_text = format!(
                    "From Fleet {fleet_id} in System({x},{y}): Contact report: We have encountered an alien fleet in System({x},{y}). It belongs to {}. Their fleet contains {size_summary} of unknown type.",
                    empire_label(game_data, event.target_empire_raw),
                );
                push_routed_message_chunked(
                    &mut messages,
                    game_data,
                    event.viewer_empire_raw,
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    &identified_text,
                );
            }
            ContactReportSource::Starbase(starbase_id) => {
                let identified_text = format!(
                    "From Starbase {starbase_id}, located in System({x},{y}): We have located and identified an alien fleet in System({x},{y}). It is {}. Their fleet contains {size_summary} of unknown type. We are alerting all fleets in the area.",
                    empire_label(game_data, event.target_empire_raw),
                );
                push_routed_message_chunked(
                    &mut messages,
                    game_data,
                    event.viewer_empire_raw,
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    &identified_text,
                );
            }
        }
    }

    for event in &events.ownership_change_events {
        if event.reporting_empire_raw == 0 {
            continue;
        }
        if let Some(planet) = game_data.planets.records.get(event.planet_idx) {
            let [x, y] = planet.coords_raw();
            let from = if event.previous_owner_empire_raw == 0 {
                "unowned world".to_string()
            } else {
                empire_label(game_data, event.previous_owner_empire_raw)
            };
            let text = format!(
                "From planet \"{}\" in System({x},{y}): We have been invaded and captured by {} from {}.",
                planet.planet_name(),
                empire_label(game_data, event.new_owner_empire_raw),
                from
            );
            push_routed_message_chunked(
                &mut messages,
                game_data,
                event.reporting_empire_raw,
                0x0c,
                RESULTS_TAIL_INVASION,
                &text,
            );
        }
    }

    for event in &events.colonization_events {
        match *event {
            ec_data::ColonizationResolvedEvent::Succeeded {
                planet_idx,
                colonizer_empire_raw,
                ..
            } => {
                if let Some(planet) = game_data.planets.records.get(planet_idx) {
                    let [x, y] = planet.coords_raw();
                    let text = format!(
                        "From colony mission in System({x},{y}): We have successfully established a colony on planet \"{}\" for {}.",
                        planet.planet_name(),
                        empire_label(game_data, colonizer_empire_raw),
                    );
                    push_routed_message_chunked(
                        &mut messages,
                        game_data,
                        colonizer_empire_raw,
                        0x09,
                        RESULTS_TAIL_COLONIZATION,
                        &text,
                    );
                }
            }
            ec_data::ColonizationResolvedEvent::BlockedByOwner {
                planet_idx,
                colonizer_empire_raw,
                owner_empire_raw,
                ..
            } => {
                if let Some(planet) = game_data.planets.records.get(planet_idx) {
                    let [x, y] = planet.coords_raw();
                    let text = format!(
                        "From colony mission in System({x},{y}): {} could not establish a colony on planet \"{}\" because it is already occupied by {}.",
                        empire_label(game_data, colonizer_empire_raw),
                        planet.planet_name(),
                        empire_label(game_data, owner_empire_raw),
                    );
                    push_routed_message_chunked(
                        &mut messages,
                        game_data,
                        colonizer_empire_raw,
                        0x09,
                        RESULTS_TAIL_COLONIZATION,
                        &text,
                    );
                }
            }
        }
    }

    for event in &events.mission_events {
        let Some(fleet) = game_data.fleets.records.get(event.fleet_idx) else {
            continue;
        };
        let coords = event
            .location_coords
            .unwrap_or_else(|| fleet.current_location_coords_raw());
        let [x, y] = coords;
        let (kind, tail, text) = match (event.kind, event.outcome) {
            (Mission::MoveOnly, MissionOutcome::Succeeded) => (
                0x05,
                RESULTS_TAIL_FLEET,
                format!(
                    "From your fleet in {}: Move mission report: We have arrived at our destination and await new orders.",
                    mission_location_phrase(event.kind, coords)
                ),
            ),
            (Mission::RendezvousSector, MissionOutcome::Succeeded) => (
                0x05,
                RESULTS_TAIL_FLEET,
                format!(
                    "From your fleet in Sector({x},{y}): Rendezvous mission report: We have arrived at the our rendezvous point and are waiting for more fleets to arrive."
                ),
            ),
            (Mission::GuardStarbase, MissionOutcome::Succeeded) => {
                let starbase_text = game_data
                    .bases
                    .records
                    .iter()
                    .find(|base| {
                        base.coords_raw() == coords
                            && base.owner_empire_raw() == event.owner_empire_raw
                            && base.active_flag_raw() != 0
                    })
                    .map(|base| format!("Starbase {}", base.base_id_raw()))
                    .unwrap_or_else(|| "the assigned starbase".to_string());
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    format!(
                        "From your fleet in System({x},{y}): Guard Starbase mission report: We have arrived at {starbase_text} and are beginning our guard/escort mission."
                    ),
                )
            }
            (Mission::GuardBlockadeWorld, MissionOutcome::Succeeded) => {
                let text = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        format!(
                            "From your fleet in System({x},{y}): Guard/Blockade World mission report: We have arrived at planet \"{}\" in Sector({x},{y}) and are beginning our guarding/blockading assignment.",
                            planet.planet_name(),
                        )
                    } else {
                        format!(
                            "From your fleet in System({x},{y}): Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment."
                        )
                    }
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment."
                    )
                };
                (0x05, RESULTS_TAIL_FLEET, text)
            }
            (Mission::MoveOnly, MissionOutcome::Aborted) => {
                let destination = fleet.standing_order_target_coords_raw();
                let [dx, dy] = destination;
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    format!(
                        "From your fleet in {}: Move mission report: Hostile action forced us to abort our mission and seek safety in System({dx},{dy}).",
                        mission_location_phrase(event.kind, coords)
                    ),
                )
            }
            (Mission::ViewWorld, MissionOutcome::Succeeded) => {
                let text = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        let ownership = if planet.owner_empire_slot_raw() == 0 {
                            "unowned".to_string()
                        } else {
                            format!(
                                "owned by {}",
                                empire_label(game_data, planet.owner_empire_slot_raw())
                            )
                        };
                        format!(
                            "From your fleet in System({x},{y}): Viewing mission report: We have entered System({x},{y}) and completed a long range analysis of planet \"{}\". The world is {} and has a potential of {} points. Until ordered otherwise, we will be moving out of the solar system.",
                            planet.planet_name(),
                            ownership,
                            u16::from_le_bytes(planet.potential_production_raw()),
                        )
                    } else {
                        format!(
                            "From your fleet in System({x},{y}): Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis."
                        )
                    }
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis."
                    )
                };
                (0x07, RESULTS_TAIL_SCOUTING, text)
            }
            (Mission::ViewWorld, MissionOutcome::Failed) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                format!(
                    "From your fleet in System({x},{y}): Viewing mission report: We found no world to analyze at the assigned destination."
                ),
            ),
            (Mission::ViewWorld, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| {
                        nearest_owned_destination_text(game_data, event.owner_empire_raw, coords)
                    })
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    format!(
                        "From your fleet in System({x},{y}): Viewing mission report: We were attacked before the viewing mission could be completed. We are aborting our assignment and seeking safety at {retreat}."
                    ),
                )
            }
            (Mission::BombardWorld, MissionOutcome::Succeeded) => {
                let bombard_event = events.bombard_events.iter().find(|bombard| {
                    bombard.planet_idx == event.planet_idx.unwrap_or(usize::MAX)
                        && bombard.attacker_empire_raw == event.owner_empire_raw
                });
                let text = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        format!(
                            "From your fleet in System({x},{y}): Bombardment mission report: We have concluded our bombing run against planet \"{}\". Friendly losses: {}. Observed enemy losses: {} ground batteries and {} armies.",
                            planet.planet_name(),
                            bombard_event
                                .map(|e| ship_loss_summary(e.attacker_losses))
                                .unwrap_or_else(|| "no ship losses".to_string()),
                            bombard_event
                                .map(|e| e.defender_battery_losses)
                                .unwrap_or(0),
                            bombard_event.map(|e| e.defender_army_losses).unwrap_or(0),
                        )
                    } else {
                        format!(
                            "From your fleet in System({x},{y}): Bombardment mission report: We have concluded our bombing run and are awaiting new orders."
                        )
                    }
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Bombardment mission report: We have concluded our bombing run and are awaiting new orders."
                    )
                };
                (0x08, RESULTS_TAIL_BOMBARD, text)
            }
            (Mission::InvadeWorld, _) | (Mission::BlitzWorld, _) => continue,
            (Mission::ScoutSector, MissionOutcome::Succeeded) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                format!(
                    "From your fleet in Sector({x},{y}): Scouting mission report: We have arrived at our destination and are beginning to scout this sector."
                ),
            ),
            (Mission::ScoutSector, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| {
                        nearest_owned_destination_text(game_data, event.owner_empire_raw, coords)
                    })
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    format!(
                        "From your fleet in Sector({x},{y}): Scouting mission report: Hostile action forced us to abort our scouting mission and withdraw toward {retreat}."
                    ),
                )
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Succeeded) => {
                let text = if let Some(planet) = game_data
                    .planets
                    .records
                    .iter()
                    .find(|planet| planet.coords_raw() == [x, y])
                {
                    let owner = if planet.owner_empire_slot_raw() == 0 {
                        "Unowned world".to_string()
                    } else {
                        empire_label(game_data, planet.owner_empire_slot_raw())
                    };
                    let stardock_summary =
                        if (0..10).any(|slot| planet.stardock_count_raw(slot) > 0) {
                            "The planet's stardock contains ships."
                        } else {
                            "The planet's stardock appears to be empty."
                        };
                    format!(
                        "From your fleet in System({x},{y}): Scouting mission report: We are in extended orbit around planet \"{}\". Owner: {}. Potential production: {} points. Stored goods: {} points. Armies: {}. Ground batteries: {}. {}",
                        planet.planet_name(),
                        owner,
                        planet.potential_production_raw()[0],
                        planet.stored_goods_raw(),
                        planet.army_count_raw(),
                        planet.ground_batteries_raw(),
                        stardock_summary,
                    )
                } else {
                    format!(
                        "From your fleet in System({x},{y}): Scouting mission report: We have arrived at our destination and are beginning to scout this solar system."
                    )
                };
                (0x07, RESULTS_TAIL_SCOUTING, text)
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| {
                        nearest_owned_destination_text(game_data, event.owner_empire_raw, coords)
                    })
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    format!(
                        "From your fleet in System({x},{y}): Scouting mission report: We were forced to break off our close reconnaissance and withdraw toward {retreat}."
                    ),
                )
            }
            _ => continue,
        };
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.owner_empire_raw,
            kind,
            tail,
            &text,
        );
    }

    for event in &events.fleet_merge_events {
        let [x, y] = event.coords;
        let text = match event.kind {
            Mission::JoinAnotherFleet => format!(
                "From your fleet in System({x},{y}): Join mission report: We have joined the {}th Fleet and are now merging with them.",
                event.host_fleet_id
            ),
            Mission::RendezvousSector if event.survivor_side => format!(
                "From your fleet in Sector({x},{y}): Rendezvous mission report: We have arrived at the our rendezvous point and are absorbing the {}th Fleet.",
                event.absorbed_fleet_id
            ),
            Mission::RendezvousSector => format!(
                "From your fleet in Sector({x},{y}): Rendezvous mission report: We have arrived at the our rendezvous point and are merging with the {}th Fleet.",
                event.host_fleet_id
            ),
            _ => continue,
        };
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.owner_empire_raw,
            0x05,
            RESULTS_TAIL_FLEET,
            &text,
        );
    }

    for event in &events.join_host_events {
        let (recipient, text) = match *event {
            ec_data::JoinMissionHostEvent::Retargeted {
                owner_empire_raw,
                previous_host_fleet_id,
                new_host_fleet_id,
                coords,
                ..
            } => {
                let [x, y] = coords;
                (
                    owner_empire_raw,
                    format!(
                        "From your fleet in Sector({x},{y}): Join mission report: Since the {previous_host_fleet_id}th Fleet has merged with the {new_host_fleet_id}th Fleet, we are now attempting to join the {new_host_fleet_id}th Fleet."
                    ),
                )
            }
            ec_data::JoinMissionHostEvent::HostDestroyed {
                owner_empire_raw,
                destroyed_host_fleet_id,
                coords,
                ..
            } => {
                let [x, y] = coords;
                (
                    owner_empire_raw,
                    format!(
                        "From your fleet in Sector({x},{y}): Join mission report: In light of the destruction of the {destroyed_host_fleet_id}th Fleet, we are holding our current position in Sector({x},{y}) and are awaiting new orders."
                    ),
                )
            }
        };
        push_routed_message_chunked(
            &mut messages,
            game_data,
            recipient,
            0x05,
            RESULTS_TAIL_FLEET,
            &text,
        );
    }

    for event in &events.diplomatic_escalation_events {
        let left_text = format!(
            "From your Fleet Command Center: Hostile action has escalated our relations with {} to enemy status.",
            empire_label(game_data, event.right_empire_raw),
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.left_empire_raw,
            0x06,
            RESULTS_TAIL_FLEET,
            &left_text,
        );

        let right_text = format!(
            "From your Fleet Command Center: Hostile action has escalated our relations with {} to enemy status.",
            empire_label(game_data, event.left_empire_raw),
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            event.right_empire_raw,
            0x06,
            RESULTS_TAIL_FLEET,
            &right_text,
        );
    }

    for mail in queued_mail {
        let subject = if mail.subject.trim().is_empty() {
            "No Subject".to_string()
        } else {
            mail.subject.trim().to_string()
        };
        let text = format!(
            "From {} (game year {}):\nSubject: {}\n{}",
            empire_label(game_data, mail.sender_empire_id),
            mail.year,
            subject,
            mail.body.trim(),
        );
        push_routed_message_chunked(
            &mut messages,
            game_data,
            mail.recipient_empire_id,
            0x08,
            RESULTS_TAIL_BOMBARD,
            &text,
        );
        if let Some(player) = game_data
            .player
            .records
            .get_mut(mail.recipient_empire_id.saturating_sub(1) as usize)
        {
            player.raw[0x30] = 1;
            player.raw[0x34] = 1;
        }
    }

    if !queued_mail.is_empty() && !existing_messages.is_empty() && existing_messages.len() % 84 != 0
    {
        return Ok(existing_messages.to_vec());
    }
    if messages.is_empty() && !existing_messages.is_empty() {
        return Ok(existing_messages.to_vec());
    }

    Ok(messages)
}
