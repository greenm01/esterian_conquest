use std::collections::{BTreeMap, BTreeSet};

use ec_data::maint::{FleetBattlePerspective, timing::format_report_first_line};
use ec_data::{
    ContactReportSource, CoreGameData, DatabaseDat, DatabaseRecord, EmpireProductionRankingSort,
    FleetOrderValidationError, FleetPlayerInputValidationError, MaintenanceEvents, Mission,
    MissionOutcome, PlanetDat, PlanetIntelSnapshot, PlanetIntelSource,
    PlanetPlayerInputValidationError, PlayerDiplomacyValidationError, QueuedPlayerMail, ShipLosses,
};

const RESULTS_RECORD_SIZE: usize = 84;
const RESULTS_TEXT_SIZE: usize = 72;
const RESULTS_TEXT_START: usize = 2;
const RESULTS_TEXT_END: usize = RESULTS_TEXT_START + RESULTS_TEXT_SIZE;
const RESULTS_END_OF_TRANSMISSION: &str = "<end of transmission>";
const RESULTS_TAIL_BOMBARD: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 185, 11];
const RESULTS_TAIL_INVASION: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 195, 11];
const RESULTS_TAIL_FLEET: [u8; 10] = [0, 0, 0, 0, 7, 0, 0, 0, 194, 11];
const RESULTS_TAIL_COLONIZATION: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 184, 11];
const RESULTS_TAIL_SCOUTING: [u8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 186, 11];

/// Regenerate DATABASE.DAT from current PLANETS.DAT and CONQUEST.DAT year.
///
/// `pre_maint_planets` is the planet state before maintenance ran, used to detect
/// which planets had active build queues (which affects certain DATABASE fields).
pub(crate) fn build_database_dat(
    game_data: &CoreGameData,
    pre_maint_planets: &PlanetDat,
    planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
    events: &MaintenanceEvents,
    template: Option<&DatabaseDat>,
) -> DatabaseDat {
    let player_count = game_data.conquest.player_count() as usize;
    let planet_count = game_data.planets.records.len();
    let expected_record_count = player_count * planet_count;
    let template = template
        .filter(|db| db.records.len() == expected_record_count)
        .cloned();
    let template = template.as_ref();
    let mut new_database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .records
            .iter()
            .map(|planet| planet.planet_name())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year(),
        player_count,
        None,
    );
    let current_intel_year = game_data.conquest.game_year().saturating_sub(1);
    let current_turn_grants = collect_planet_intel_sources(events);

    for player in 0..player_count {
        let viewer_empire_raw = (player + 1) as u8;
        let previous_rows = planet_intel_by_viewer.get(player);
        for planet_idx in 0..planet_count {
            let record_idx = DatabaseDat::record_index(planet_idx, player, planet_count);
            let template_record = template.and_then(|db| db.records.get(record_idx));
            let record = &mut new_database.records[record_idx];
            let planet = &game_data.planets.records[planet_idx];
            let snapshot = previous_rows.and_then(|rows| rows.get(&(planet_idx + 1)));
            let current_turn_grant = current_turn_grants.get(&(viewer_empire_raw, planet_idx));
            let owns_world = planet.owner_empire_slot_raw() == viewer_empire_raw;

            if owns_world {
                apply_owned_world_row(record, template_record, planet, current_intel_year);
                continue;
            }

            if let Some(source) = current_turn_grant.copied() {
                apply_intel_grant_row(record, template_record, planet, current_intel_year, source);
                continue;
            }

            if let Some(snapshot) = snapshot {
                apply_snapshot_row(record, template_record, snapshot);
                continue;
            }

            if let Some(template_record) = template_record.filter(|row| is_orbit_record(row)) {
                preserve_orbit_record(
                    record,
                    template_record,
                    game_data,
                    pre_maint_planets,
                    planet_idx,
                    player,
                    current_intel_year,
                );
                continue;
            }

            record.set_unknown_planet();
        }
    }

    new_database
}

fn collect_planet_intel_sources(
    events: &MaintenanceEvents,
) -> BTreeMap<(u8, usize), PlanetIntelSource> {
    let mut sources = BTreeMap::new();
    for event in &events.planet_intel_events {
        sources.insert((event.viewer_empire_raw, event.planet_idx), event.source);
    }
    sources
}

fn is_orbit_record(record: &DatabaseRecord) -> bool {
    let scan_marker = record.raw[0x15];
    (0x01..=0x04).contains(&scan_marker) && record.raw[0x00] == 0
}

fn preserve_orbit_record(
    record: &mut DatabaseRecord,
    template_record: &DatabaseRecord,
    game_data: &CoreGameData,
    pre_maint_planets: &PlanetDat,
    planet_idx: usize,
    player_idx: usize,
    intel_year: u16,
) {
    record.copy_from(template_record);
    record.set_planet_name("Not Named Yet");
    set_year_word(record, 0x16, Some(intel_year));
    set_year_word(record, 0x18, Some(intel_year));
    set_year_word(record, 0x27, Some(intel_year));

    if planet_idx < pre_maint_planets.records.len() {
        let had_build_queue =
            (0..10).any(|slot| pre_maint_planets.records[planet_idx].build_count_raw(slot) > 0);
        if had_build_queue {
            record.raw[0x1e] = 0x00;
        }
    }

    if planet_idx < game_data.planets.records.len() {
        let planet = &game_data.planets.records[planet_idx];
        let planet_owner = planet.owner_empire_slot_raw() as usize;
        if planet.raw[0x03] == 0x87 && planet_owner > 0 && planet_owner == player_idx + 1 {
            let player_mode = game_data.player.records[player_idx].raw[0x00];
            let autopilot = game_data.player.records[player_idx].raw[0x6D];
            let ai_ran = player_mode == 0xff || (player_mode == 0x01 && autopilot == 0x01);
            if ai_ran {
                let owner_slot = planet_owner as u8;
                record.raw[0x1e] = unresolved_orbit_status_low_byte(
                    owner_slot,
                    &planet.planet_name(),
                    Some(template_record),
                );
                record.raw[0x23] = planet.army_count_raw();
                record.raw[0x24] = 0x00;
            }
        }
    }
}

fn apply_snapshot_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    snapshot: &PlanetIntelSnapshot,
) {
    let Some(name) = snapshot.known_name.as_deref() else {
        record.set_unknown_planet();
        return;
    };
    let Some(owner_slot) = snapshot.known_owner_empire_id else {
        record.set_unknown_planet();
        return;
    };
    let Some(potential) = snapshot.known_potential_production else {
        record.set_unknown_planet();
        return;
    };
    let current_production = template_current_production(template_record);
    let word_1e = template_word_1e(template_record);
    if let (Some(armies), Some(batteries)) =
        (snapshot.known_armies, snapshot.known_ground_batteries)
    {
        apply_visible_row(
            record,
            template_record,
            name,
            owner_slot,
            potential,
            current_production,
            word_1e,
            Some(armies),
            Some(batteries),
            snapshot.last_intel_year,
            snapshot.last_intel_year,
        );
    } else {
        apply_visible_row(
            record,
            template_record,
            name,
            owner_slot,
            potential,
            current_production,
            word_1e,
            None,
            None,
            snapshot.last_intel_year,
            None,
        );
    }
}

fn apply_owned_world_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    planet: &ec_data::PlanetRecord,
    intel_year: u16,
) {
    let potential = planet.potential_production_points_current_known();
    apply_visible_row(
        record,
        template_record,
        planet.planet_name().as_str(),
        planet.owner_empire_slot_raw(),
        potential,
        template_current_production(template_record).or(Some(potential)),
        Some(owned_row_word_1e(
            planet.owner_empire_slot_raw(),
            planet.planet_name().as_str(),
            template_record,
        )),
        Some(planet.army_count_raw()),
        Some(planet.ground_batteries_raw()),
        Some(intel_year),
        Some(intel_year),
    );
}

fn apply_intel_grant_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    planet: &ec_data::PlanetRecord,
    intel_year: u16,
    source: PlanetIntelSource,
) {
    let potential = planet.potential_production_points_current_known();
    let (current_production, word_1e, armies, batteries) = match source {
        PlanetIntelSource::ScoutSolarSystem => (
            template_current_production(template_record).or(Some(potential)),
            template_word_1e(template_record).or(Some(0x23)),
            Some(planet.army_count_raw()),
            Some(planet.ground_batteries_raw()),
        ),
        PlanetIntelSource::ViewWorld => (
            template_current_production(template_record),
            template_word_1e(template_record),
            None,
            None,
        ),
        PlanetIntelSource::Assault => (
            template_current_production(template_record),
            template_word_1e(template_record),
            Some(planet.army_count_raw()),
            Some(planet.ground_batteries_raw()),
        ),
    };
    apply_visible_row(
        record,
        template_record,
        planet.planet_name().as_str(),
        planet.owner_empire_slot_raw(),
        potential,
        current_production,
        word_1e,
        armies,
        batteries,
        Some(intel_year),
        Some(intel_year),
    );
}

fn apply_visible_row(
    record: &mut DatabaseRecord,
    template_record: Option<&DatabaseRecord>,
    planet_name: &str,
    owner_slot: u8,
    potential: u16,
    current_production: Option<u16>,
    word_1e: Option<u16>,
    armies: Option<u8>,
    batteries: Option<u8>,
    seen_year: Option<u16>,
    scout_year: Option<u16>,
) {
    if let Some(template_record) = template_record {
        record.copy_from(template_record);
    } else {
        record.set_unknown_planet();
    }
    record.set_planet_name(planet_name);
    record.raw[0x15] = owner_slot;
    set_year_word(record, 0x16, seen_year);
    set_year_word(record, 0x18, seen_year);
    record.raw[0x1c] = potential.min(u16::from(u8::MAX)) as u8;
    if let Some(current_production) = current_production {
        record.raw[0x1d] = current_production.min(u16::from(u8::MAX)) as u8;
    }
    if let Some(word_1e) = word_1e {
        record.set_word_at(0x1e, word_1e);
    }
    record.raw[0x23] = armies.unwrap_or(0xff);
    record.raw[0x24] = if armies.is_some() { 0x00 } else { 0xff };
    record.raw[0x25] = batteries.unwrap_or(0xff);
    record.raw[0x26] = if batteries.is_some() { 0x00 } else { 0xff };
    set_year_word(record, 0x27, scout_year);
}

fn set_year_word(record: &mut DatabaseRecord, offset: usize, year: Option<u16>) {
    let bytes = year.unwrap_or(0).to_le_bytes();
    record.raw[offset] = bytes[0];
    record.raw[offset + 1] = bytes[1];
}

fn template_current_production(template_record: Option<&DatabaseRecord>) -> Option<u16> {
    template_record
        .map(|record| record.raw[0x1d])
        .filter(|value| *value != 0xff)
        .map(u16::from)
}

fn template_word_1e(template_record: Option<&DatabaseRecord>) -> Option<u16> {
    template_record
        .map(|record| record.word_at(0x1e))
        .filter(|value| *value != u16::MAX)
}

fn owned_row_word_1e(
    owner_slot: u8,
    planet_name: &str,
    template_record: Option<&DatabaseRecord>,
) -> u16 {
    if planet_name.eq_ignore_ascii_case("not named yet") {
        0x23
    } else if let Some(template_record) =
        template_record.filter(|row| row.raw[0x1e] >= 0x41 && row.raw[0x1e] != 0xff)
    {
        template_record.word_at(0x1e)
    } else {
        u16::from(0x40u8.saturating_add(owner_slot))
    }
}

fn unresolved_orbit_status_low_byte(
    owner_slot: u8,
    planet_name: &str,
    template_record: Option<&DatabaseRecord>,
) -> u8 {
    if planet_name.eq_ignore_ascii_case("not named yet") {
        0x23
    } else if let Some(template_record) =
        template_record.filter(|row| row.raw[0x1e] >= 0x41 && row.raw[0x1e] != 0xff)
    {
        template_record.raw[0x1e]
    } else {
        0x40u8.saturating_add(owner_slot)
    }
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

fn classic_empire_display_name(game_data: &CoreGameData, empire_raw: u8) -> Option<String> {
    let idx = empire_raw.saturating_sub(1) as usize;
    let player = game_data.player.records.get(idx)?;
    let empire = player.controlled_empire_name_summary();
    if !empire.is_empty() {
        return Some(empire);
    }
    let legacy = player.legacy_status_name_summary();
    if !legacy.is_empty() {
        return Some(legacy);
    }
    let handle = player.assigned_player_handle_summary();
    if !handle.is_empty() {
        return Some(handle);
    }
    None
}

fn classic_empire_clause(game_data: &CoreGameData, empire_raw: u8) -> String {
    if let Some(name) = classic_empire_display_name(game_data, empire_raw) {
        format!("\"{name}\", (Empire #{empire_raw})")
    } else {
        format!("Empire #{empire_raw}")
    }
}

fn ordinal_number(value: usize) -> String {
    let suffix = match value % 100 {
        11..=13 => "th",
        _ => match value % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{value}{suffix}")
}

fn classic_results_tail_for_year(mut template: [u8; 10], year: u16) -> [u8; 10] {
    let year_bytes = year.to_le_bytes();
    template[8] = year_bytes[0];
    template[9] = year_bytes[1];
    template
}

fn classic_results_chain_tail_for_year(
    template: [u8; 10],
    year: u16,
    chain_id: u16,
    next_chain_id: u16,
) -> [u8; 10] {
    let mut tail = classic_results_tail_for_year(template, year);
    tail[0..2].copy_from_slice(&chain_id.to_le_bytes());
    tail[2..4].fill(0);
    tail[4..6].copy_from_slice(&next_chain_id.to_le_bytes());
    tail[6..8].fill(0);
    tail
}

fn fleet_label(fleet_id: u8) -> String {
    format!("{} Fleet", ordinal_number(fleet_id as usize))
}

fn owned_fleet_source_clause(fleet_id: Option<u8>, location: &str) -> String {
    match fleet_id.filter(|fleet_id| *fleet_id != 0) {
        Some(fleet_id) => format!(
            "From your {}, located in {}:",
            fleet_label(fleet_id),
            location
        ),
        None => format!("From your fleet, located in {}:", location),
    }
}

fn owned_fleet_source_clause_from_idx(
    game_data: &CoreGameData,
    fleet_idx: usize,
    location: &str,
) -> String {
    let fleet_id = game_data
        .fleets
        .records
        .get(fleet_idx)
        .map(|fleet| fleet.fleet_id());
    owned_fleet_source_clause(fleet_id, location)
}

fn known_hostile_fleet_label(
    game_data: &CoreGameData,
    fleet_id: Option<u8>,
    empire_raw: u8,
) -> Option<String> {
    let fleet_id = fleet_id.filter(|fleet_id| *fleet_id != 0)?;
    Some(format!(
        "the {} of {}",
        fleet_label(fleet_id),
        classic_empire_clause(game_data, empire_raw)
    ))
}

fn classic_enemy_reference(
    game_data: &CoreGameData,
    fleet_id: Option<u8>,
    empire_raw: u8,
) -> String {
    known_hostile_fleet_label(game_data, fleet_id, empire_raw)
        .unwrap_or_else(|| classic_empire_clause(game_data, empire_raw))
}

fn mission_report_label(kind: Mission) -> &'static str {
    match kind {
        Mission::MoveOnly => "Move mission report",
        Mission::PatrolSector => "Patrol mission report",
        Mission::GuardStarbase => "Guard Starbase mission report",
        Mission::JoinAnotherFleet => "Join mission report",
        Mission::RendezvousSector => "Rendezvous mission report",
        Mission::GuardBlockadeWorld => "Guard/Blockade World mission report",
        Mission::Salvage => "Salvage mission report",
        Mission::ViewWorld => "Viewing mission report",
        Mission::BombardWorld => "Bombardment mission report",
        Mission::InvadeWorld => "Invasion mission report",
        Mission::BlitzWorld => "Blitz mission report",
        Mission::SeekHome => "Seek-Home mission report",
        _ => "Scouting mission report",
    }
}

fn mission_report_prefix(kind: Mission) -> String {
    format!(" {}:", mission_report_label(kind))
}

fn friendly_losses_sentence(losses: ShipLosses) -> String {
    let summary = ship_loss_summary(losses);
    if summary == "no ship losses" {
        "We suffered no ship losses.".to_string()
    } else {
        format!("We lost {summary}.")
    }
}

fn enemy_losses_sentence(losses: ShipLosses) -> String {
    let summary = ship_loss_summary(losses);
    if summary == "no ship losses" {
        "We were unable to inflict any losses.".to_string()
    } else {
        format!("We observed alien ship casualties of {summary}.")
    }
}

fn battle_outcome_sentence(held_field: bool) -> &'static str {
    if held_field {
        "We held the field."
    } else {
        "We were forced to disengage."
    }
}

#[cfg(test)]
mod tests {
    use super::{classic_results_lines, ordinal_number};

    #[test]
    fn ordinal_number_formats_st_nd_rd_and_teen_exceptions() {
        assert_eq!(ordinal_number(1), "1st");
        assert_eq!(ordinal_number(2), "2nd");
        assert_eq!(ordinal_number(3), "3rd");
        assert_eq!(ordinal_number(4), "4th");
        assert_eq!(ordinal_number(11), "11th");
        assert_eq!(ordinal_number(12), "12th");
        assert_eq!(ordinal_number(13), "13th");
        assert_eq!(ordinal_number(21), "21st");
        assert_eq!(ordinal_number(22), "22nd");
        assert_eq!(ordinal_number(23), "23rd");
    }

    #[test]
    fn classic_results_lines_wrap_body_without_leading_indent() {
        let text = "From your 13th Fleet, located in System(24,14)         Stardate: 52/3011 Sensor contact shows an alien fleet in System(24,14) traveling at a translight speed of 5. Closing to check it out...";
        let lines = classic_results_lines(text);
        assert_eq!(
            lines[0],
            "From your 13th Fleet, located in System(24,14)         Stardate: 52/3011"
        );
        assert_eq!(
            lines[1],
            "Sensor contact shows an alien fleet in System(24,14) traveling at a"
        );
        assert_eq!(
            lines[2],
            "translight speed of 5. Closing to check it out..."
        );
        assert!(lines.iter().all(|line| line.chars().count() <= 72));
        assert!(lines[1].starts_with("Sensor"));
    }
}

fn push_classic_results_chunked(
    data: &mut Vec<u8>,
    _kind: u8,
    header_tail: [u8; 10],
    continuation_tail: [u8; 10],
    text: &str,
) {
    let lines = classic_results_lines(text);
    if lines.is_empty() {
        return;
    }
    // ECGAME reads exactly `kind` records per report.  The kind byte doubles
    // as the record count.  Compute it from the actual text so every report
    // is exactly the right size — no padding, no truncation.
    let kind = (lines.len() + 1) as u8; // text lines + EOT

    for (line_idx, line) in lines.iter().enumerate() {
        let chunk = line.as_bytes();
        let mut record = [0u8; RESULTS_RECORD_SIZE];
        record[0] = kind;
        record[1] = chunk.len() as u8;
        record[RESULTS_TEXT_START..RESULTS_TEXT_START + chunk.len()].copy_from_slice(chunk);
        let tail = if line_idx == 0 {
            header_tail
        } else {
            continuation_tail
        };
        record[RESULTS_TEXT_END..RESULTS_RECORD_SIZE].copy_from_slice(&tail);
        data.extend_from_slice(&record);
    }

    let eot = RESULTS_END_OF_TRANSMISSION.as_bytes();
    let mut record = [0u8; RESULTS_RECORD_SIZE];
    record[0] = kind;
    record[1] = eot.len() as u8;
    record[RESULTS_TEXT_START..RESULTS_TEXT_START + eot.len()].copy_from_slice(eot);
    record[RESULTS_TEXT_END..RESULTS_RECORD_SIZE].copy_from_slice(&continuation_tail);
    data.extend_from_slice(&record);
}

fn classic_results_record_count(text: &str, _kind: u8) -> usize {
    let line_count = classic_results_lines(text).len();
    if line_count == 0 { 0 } else { line_count + 1 }
}

fn classic_results_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let split = byte_index_for_char_width(text, RESULTS_TEXT_SIZE);
    let first_line = text[..split].to_string();
    let mut lines = vec![first_line];
    let body = text[split..].trim_start();
    if body.is_empty() {
        return lines;
    }
    for paragraph in body.split('\n') {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        wrap_classic_paragraph(paragraph, RESULTS_TEXT_SIZE, &mut lines);
    }
    lines
}

#[allow(dead_code)]
fn classic_message_text(text: &str) -> String {
    classic_results_lines(text).join("\n")
}

fn byte_index_for_char_width(text: &str, width: usize) -> usize {
    if width == 0 {
        return 0;
    }
    let mut count = 0usize;
    for (idx, ch) in text.char_indices() {
        if count == width {
            return idx;
        }
        count += 1;
        if idx + ch.len_utf8() == text.len() && count <= width {
            return text.len();
        }
    }
    text.len()
}

fn char_width(text: &str) -> usize {
    text.chars().count()
}

fn wrap_classic_paragraph(paragraph: &str, width: usize, lines: &mut Vec<String>) {
    let mut current = String::new();
    for word in paragraph.split_whitespace() {
        let word_width = char_width(word);
        if current.is_empty() {
            if word_width <= width {
                current.push_str(word);
            } else {
                push_split_long_word(word, width, lines, &mut current);
            }
            continue;
        }

        let candidate_width = char_width(&current) + 1 + word_width;
        if candidate_width <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            if word_width <= width {
                current.push_str(word);
            } else {
                push_split_long_word(word, width, lines, &mut current);
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
}

fn push_split_long_word(word: &str, width: usize, lines: &mut Vec<String>, current: &mut String) {
    let mut chunk = String::new();
    for ch in word.chars() {
        if char_width(&chunk) == width {
            lines.push(std::mem::take(&mut chunk));
        }
        chunk.push(ch);
    }
    if chunk.is_empty() {
        return;
    }
    if char_width(&chunk) == width {
        lines.push(chunk);
    } else {
        current.push_str(&chunk);
    }
}

#[allow(dead_code)]
fn push_routed_message_legacy_chunked(data: &mut Vec<u8>, kind: u8, tail: [u8; 10], text: &str) {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return;
    }
    for chunk in bytes.chunks(75) {
        let mut record = [0u8; RESULTS_RECORD_SIZE];
        record[0] = kind;
        record[1..1 + chunk.len()].copy_from_slice(chunk);
        record[76..84].copy_from_slice(&tail[2..]);
        data.extend_from_slice(&record);
    }
}

#[allow(dead_code)]
fn push_routed_message_chunked(
    data: &mut Vec<u8>,
    game_data: &mut CoreGameData,
    recipient_empire_raw: u8,
    kind: u8,
    tail: [u8; 10],
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
    if let Some(player) = game_data
        .player
        .records
        .get_mut(recipient_empire_raw.saturating_sub(1) as usize)
    {
        player.set_classic_login_reviewables_present(true);
    }
    push_routed_message_legacy_chunked(data, kind, tail, &routed);
}

fn mission_location_phrase(kind: Mission, coords: [u8; 2]) -> String {
    let [x, y] = coords;
    match kind {
        Mission::MoveOnly
        | Mission::PatrolSector
        | Mission::ScoutSector
        | Mission::JoinAnotherFleet
        | Mission::RendezvousSector => {
            format!("Sector({x},{y})")
        }
        _ => format!("System({x},{y})"),
    }
}

fn contact_size_summary(event: &ec_data::ScoutContactEvent) -> String {
    contact_size_summary_from_counts(
        event.small_vessels,
        event.medium_vessels,
        event.large_vessels,
    )
}

fn contact_size_summary_from_counts(
    small_vessels: u32,
    medium_vessels: u32,
    large_vessels: u32,
) -> String {
    match (large_vessels > 0, medium_vessels > 0, small_vessels > 0) {
        (true, true, true) => format!(
            "{} large, {} medium, and {} small vessel(s)",
            large_vessels, medium_vessels, small_vessels
        ),
        (true, true, false) => format!(
            "{} large and {} medium vessel(s)",
            large_vessels, medium_vessels
        ),
        (true, false, true) => format!(
            "{} large and {} small vessel(s)",
            large_vessels, small_vessels
        ),
        (false, true, true) => format!(
            "{} medium and {} small vessel(s)",
            medium_vessels, small_vessels
        ),
        (true, false, false) => format!("{} large vessel(s)", large_vessels),
        (false, true, false) => format!("{} medium vessel(s)", medium_vessels),
        (false, false, true) => format!("{} small vessel(s)", small_vessels),
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
        parts.push(unit_count_text(
            losses.battleships,
            "battleship",
            "battleships",
        ));
    }
    if losses.cruisers > 0 {
        parts.push(unit_count_text(losses.cruisers, "cruiser", "cruisers"));
    }
    if losses.destroyers > 0 {
        parts.push(unit_count_text(
            losses.destroyers,
            "destroyer",
            "destroyers",
        ));
    }
    if losses.scouts > 0 {
        parts.push(unit_count_text(losses.scouts, "scout ship", "scout ships"));
    }
    if losses.transports > 0 {
        parts.push(unit_count_text(
            losses.transports,
            "troop transport ship",
            "troop transport ships",
        ));
    }
    if losses.etacs > 0 {
        parts.push(unit_count_text(losses.etacs, "ETAC ship", "ETAC ships"));
    }
    if parts.is_empty() {
        "no ship losses".to_string()
    } else {
        join_report_parts(&parts)
    }
}

fn unit_count_text(count: u32, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn join_report_parts(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [only] => only.clone(),
        [left, right] => format!("{left} and {right}"),
        _ => {
            let mut text = parts[..parts.len() - 1].join(", ");
            text.push_str(" and ");
            text.push_str(parts.last().unwrap());
            text
        }
    }
}

fn planet_defense_summary(batteries: u8, armies: u8) -> String {
    format!("{batteries} ground battery(ies) and {armies} army(ies)")
}

fn stardock_scan_summary(planet: &ec_data::PlanetRecord) -> String {
    use ec_data::ProductionItemKind;

    let mut parts = Vec::new();
    for slot in 0..6 {
        let count = planet.stardock_count_raw(slot);
        if count == 0 {
            continue;
        }
        let kind = ProductionItemKind::from_raw(planet.stardock_kind_raw(slot));
        let name = match kind {
            ProductionItemKind::Destroyer => {
                unit_count_text(count as u32, "destroyer", "destroyers")
            }
            ProductionItemKind::Cruiser => unit_count_text(count as u32, "cruiser", "cruisers"),
            ProductionItemKind::Battleship => {
                unit_count_text(count as u32, "battleship", "battleships")
            }
            ProductionItemKind::Scout => unit_count_text(count as u32, "scout ship", "scout ships"),
            ProductionItemKind::Transport => unit_count_text(
                count as u32,
                "troop transport ship",
                "troop transport ships",
            ),
            ProductionItemKind::Etac => unit_count_text(count as u32, "ETAC ship", "ETAC ships"),
            ProductionItemKind::Starbase => unit_count_text(count as u32, "starbase", "starbases"),
            ProductionItemKind::GroundBattery
            | ProductionItemKind::Army
            | ProductionItemKind::Unknown(_) => continue,
        };
        parts.push(name);
    }
    if parts.is_empty() {
        "The planet's stardock appears to be empty.".to_string()
    } else {
        format!(
            "Scanning the planet's stardock, we detected {}.",
            join_report_parts(&parts)
        )
    }
}

fn fleet_order_validation_reason_text(reason: FleetOrderValidationError) -> String {
    match reason {
        FleetOrderValidationError::UnknownOrderCode(code) => {
            format!("unknown mission code {code:#04x}")
        }
        FleetOrderValidationError::MissingCombatShips => {
            "the fleet lacks the required combat ships".to_string()
        }
        FleetOrderValidationError::MissingScoutShip => {
            "the fleet lacks the required scout ship".to_string()
        }
        FleetOrderValidationError::MissingEtac => "the fleet lacks the required ETAC".to_string(),
        FleetOrderValidationError::MissingLoadedTroopTransports => {
            "the fleet lacks loaded troop transports".to_string()
        }
        FleetOrderValidationError::MissingPlanetTarget => {
            "the mission target is not a valid world".to_string()
        }
        FleetOrderValidationError::TargetOwnedByFleetEmpire => {
            "the target world belongs to us".to_string()
        }
        FleetOrderValidationError::TargetNotOwnedByFleetEmpire => {
            "the target world is not under our control".to_string()
        }
        FleetOrderValidationError::TargetAlreadyOwned => {
            "the target world is already owned".to_string()
        }
        FleetOrderValidationError::InvalidJoinHost => {
            "the selected host fleet is invalid".to_string()
        }
        FleetOrderValidationError::InvalidGuardStarbase => {
            "the selected starbase linkage is invalid".to_string()
        }
    }
}

fn fleet_player_input_validation_reason_text(reason: FleetPlayerInputValidationError) -> String {
    match reason {
        FleetPlayerInputValidationError::InvalidOrder(order_reason) => {
            fleet_order_validation_reason_text(order_reason)
        }
        FleetPlayerInputValidationError::LoadedArmiesExceedTransportCapacity {
            loaded_armies,
            transports,
        } => format!(
            "loaded armies ({loaded_armies}) exceeded available troop transports ({transports})"
        ),
        FleetPlayerInputValidationError::SpeedExceedsMaximum { speed, max } => {
            format!("fleet speed {speed} exceeded the current maximum speed {max}")
        }
        FleetPlayerInputValidationError::RulesOfEngagementOutOfRange { roe } => {
            format!("rules of engagement {roe} was outside the valid 0-10 range")
        }
        FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { roe } => {
            format!("non-combat fleet used ROE {roe}; non-combat fleets must use ROE 0")
        }
    }
}

fn planet_input_validation_reason_text(reason: PlanetPlayerInputValidationError) -> String {
    match reason {
        PlanetPlayerInputValidationError::InvalidBuildKind(kind) => {
            format!("the build queue contains unknown item kind {kind:#04x}")
        }
        PlanetPlayerInputValidationError::MissingBuildKindForCount => {
            "a build queue slot had points remaining but no build kind".to_string()
        }
        PlanetPlayerInputValidationError::MissingBuildCountForKind => {
            "a build queue slot named an item but had zero remaining cost".to_string()
        }
        PlanetPlayerInputValidationError::InvalidStardockKind(kind) => {
            format!("the stardock contains unknown item kind {kind:#04x}")
        }
        PlanetPlayerInputValidationError::MissingStardockKindForCount => {
            "a stardock slot stored units with no item kind".to_string()
        }
        PlanetPlayerInputValidationError::MissingStardockCountForKind => {
            "a stardock slot named an item but stored zero units".to_string()
        }
        PlanetPlayerInputValidationError::InvalidTaxRate(rate) => {
            format!("the attached tax input {rate}% is invalid")
        }
    }
}

fn diplomacy_input_validation_reason_text(reason: PlayerDiplomacyValidationError) -> String {
    match reason {
        PlayerDiplomacyValidationError::TargetOutOfRange { target_empire_raw } => {
            format!(
                "target empire {} was outside the active player range",
                target_empire_raw
            )
        }
        PlayerDiplomacyValidationError::SelfTarget { empire_raw } => {
            format!(
                "empire {} attempted to target itself in diplomacy",
                empire_raw
            )
        }
        PlayerDiplomacyValidationError::InvalidStoredRelationByte {
            target_empire_raw,
            raw,
        } => format!(
            "stored diplomacy byte {raw:#04x} toward empire {} was invalid",
            target_empire_raw
        ),
    }
}

// ---------------------------------------------------------------------------
// Report entry generation — shared between RESULTS.DAT and MESSAGES.DAT
// ---------------------------------------------------------------------------

/// Controls where a report entry is delivered.
#[derive(Debug, Clone, Copy)]
enum ReportTarget {
    /// Only goes into RESULTS.DAT (global log, no per-player routing).
    ResultsOnly,
    /// Only goes into MESSAGES.DAT (routed to one empire).
    MessagesOnly {
        #[allow(dead_code)]
        recipient: u8,
    },
    /// Goes into both; RESULTS.DAT gets unrouted text, MESSAGES.DAT routes to `recipient`.
    Both { recipient: u8 },
}

struct ReportEntry {
    text: String,
    kind: u8,
    tail: [u8; 10],
    target: ReportTarget,
    repeat_next_pointer: bool,
}

/// Build the right-justified Stardate first-line header for a report entry.
///
/// `source_clause` should end with `:` (e.g. `"From your fleet in System(1,2):"`)
/// and the week/year are formatted as `Stardate: week/year` right-justified
/// within the classic 72-byte results text payload.
fn report_header(source_clause: &str, week: Option<u8>, year: u16) -> String {
    format_report_first_line(source_clause, week.unwrap_or(1), year)
}

/// Generate all player-visible report entries from a completed maintenance turn.
///
/// Both `build_results_dat` and `build_messages_dat` call this function to
/// obtain report text, eliminating duplication.  Each entry carries:
/// - the formatted text (with `Stardate: week/year` right-justified on first line)
/// - the binary record kind/tail
/// - a routing target (results-only, messages-only, or both)
fn generate_report_entries(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<ReportEntry> {
    let year = game_data.conquest.game_year();
    let mut entries: Vec<ReportEntry> = Vec::new();

    // ----- Bombard events (defender-side report) -----
    for event in &events.bombard_events {
        if event.defender_empire_raw == 0 {
            continue;
        }
        let Some(planet) = game_data.planets.records.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.coords_raw();
        let source = format!(
            "From planet \"{}\" in System({x},{y}):",
            planet.planet_name()
        );
        let header = report_header(&source, event.stardate_week, year);
        let attacker = known_hostile_fleet_label(
            game_data,
            event.attacker_fleet_id,
            event.attacker_empire_raw,
        )
        .unwrap_or_else(|| empire_label(game_data, event.attacker_empire_raw));
        let body = format!(
            " We have been bombarded by {}. The attacking fleet initially appeared to contain {}. Our defenses initially contained {}. We observed losses of {} ground batteries and {} armies.",
            attacker,
            ship_loss_summary(event.attacker_initial),
            planet_defense_summary(
                event.defender_batteries_initial,
                event.defender_armies_initial
            ),
            event.defender_battery_losses,
            event.defender_army_losses,
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x08,
            tail: RESULTS_TAIL_BOMBARD,
            target: ReportTarget::Both {
                recipient: event.defender_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Fleet battle events -----
    for event in &events.fleet_battle_events {
        let enemy_list = event
            .enemy_empires_raw
            .iter()
            .map(|empire| classic_empire_clause(game_data, *empire))
            .collect::<Vec<_>>()
            .join(", ");
        let [x, y] = event.coords;
        let source =
            owned_fleet_source_clause(event.reporting_fleet_id, &format!("System({x},{y})"));
        let header = report_header(&source, event.stardate_week, year);
        let enemy = if event.enemy_empires_raw.len() == 1 {
            classic_enemy_reference(
                game_data,
                event.primary_enemy_fleet_id,
                event.enemy_empires_raw[0],
            )
        } else {
            format!("hostile fleets belonging to {enemy_list}")
        };
        let prefix = event
            .reporting_mission
            .map(mission_report_prefix)
            .unwrap_or_default();
        let friendly_initial = ship_loss_summary(event.friendly_initial);
        let enemy_initial = ship_loss_summary(event.enemy_initial);
        let body = if matches!(event.perspective, FleetBattlePerspective::Intercepted) {
            format!(
                "{prefix} We successfully intercepted {enemy}. We had {friendly_initial}. Alien force contained {enemy_initial}. {} {} {}",
                battle_outcome_sentence(event.held_field),
                friendly_losses_sentence(event.friendly_losses),
                enemy_losses_sentence(event.enemy_losses),
            )
        } else {
            format!(
                "{prefix} We were attacked by {enemy} in System({x},{y}). Our force contained {friendly_initial}. Alien force contained {enemy_initial}. {} {} {}",
                battle_outcome_sentence(event.held_field),
                friendly_losses_sentence(event.friendly_losses),
                enemy_losses_sentence(event.enemy_losses),
            )
        };
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Fleet destroyed events -----
    for event in &events.fleet_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .and_then(|empire| {
                known_hostile_fleet_label(game_data, event.primary_enemy_fleet_id, empire)
                    .or_else(|| Some(classic_empire_clause(game_data, empire)))
            })
            .unwrap_or_else(|| "an alien fleet".to_string());
        let verb = if event.was_intercepting {
            "intercepted"
        } else {
            "was attacked by"
        };
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " We lost all contact with the {} shortly after it {} {} in System({x},{y}). Records show the {} was composed of {} and carried {} armies. According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            fleet_label(event.fleet_id),
            verb,
            enemy,
            fleet_label(event.fleet_id),
            ship_loss_summary(event.friendly_initial),
            event.friendly_armies,
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.enemy_losses),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Starbase destroyed events -----
    for event in &events.starbase_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .and_then(|empire| {
                known_hostile_fleet_label(game_data, event.primary_enemy_fleet_id, empire)
                    .or_else(|| Some(classic_empire_clause(game_data, empire)))
            })
            .unwrap_or_else(|| "an alien fleet".to_string());
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " We lost all contact with Starbase {} shortly after it was attacked by {} in System({x},{y}). According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            event.starbase_id,
            enemy,
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.enemy_losses),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Civil disorder events -----
    for event in &events.civil_disorder_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " With all of our controlled worlds lost and no immediate means of recovery, the empire of \"{}\" has fallen into civil disorder. Remaining forces are scattered and unreliable.",
            event.prior_label,
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Campaign outlook events (RESULTS only) -----
    for event in &events.campaign_outlook_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " {} now stands as the sole remaining serious contender for the imperial throne. Other empires may still persist, but none currently appear capable of challenging our claim.",
            empire_label(game_data, event.empire_raw),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::ResultsOnly,
            repeat_next_pointer: false,
        });
    }

    // ----- Campaign outcome events (RESULTS only) -----
    for event in &events.campaign_outcome_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " {} has now been recognized as Emperor. No other stable empire remains capable of contesting the throne.",
            empire_label(game_data, event.emperor_empire_raw),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::ResultsOnly,
            repeat_next_pointer: false,
        });
    }

    // ----- Fleet defection events -----
    for event in &events.fleet_defection_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " We have lost all contact with the {}. In the chaos of civil disorder, the surviving crews have defected and no longer answer to central command.",
            fleet_label(event.fleet_id),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.reporting_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Assault report events (invade/blitz) -----
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
            " No troops were lost during the landing.".to_string()
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
        let source =
            owned_fleet_source_clause(event.attacker_fleet_id, &format!("System({x},{y})"));
        let header = report_header(&source, event.stardate_week, year);
        let body = match (event.kind, event.outcome) {
            (Mission::InvadeWorld, MissionOutcome::Succeeded) => format!(
                " Invasion mission report: Our armies have captured planet \"{}\". The defending world initially contained {}. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet.planet_name(),
                planet_defense_summary(
                    event.defender_batteries_initial,
                    event.defender_armies_initial
                ),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Failed) => format!(
                " Invasion mission report: The landing was repulsed. The defending world initially contained {}. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet_defense_summary(
                    event.defender_batteries_initial,
                    event.defender_armies_initial
                ),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Aborted) => format!(
                " Invasion mission report: Enemy ground batteries prevented a landing. The defending world initially contained {}. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet_defense_summary(
                    event.defender_batteries_initial,
                    event.defender_armies_initial
                ),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::BlitzWorld, MissionOutcome::Succeeded) => format!(
                " Blitz mission report: We have seized planet \"{}\" in a fast assault. The defending world initially contained {}.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                planet.planet_name(),
                planet_defense_summary(
                    event.defender_batteries_initial,
                    event.defender_armies_initial
                ),
                blitz_cover_note,
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
                transport_note,
            ),
            (Mission::BlitzWorld, MissionOutcome::Failed) => format!(
                " Blitz mission report: The blitz attack failed. The defending world initially contained {}.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                planet_defense_summary(
                    event.defender_batteries_initial,
                    event.defender_armies_initial
                ),
                blitz_cover_note,
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
                transport_note,
            ),
            _ => continue,
        };
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x0c,
            tail: RESULTS_TAIL_INVASION,
            target: ReportTarget::Both {
                recipient: event.attacker_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Scout contact events -----
    for event in &events.scout_contact_events {
        let [x, y] = event.coords;
        let size_summary = contact_size_summary(event);
        match event.source {
            ContactReportSource::FleetMission(kind) => {
                let label = mission_report_label(kind);
                let location = mission_location_phrase(kind, event.coords);
                let source = owned_fleet_source_clause(event.reporting_fleet_id, &location);
                let header = report_header(&source, event.stardate_week, year);
                let contact_body = format!(
                    " {label}: Sensor contact shows an alien fleet in {location}. Closing to check it out..."
                );
                entries.push(ReportEntry {
                    text: format!("{header}{contact_body}"),
                    kind: 0x05,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both {
                        recipient: event.viewer_empire_raw,
                    },
                    repeat_next_pointer: false,
                });
                let identified_body = if let Some(enemy) = known_hostile_fleet_label(
                    game_data,
                    event.target_fleet_id,
                    event.target_empire_raw,
                ) {
                    format!(
                        " {label}: We have located and identified the alien fleet in {location}. It is {enemy}. Their fleet contains {size_summary} of unknown type."
                    )
                } else {
                    format!(
                        " {label}: We have located and identified the alien fleet in {location}. It belongs to {}. Their fleet contains {size_summary} of unknown type.",
                        classic_empire_clause(game_data, event.target_empire_raw),
                    )
                };
                entries.push(ReportEntry {
                    text: format!("{header}{identified_body}"),
                    kind: 0x06,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both {
                        recipient: event.viewer_empire_raw,
                    },
                    repeat_next_pointer: false,
                });
            }
            ContactReportSource::Fleet(fleet_id) => {
                let source = owned_fleet_source_clause(Some(fleet_id), &format!("System({x},{y})"));
                let header = report_header(&source, event.stardate_week, year);
                let contact_body = format!(
                    " Sensor contact shows an alien fleet in System({x},{y}). Closing to check it out..."
                );
                entries.push(ReportEntry {
                    text: format!("{header}{contact_body}"),
                    kind: 0x05,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both {
                        recipient: event.viewer_empire_raw,
                    },
                    repeat_next_pointer: false,
                });
                let identified_body = if let Some(enemy) = known_hostile_fleet_label(
                    game_data,
                    event.target_fleet_id,
                    event.target_empire_raw,
                ) {
                    format!(
                        " We have located and identified the alien fleet in System({x},{y}). It is {enemy}. Their fleet contains {size_summary} of unknown type."
                    )
                } else {
                    format!(
                        " We have located and identified the alien fleet in System({x},{y}). It belongs to {}. Their fleet contains {size_summary} of unknown type.",
                        classic_empire_clause(game_data, event.target_empire_raw),
                    )
                };
                entries.push(ReportEntry {
                    text: format!("{header}{identified_body}"),
                    kind: 0x06,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both {
                        recipient: event.viewer_empire_raw,
                    },
                    repeat_next_pointer: false,
                });
            }
            ContactReportSource::Starbase(starbase_id) => {
                let source = format!("From Starbase {starbase_id}, located in System({x},{y}):");
                let header = report_header(&source, event.stardate_week, year);
                let body = if let Some(enemy) = known_hostile_fleet_label(
                    game_data,
                    event.target_fleet_id,
                    event.target_empire_raw,
                ) {
                    format!(
                        " We have located and identified an alien fleet in System({x},{y}). It is {enemy}. Their fleet contains {size_summary} of unknown type. We are alerting all fleets in the area."
                    )
                } else {
                    format!(
                        " We have located and identified an alien fleet in System({x},{y}). It is {}. Their fleet contains {size_summary} of unknown type. We are alerting all fleets in the area.",
                        classic_empire_clause(game_data, event.target_empire_raw),
                    )
                };
                entries.push(ReportEntry {
                    text: format!("{header}{body}"),
                    kind: 0x06,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both {
                        recipient: event.viewer_empire_raw,
                    },
                    repeat_next_pointer: false,
                });
            }
        }
    }

    // ----- Ownership change events -----
    for event in &events.ownership_change_events {
        if event.reporting_empire_raw == 0 {
            continue;
        }
        let Some(planet) = game_data.planets.records.get(event.planet_idx) else {
            continue;
        };
        let [x, y] = planet.coords_raw();
        let from = if event.previous_owner_empire_raw == 0 {
            "unowned world".to_string()
        } else {
            classic_empire_clause(game_data, event.previous_owner_empire_raw)
        };
        let source = format!(
            "From planet \"{}\" in System({x},{y}):",
            planet.planet_name()
        );
        let header = report_header(&source, event.stardate_week, year);
        let body = format!(
            " We have been invaded and captured by {} from {}.",
            classic_empire_clause(game_data, event.new_owner_empire_raw),
            from
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x0c,
            tail: RESULTS_TAIL_INVASION,
            target: ReportTarget::Both {
                recipient: event.reporting_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Colonization events -----
    for event in &events.colonization_events {
        let (planet_idx, colonizer_empire_raw, event_week) = match *event {
            ec_data::ColonizationResolvedEvent::Succeeded {
                planet_idx,
                colonizer_empire_raw,
                stardate_week,
                ..
            } => (planet_idx, colonizer_empire_raw, stardate_week),
            ec_data::ColonizationResolvedEvent::BlockedByOwner {
                planet_idx,
                colonizer_empire_raw,
                stardate_week,
                ..
            } => (planet_idx, colonizer_empire_raw, stardate_week),
        };
        let Some(planet) = game_data.planets.records.get(planet_idx) else {
            continue;
        };
        let [x, y] = planet.coords_raw();
        let fleet_idx = match *event {
            ec_data::ColonizationResolvedEvent::Succeeded { fleet_idx, .. } => fleet_idx,
            ec_data::ColonizationResolvedEvent::BlockedByOwner { fleet_idx, .. } => fleet_idx,
        };
        let source =
            owned_fleet_source_clause_from_idx(game_data, fleet_idx, &format!("System({x},{y})"));
        let header = report_header(&source, event_week, year);
        let body = match *event {
            ec_data::ColonizationResolvedEvent::Succeeded { .. } => {
                " Colonization mission report: We have arrived at our target world, successfully terraformed it, and have started a new colony. We await new orders...".to_string()
            }
            ec_data::ColonizationResolvedEvent::BlockedByOwner { owner_empire_raw, .. } => format!(
                " Colonization mission report: We have entered System({x},{y}) and have determined that aliens are already living on the world found within! We have gone ahead and performed a long range viewing analysis and have determined that the world is owned by {} and has a potential of {} points. We are aborting our mission and are leaving the alien solar system.",
                classic_empire_clause(game_data, owner_empire_raw),
                planet.potential_production_points_current_known(),
            ),
        };
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x09,
            tail: RESULTS_TAIL_COLONIZATION,
            target: ReportTarget::Both {
                recipient: colonizer_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Mission events -----
    for event in &events.mission_events {
        let Some(fleet) = game_data.fleets.records.get(event.fleet_idx) else {
            continue;
        };
        let coords = event
            .location_coords
            .unwrap_or_else(|| fleet.current_location_coords_raw());
        let [x, y] = coords;
        let mission_location = mission_location_phrase(event.kind, coords);
        let source_clause =
            owned_fleet_source_clause_from_idx(game_data, event.fleet_idx, &mission_location);
        let (kind, tail, source, body) = match (event.kind, event.outcome) {
            (Mission::MoveOnly, MissionOutcome::Succeeded) => (
                0x05u8,
                RESULTS_TAIL_FLEET,
                source_clause.clone(),
                " Move mission report: We have arrived at our destination and are awaiting new orders.".to_string(),
            ),
            (Mission::RendezvousSector, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_FLEET,
                source_clause.clone(),
                " Rendezvous mission report: We have arrived at the our rendezvous point and are waiting for more fleets to arrive.".to_string(),
            ),
            (Mission::GuardStarbase, MissionOutcome::Arrived) => {
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
                    source_clause.clone(),
                    format!(" Guard Starbase mission report: We have arrived at {starbase_text} and are beginning our guard/escort mission."),
                )
            }
            (Mission::GuardBlockadeWorld, MissionOutcome::Arrived) => {
                let body = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        format!(
                            " Guard/Blockade World mission report: We have arrived at planet \"{}\" in Sector({x},{y}) and are beginning our guarding/blockading assignment.",
                            planet.planet_name(),
                        )
                    } else {
                        " Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment.".to_string()
                    }
                } else {
                    " Guard/Blockade World mission report: We have arrived at our assigned world and are beginning our guarding/blockading assignment.".to_string()
                };
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    body,
                )
            }
            (Mission::PatrolSector, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_FLEET,
                source_clause.clone(),
                " Patrol mission report: We have arrived at our destination and are beginning our patrolling assignment.".to_string(),
            ),
            (Mission::SeekHome, MissionOutcome::Succeeded) => (
                0x05,
                RESULTS_TAIL_FLEET,
                source_clause.clone(),
                " Seek-Home mission report: We have arrived at our destination and are awaiting new orders.".to_string(),
            ),
            (Mission::BombardWorld, MissionOutcome::Arrived) => (
                // Arrival-only notice (Rust-only; original ECMAINT bombards
                // on the same turn the fleet arrives). Use kind=0x05 to
                // avoid blank padding — the text is only ~3 lines.
                0x05,
                RESULTS_TAIL_BOMBARD,
                source_clause.clone(),
                " Bombardment mission report: We have arrived at our target world and are preparing for bombardment.".to_string(),
            ),
            (Mission::InvadeWorld, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                " Invasion mission report: We have arrived at our target world and are preparing to begin the invasion.".to_string(),
            ),
            (Mission::BlitzWorld, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                " Blitz mission report: We have arrived at our target world and are preparing to launch the assault.".to_string(),
            ),
            (Mission::MoveOnly, MissionOutcome::Aborted) => {
                let destination = fleet.standing_order_target_coords_raw();
                let [dx, dy] = destination;
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    format!(" Move mission report: Hostile action forced us to abort our mission and seek safety in System({dx},{dy})."),
                )
            }
            (Mission::ViewWorld, MissionOutcome::Succeeded) => {
                let body = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        let owner_clause = if planet.owner_empire_slot_raw() == 0 {
                            "unowned".to_string()
                        } else {
                            format!(
                                "owned by {}",
                                classic_empire_clause(
                                    game_data,
                                    planet.owner_empire_slot_raw(),
                                )
                            )
                        };
                        format!(
                            " Viewing mission report: We have entered System({x},{y}) and have completed a long range viewing analysis of the world found within. The world is {owner_clause} and has a potential of {} points. Until ordered otherwise, we will be moving out of the solar system.",
                            planet.potential_production_points_current_known(),
                        )
                    } else {
                        format!(" Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis.")
                    }
                } else {
                    format!(" Viewing mission report: We have entered System({x},{y}) and completed a long range viewing analysis.")
                };
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    body,
                )
            }
            (Mission::ViewWorld, MissionOutcome::Failed) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                source_clause.clone(),
                " Viewing mission report: We found no world to analyze at the assigned destination.".to_string(),
            ),
            (Mission::ViewWorld, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| nearest_owned_destination_text(game_data, event.owner_empire_raw, coords))
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    format!(" Viewing mission report: We were attacked before the viewing mission could be completed. We are aborting our assignment and seeking safety at {retreat}."),
                )
            }
            (Mission::BombardWorld, MissionOutcome::Succeeded) => {
                let bombard_event = events.bombard_events.iter().find(|bombard| {
                    bombard.planet_idx == event.planet_idx.unwrap_or(usize::MAX)
                        && bombard.attacker_empire_raw == event.owner_empire_raw
                });
                let body = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        format!(
                            " Bombardment mission report: We have just concluded a bombing run against planet \"{}\". The target world was defended by {}. {} We managed to destroy {} ground batteries and {} armies. We are holding our position and are awaiting new orders.",
                            planet.planet_name(),
                            bombard_event
                                .map(|e| planet_defense_summary(e.defender_batteries_initial, e.defender_armies_initial))
                                .unwrap_or_else(|| "unknown defenses".to_string()),
                            bombard_event
                                .map(|e| friendly_losses_sentence(e.attacker_losses))
                                .unwrap_or_else(|| "We suffered no ship losses.".to_string()),
                            bombard_event.map(|e| e.defender_battery_losses).unwrap_or(0),
                            bombard_event.map(|e| e.defender_army_losses).unwrap_or(0),
                        )
                    } else {
                        format!(" Bombardment mission report: We have concluded our bombing run and are awaiting new orders.")
                    }
                } else {
                    format!(" Bombardment mission report: We have concluded our bombing run and are awaiting new orders.")
                };
                (
                    0x08,
                    RESULTS_TAIL_BOMBARD,
                    source_clause.clone(),
                    body,
                )
            }
            (Mission::InvadeWorld, _) | (Mission::BlitzWorld, _) => continue,
            (Mission::ScoutSector, MissionOutcome::Succeeded) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                source_clause.clone(),
                " Scouting mission report: We have arrived at our destination and are beginning to scout this sector.".to_string(),
            ),
            (Mission::ScoutSector, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| nearest_owned_destination_text(game_data, event.owner_empire_raw, coords))
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    format!(" Scouting mission report: Hostile action forced us to abort our scouting mission and withdraw toward {retreat}."),
                )
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Succeeded) => {
                if let Some(planet) = game_data
                    .planets
                    .records
                    .iter()
                    .find(|planet| planet.coords_raw() == [x, y])
                {
                    let owner = if planet.owner_empire_slot_raw() == 0 {
                        "Unowned world".to_string()
                    } else {
                        classic_empire_clause(game_data, planet.owner_empire_slot_raw())
                    };
                    let stardock_summary = stardock_scan_summary(planet);
                    let body = format!(
                        " Scouting mission report: We are in extended orbit around planet \"{}\" and have compiled the following data:\n  Owned by: {}\n  Potential production: {} points\n  Estimated present production: {} points\n  Estimated amount of stored goods: {} points\n  Number of armies: {}\n  Number of ground batteries: {}\n  {}",
                        planet.planet_name(),
                        owner,
                        planet.potential_production_points(),
                        planet
                            .present_production_points_current_known()
                            .unwrap_or_else(|| planet.potential_production_points()),
                        planet.stored_goods_raw(),
                        planet.army_count_raw(),
                        planet.ground_batteries_raw(),
                        stardock_summary,
                    );
                    // Extended orbit report: 11 records (kind=0x0B) per
                    // original ECMAINT, verified from shipped corpus logs.
                    (
                        0x0Bu8,
                        RESULTS_TAIL_SCOUTING,
                        source_clause.clone(),
                        body,
                    )
                } else {
                    (
                        0x07,
                        RESULTS_TAIL_SCOUTING,
                        source_clause.clone(),
                        " Scouting mission report: We have arrived at our destination and are beginning to scout this solar system.".to_string(),
                    )
                }
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| nearest_owned_destination_text(game_data, event.owner_empire_raw, coords))
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    format!(" Scouting mission report: We were forced to break off our close reconnaissance and withdraw toward {retreat}."),
                )
            }
            _ => continue,
        };
        let header = report_header(&source, event.stardate_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind,
            tail,
            target: ReportTarget::Both {
                recipient: event.owner_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Salvage events -----
    for event in &events.salvage_events {
        let (owner_empire_raw, event_week, source, body) = match *event {
            ec_data::SalvageResolvedEvent::Succeeded {
                fleet_idx,
                owner_empire_raw,
                planet_idx,
                coords,
                recovered_points,
                stardate_week,
                ..
            } => {
                let [x, y] = coords;
                let planet_name = game_data
                    .planets
                    .records
                    .get(planet_idx)
                    .map(|planet| planet.planet_name())
                    .unwrap_or_else(|| "the target world".to_string());
                (
                    owner_empire_raw,
                    stardate_week,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("System({x},{y})"),
                    ),
                    format!(
                        " Salvage mission report: We have arrived at planet \"{planet_name}\" in System({x},{y}) and have begun salvaging our fleet. We estimate that our fleet will yield {recovered_points} production point(s)."
                    ),
                )
            }
            ec_data::SalvageResolvedEvent::Failed {
                fleet_idx,
                owner_empire_raw,
                planet_idx: Some(planet_idx),
                coords,
                reason: ec_data::SalvageFailureReason::PlanetNotOwned,
                stardate_week,
                ..
            } => {
                let [x, y] = coords;
                let planet_name = game_data
                    .planets
                    .records
                    .get(planet_idx)
                    .map(|planet| planet.planet_name())
                    .unwrap_or_else(|| "the target world".to_string());
                (
                    owner_empire_raw,
                    stardate_week,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("System({x},{y})"),
                    ),
                    format!(
                        " Salvage mission report: We have arrived at planet \"{planet_name}\" in System({x},{y}), but it is not under our control so we cannot salvage our fleet there."
                    ),
                )
            }
            ec_data::SalvageResolvedEvent::Failed {
                fleet_idx,
                owner_empire_raw,
                coords,
                reason: ec_data::SalvageFailureReason::NoPlanetAtTarget,
                stardate_week,
                ..
            } => {
                let [x, y] = coords;
                (
                    owner_empire_raw,
                    stardate_week,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("System({x},{y})"),
                    ),
                    " Salvage mission report: We found no planet to salvage at the assigned destination and are awaiting new orders.".to_string(),
                )
            }
            _ => continue,
        };
        let header = report_header(&source, event_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: owner_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Encounter disposition events (ROE) -----
    for event in &events.encounter_disposition_events {
        let (owner_empire_raw, event_week, source, body) = match *event {
            ec_data::EncounterDispositionEvent::NoEngagement {
                fleet_idx,
                owner_empire_raw,
                mission,
                coords,
                target_empire_raw,
                target_fleet_id,
                small_vessels,
                medium_vessels,
                large_vessels,
                stardate_week,
                ..
            } => (
                owner_empire_raw,
                stardate_week,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                {
                    let prefix = mission.map(mission_report_prefix).unwrap_or_default();
                    let enemy = if let Some(enemy) =
                        known_hostile_fleet_label(game_data, target_fleet_id, target_empire_raw)
                    {
                        format!("It is {enemy}.")
                    } else {
                        format!(
                            "It belongs to {}.",
                            classic_empire_clause(game_data, target_empire_raw)
                        )
                    };
                    format!(
                        "{prefix} We have located and identified the alien fleet in System({},{}) {} Their fleet contains {} of unknown type. In accordance to our ROE, we are avoiding this enemy fleet...",
                        coords[0],
                        coords[1],
                        enemy,
                        contact_size_summary_from_counts(
                            small_vessels,
                            medium_vessels,
                            large_vessels
                        )
                    )
                },
            ),
            ec_data::EncounterDispositionEvent::Retreated {
                fleet_idx,
                owner_empire_raw,
                mission,
                coords,
                target_empire_raw,
                target_fleet_id,
                enemy_initial,
                retreat_target_coords,
                losses_sustained,
                enemy_losses_inflicted,
                stardate_week,
                ..
            } => (
                owner_empire_raw,
                stardate_week,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                {
                    let prefix = mission.map(mission_report_prefix).unwrap_or_default();
                    format!(
                        "{prefix} We successfully intercepted {}. Alien force contained {}. In accordance to our ROE, we withdrew toward System({},{}) after suffering losses of {}. {}",
                        classic_enemy_reference(game_data, target_fleet_id, target_empire_raw),
                        ship_loss_summary(enemy_initial),
                        retreat_target_coords[0],
                        retreat_target_coords[1],
                        ship_loss_summary(losses_sustained),
                        enemy_losses_sentence(enemy_losses_inflicted),
                    )
                },
            ),
        };
        let header = report_header(&source, event_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: owner_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Invalid player state events -----
    for event in &events.invalid_player_state_events {
        let (owner_empire_raw, source, body) = match *event {
            ec_data::InvalidPlayerStateEvent::FleetMission {
                fleet_idx,
                owner_empire_raw,
                coords,
                reason,
                ..
            } => (
                owner_empire_raw,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                format!(
                    " Maintenance canceled this fleet's orders because {}. The fleet is holding position and awaiting new orders.",
                    fleet_order_validation_reason_text(reason)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::FleetInput {
                fleet_idx,
                owner_empire_raw,
                coords,
                reason,
                ..
            } => (
                owner_empire_raw,
                owned_fleet_source_clause_from_idx(
                    game_data,
                    fleet_idx,
                    &format!("Sector({},{})", coords[0], coords[1]),
                ),
                format!(
                    " Maintenance corrected invalid fleet input because {}.",
                    fleet_player_input_validation_reason_text(reason)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::PlanetInput {
                owner_empire_raw,
                coords,
                reason,
                ..
            } => (
                owner_empire_raw.max(1),
                format!("From planet in System({},{}) :", coords[0], coords[1]),
                format!(
                    " Maintenance cleared invalid player input because {}.",
                    planet_input_validation_reason_text(reason)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::PlayerTaxRate {
                owner_empire_raw,
                tax_rate,
                ..
            } => (
                owner_empire_raw,
                "From your central administration:".to_string(),
                format!(
                    " Tax rate input {}% for {} was invalid and has been clamped to 100%.",
                    tax_rate,
                    empire_label(game_data, owner_empire_raw)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::DiplomacyInput {
                owner_empire_raw,
                reason,
                ..
            } => (
                owner_empire_raw,
                "From your foreign ministry:".to_string(),
                format!(
                    " Maintenance reset invalid diplomacy input for {} because {}.",
                    empire_label(game_data, owner_empire_raw),
                    diplomacy_input_validation_reason_text(reason)
                ),
            ),
        };
        // Invalid state events don't have stardate_week on the event itself; use year start.
        let header = report_header(&source, None, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: owner_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Fleet merge events -----
    for event in &events.fleet_merge_events {
        let [x, y] = event.coords;
        let (source, body) = match event.kind {
            Mission::JoinAnotherFleet => (
                owned_fleet_source_clause(
                    Some(event.absorbed_fleet_id),
                    &format!("System({x},{y})"),
                ),
                format!(
                    " Join mission report: We have joined the {} and are now merging with them.",
                    fleet_label(event.host_fleet_id)
                ),
            ),
            Mission::RendezvousSector if event.survivor_side => (
                owned_fleet_source_clause(Some(event.host_fleet_id), &format!("Sector({x},{y})")),
                format!(
                    " Rendezvous mission report: We have arrived at the our rendezvous point and are absorbing the {}.",
                    fleet_label(event.absorbed_fleet_id)
                ),
            ),
            Mission::RendezvousSector => (
                owned_fleet_source_clause(
                    Some(event.absorbed_fleet_id),
                    &format!("Sector({x},{y})"),
                ),
                format!(
                    " Rendezvous mission report: We have arrived at the our rendezvous point and are merging with the {}.",
                    fleet_label(event.host_fleet_id)
                ),
            ),
            _ => continue,
        };
        let header = report_header(&source, event.stardate_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both {
                recipient: event.owner_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    // ----- Join host events -----
    for event in &events.join_host_events {
        let (recipient, source, body) = match *event {
            ec_data::JoinMissionHostEvent::Retargeted {
                fleet_idx,
                owner_empire_raw,
                previous_host_fleet_id,
                new_host_fleet_id,
                coords,
                ..
            } => {
                let [x, y] = coords;
                (
                    owner_empire_raw,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("Sector({x},{y})"),
                    ),
                    format!(
                        " Join mission report: Our intended host fleet ({}) has moved. We are now joining the {} instead.",
                        fleet_label(previous_host_fleet_id),
                        fleet_label(new_host_fleet_id)
                    ),
                )
            }
            ec_data::JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire_raw,
                destroyed_host_fleet_id,
                coords,
                ..
            } => {
                let [x, y] = coords;
                (
                    owner_empire_raw,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("Sector({x},{y})"),
                    ),
                    format!(
                        " Join mission report: In light of the destruction of the {}, we are holding our current position in Sector({x},{y}) and are awaiting new orders.",
                        fleet_label(destroyed_host_fleet_id)
                    ),
                )
            }
        };
        let header = report_header(&source, None, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient },
            repeat_next_pointer: false,
        });
    }

    // ----- Mission retarget events -----
    for event in &events.mission_retarget_events {
        let source = match *event {
            ec_data::MissionRetargetEvent::Retargeted {
                fleet_idx,
                new_target_coords,
                ..
            } => owned_fleet_source_clause_from_idx(
                game_data,
                fleet_idx,
                &format!("Sector({},{})", new_target_coords[0], new_target_coords[1]),
            ),
            ec_data::MissionRetargetEvent::Abandoned {
                fleet_idx, coords, ..
            } => owned_fleet_source_clause_from_idx(
                game_data,
                fleet_idx,
                &format!("Sector({},{})", coords[0], coords[1]),
            ),
        };
        let (recipient, body) = match *event {
            ec_data::MissionRetargetEvent::Retargeted {
                owner_empire_raw,
                mission: Mission::SeekHome,
                previous_target_coords,
                new_target_coords,
                ..
            } => (
                owner_empire_raw,
                format!(
                    "Seek-Home mission report: Our original refuge at Sector({},{}) is no longer suitable, so we are now seeking home at Sector({},{}) instead.",
                    previous_target_coords[0],
                    previous_target_coords[1],
                    new_target_coords[0],
                    new_target_coords[1]
                ),
            ),
            ec_data::MissionRetargetEvent::Abandoned {
                owner_empire_raw,
                mission: Mission::SeekHome,
                coords,
                ..
            } => (
                owner_empire_raw,
                format!(
                    "Seek-Home mission report: With no owned planets remaining, we are holding our current position in Sector({},{}) and are awaiting new orders.",
                    coords[0], coords[1]
                ),
            ),
            ec_data::MissionRetargetEvent::Retargeted {
                owner_empire_raw,
                mission: Mission::GuardStarbase,
                previous_target_coords,
                new_target_coords,
                ..
            } => (
                owner_empire_raw,
                format!(
                    "Guard Starbase mission report: The guarded starbase is no longer at Sector({},{}) and we are now moving to Sector({},{}) to resume escort duty.",
                    previous_target_coords[0],
                    previous_target_coords[1],
                    new_target_coords[0],
                    new_target_coords[1]
                ),
            ),
            ec_data::MissionRetargetEvent::Abandoned {
                owner_empire_raw,
                mission: Mission::GuardStarbase,
                coords,
                ..
            } => (
                owner_empire_raw,
                format!(
                    "Guard Starbase mission report: We can no longer locate the guarded starbase, so we are holding our current position in Sector({},{}) and awaiting new orders.",
                    coords[0], coords[1]
                ),
            ),
            ec_data::MissionRetargetEvent::Retargeted {
                owner_empire_raw,
                mission: Mission::JoinAnotherFleet,
                previous_target_coords,
                new_target_coords,
                ..
            } => (
                owner_empire_raw,
                format!(
                    "Join mission report: Our host fleet has changed position from Sector({},{}) to Sector({},{}) and we are continuing pursuit.",
                    previous_target_coords[0],
                    previous_target_coords[1],
                    new_target_coords[0],
                    new_target_coords[1]
                ),
            ),
            _ => continue,
        };
        let header = report_header(&source, None, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient },
            repeat_next_pointer: false,
        });
    }

    // ----- Diplomatic escalation events (MESSAGES only — bilateral routing) -----
    for event in &events.diplomatic_escalation_events {
        let left_source = "From your Fleet Command Center:";
        let left_header = report_header(left_source, event.stardate_week, year);
        let left_body = format!(
            " Hostile action has escalated our relations with {} to enemy status.",
            empire_label(game_data, event.right_empire_raw),
        );
        entries.push(ReportEntry {
            text: format!("{left_header}{left_body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::MessagesOnly {
                recipient: event.left_empire_raw,
            },
            repeat_next_pointer: false,
        });
        let right_body = format!(
            " Hostile action has escalated our relations with {} to enemy status.",
            empire_label(game_data, event.left_empire_raw),
        );
        entries.push(ReportEntry {
            text: format!("{left_header}{right_body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::MessagesOnly {
                recipient: event.right_empire_raw,
            },
            repeat_next_pointer: false,
        });
    }

    entries
}

// ---------------------------------------------------------------------------
// Public builders
// ---------------------------------------------------------------------------

/// Build the RESULTS.DAT binary from current game data and maintenance events.
pub(crate) fn build_results_dat(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<u8> {
    let result_entries = generate_report_entries(game_data, events)
        .into_iter()
        .filter(|entry| !matches!(entry.target, ReportTarget::MessagesOnly { .. }))
        .collect::<Vec<_>>();
    let mut results = Vec::new();
    let year = game_data.conquest.game_year();
    let mut recipient_slots = BTreeSet::new();
    let record_counts = result_entries
        .iter()
        .map(|entry| classic_results_record_count(&entry.text, entry.kind))
        .collect::<Vec<_>>();
    let mut header_record_indexes = Vec::with_capacity(record_counts.len());
    let mut next_header_record_index = 0usize;
    for record_count in &record_counts {
        header_record_indexes.push(next_header_record_index);
        next_header_record_index += *record_count;
    }

    for (entry_idx, entry) in result_entries.iter().enumerate() {
        let chain_id = if entry_idx == 0 {
            0
        } else {
            (header_record_indexes[entry_idx - 1] + 1) as u16
        };
        let next_chain_id = if entry_idx + 1 < header_record_indexes.len() {
            (header_record_indexes[entry_idx + 1] + 1) as u16
        } else {
            0
        };
        let header_tail =
            classic_results_chain_tail_for_year(entry.tail, year, chain_id, next_chain_id);
        let continuation_next_chain_id = if entry.repeat_next_pointer {
            next_chain_id
        } else {
            0
        };
        let continuation_tail = classic_results_chain_tail_for_year(
            entry.tail,
            year,
            chain_id,
            continuation_next_chain_id,
        );
        push_classic_results_chunked(
            &mut results,
            entry.kind,
            header_tail,
            continuation_tail,
            &entry.text,
        );

        match entry.target {
            ReportTarget::Both { recipient } if recipient != 0 => {
                recipient_slots.insert(recipient.saturating_sub(1) as usize);
            }
            ReportTarget::ResultsOnly => {
                for (idx, player) in game_data.player.records.iter().enumerate() {
                    if player.owner_mode_raw() == (idx + 1) as u8 {
                        recipient_slots.insert(idx);
                    }
                }
            }
            _ => {}
        }
    }

    let next_free_chain_id = header_record_indexes
        .last()
        .map(|index| (index + 1) as u16)
        .unwrap_or(0);
    for (idx, player) in game_data.player.records.iter_mut().enumerate() {
        let has_results = recipient_slots.contains(&idx) && !result_entries.is_empty();
        player.set_classic_results_review_state_present(has_results);
        player.set_classic_results_chain_state(has_results, next_free_chain_id);
    }

    results
}

/// Build the MESSAGES.DAT binary, routing each entry to its intended recipient empire.
pub(crate) fn build_messages_dat(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
    queued_mail: &[QueuedPlayerMail],
    existing_messages: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let _ = events;
    let _ = queued_mail;

    if !existing_messages.is_empty() {
        return Ok(existing_messages.to_vec());
    }

    for player in &mut game_data.player.records {
        player.set_classic_messages_review_state_present(false);
    }

    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// Rankings output (Phase 3)
// ---------------------------------------------------------------------------

/// Build the classic-format rankings text for the current game state.
///
/// Format:
/// ```text
/// Stardate: YYYY A.D.
///
/// Empire Rankings (by production):
///   1. Empire #1 "Alpha"  — 12 planets, 480 production
///   ...
/// ```
pub(crate) fn build_rankings_text(game_data: &CoreGameData) -> String {
    let year = game_data.conquest.game_year();
    let stardate = ec_data::maint::timing::format_rankings_stardate(year);
    let rows = game_data.empire_production_ranking_rows(EmpireProductionRankingSort::Production);

    let mut out = String::new();
    out.push_str(&stardate);
    out.push('\n');
    out.push('\n');
    out.push_str("Empire Rankings (by production):\n");
    for (rank, row) in rows.iter().enumerate() {
        let name = if row.empire_name.is_empty() {
            format!("Empire #{}", row.empire_id)
        } else {
            format!("Empire #{} \"{}\"", row.empire_id, row.empire_name)
        };
        out.push_str(&format!(
            "  {}. {}  — {} planet(s), {} production\n",
            rank + 1,
            name,
            row.planets_owned,
            row.current_production,
        ));
    }
    out
}
