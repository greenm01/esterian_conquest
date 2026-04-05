use std::collections::{BTreeMap, BTreeSet};

use crate::maint::{FleetBattlePerspective, timing::format_report_first_line};
use nc_data::{
    ContactReportSource, CoreGameData, EmpireProductionRankingSort, FleetOrderValidationError,
    FleetPlayerInputValidationError, MaintenanceEvents, Mission, MissionOutcome, Order,
    PlanetIntelEvent, PlanetIntelSnapshot, PlanetIntelSource, PlanetPlayerInputValidationError,
    PlayerDiplomacyValidationError, ReportBlockRow, ShipLosses,
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

fn fleet_label(fleet_number: u8) -> String {
    format!("{} Fleet", ordinal_number(fleet_number as usize))
}

fn owned_fleet_source_clause(fleet_number: Option<u8>, location: &str) -> String {
    match fleet_number.filter(|fleet_number| *fleet_number != 0) {
        Some(fleet_number) => format!(
            "From your {}, located in {}:",
            fleet_label(fleet_number),
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
    let fleet_number = game_data
        .fleets
        .records
        .get(fleet_idx)
        .map(|fleet| fleet.local_slot_word_raw() as u8);
    owned_fleet_source_clause(fleet_number, location)
}

fn known_hostile_fleet_label(
    game_data: &CoreGameData,
    fleet_number: Option<u8>,
    empire_raw: u8,
) -> Option<String> {
    let fleet_number = fleet_number.filter(|fleet_number| *fleet_number != 0)?;
    Some(format!(
        "the {} of {}",
        fleet_label(fleet_number),
        classic_empire_clause(game_data, empire_raw)
    ))
}

fn classic_enemy_reference(
    game_data: &CoreGameData,
    fleet_number: Option<u8>,
    empire_raw: u8,
) -> String {
    known_hostile_fleet_label(game_data, fleet_number, empire_raw)
        .unwrap_or_else(|| classic_empire_clause(game_data, empire_raw))
}

fn mission_short_label(kind: Mission) -> &'static str {
    match kind {
        Mission::MoveOnly => "Move",
        Mission::SeekHome => "Seek Home",
        Mission::PatrolSector => "Patrol",
        Mission::ViewWorld => "View",
        Mission::GuardStarbase => "Guard SB",
        Mission::GuardBlockadeWorld => "Guard",
        Mission::ScoutSector => "Scout",
        Mission::ScoutSolarSystem => "Scout SS",
        Mission::BombardWorld => "Bombard",
        Mission::InvadeWorld => "Invade",
        Mission::BlitzWorld => "Blitz",
        Mission::Salvage => "Salvage",
        Mission::JoinAnotherFleet => "Join",
        Mission::RendezvousSector => "Rendezvous",
        _ => "Mission",
    }
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
        let text = "From your 13th Fleet, located in System(24,14)         Stardate: 52/3011 Sensor contact \u{2014} detected and identified an alien fleet in System(24,14). It is the 5th Fleet of \"Enemy\", (Empire #2). Their fleet contains 2 small vessel(s) of unknown type.";
        let lines = classic_results_lines(text);
        assert_eq!(
            lines[0],
            "From your 13th Fleet, located in System(24,14)         Stardate: 52/3011"
        );
        assert_eq!(
            lines[1],
            "Sensor contact \u{2014} detected and identified an alien fleet in"
        );
        assert_eq!(
            lines[2],
            "System(24,14). It is the 5th Fleet of \"Enemy\", (Empire #2). Their fleet"
        );
        assert!(lines.iter().all(|line| line.chars().count() <= 72));
        assert!(lines[1].starts_with("Sensor"));
    }
}

fn push_classic_results_chunked(
    data: &mut Vec<u8>,
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

fn contact_fleet_description(event: &nc_data::ScoutContactEvent) -> String {
    let summary = contact_size_summary_from_counts(
        event.small_vessels,
        event.medium_vessels,
        event.large_vessels,
    );
    if event.small_vessels == 0 && event.medium_vessels == 0 && event.large_vessels == 0 {
        summary
    } else {
        format!("{summary} of unknown type")
    }
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

fn aborted_mission_follow_on_text(
    game_data: &CoreGameData,
    fleet: &nc_data::FleetRecord,
    empire_raw: u8,
) -> String {
    if fleet.standing_order_kind() == Order::SeekHome && fleet.current_speed() > 0 {
        let retreat_target = fleet.standing_order_target_coords_raw();
        format!(
            "seeking safety at {}",
            nearest_owned_destination_text(game_data, empire_raw, retreat_target)
        )
    } else {
        "holding position and awaiting new orders".to_string()
    }
}

fn mission_event_has_assault_report(
    events: &MaintenanceEvents,
    event: &nc_data::MissionEvent,
) -> bool {
    let Some(planet_idx) = event.planet_idx else {
        return false;
    };
    events.assault_report_events.iter().any(|assault| {
        assault.kind == event.kind
            && assault.planet_idx == planet_idx
            && assault.attacker_empire_raw == event.owner_empire_raw
            && assault.outcome == event.outcome
    })
}

fn mission_event_has_fleet_destroyed(
    game_data: &CoreGameData,
    events: &MaintenanceEvents,
    event: &nc_data::MissionEvent,
) -> bool {
    let Some(fleet) = game_data.fleets.records.get(event.fleet_idx) else {
        return false;
    };
    let fleet_number = fleet.local_slot_word_raw() as u8;
    events.fleet_destroyed_events.iter().any(|destroyed| {
        destroyed.fleet_number == fleet_number
            && destroyed.reporting_empire_raw == event.owner_empire_raw
    })
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AbortDisposition {
    Retreating,
    Holding,
}

fn fleet_abort_disposition(fleet: &nc_data::FleetRecord) -> AbortDisposition {
    if fleet.standing_order_kind() == Order::SeekHome && fleet.current_speed() > 0 {
        AbortDisposition::Retreating
    } else {
        AbortDisposition::Holding
    }
}

fn fleet_abort_disposition_text(disposition: AbortDisposition) -> &'static str {
    match disposition {
        AbortDisposition::Retreating => "seeking safety at their nearest colony",
        AbortDisposition::Holding => "holding position and awaiting new orders",
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

fn fleet_force_summary(losses: ShipLosses, loaded_armies: u32) -> String {
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
        let transport_summary = unit_count_text(
            losses.transports,
            "troop transport ship",
            "troop transport ships",
        );
        if loaded_armies > 0 {
            parts.push(format!(
                "{transport_summary} carrying {loaded_armies} armies"
            ));
        } else {
            parts.push(transport_summary);
        }
    }
    if losses.etacs > 0 {
        parts.push(unit_count_text(losses.etacs, "ETAC ship", "ETAC ships"));
    }
    if parts.is_empty() {
        "no ships".to_string()
    } else {
        join_report_parts(&parts)
    }
}

fn fleet_force_summary_with_starbases(
    losses: ShipLosses,
    loaded_armies: u32,
    starbases: u32,
) -> String {
    let ship_summary = fleet_force_summary(losses, loaded_armies);
    if starbases > 0 {
        let sb = unit_count_text(starbases, "starbase", "starbases");
        if ship_summary == "no ships" {
            sb
        } else {
            format!("{ship_summary} and {sb}")
        }
    } else {
        ship_summary
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

fn bombardment_collateral_damage_sentence(
    stardock_items_destroyed: u32,
    stored_goods_destroyed: u32,
    factories_destroyed: u16,
) -> String {
    let mut parts = Vec::new();
    if stardock_items_destroyed > 0 {
        parts.push(unit_count_text(
            stardock_items_destroyed,
            "stardock item",
            "stardock items",
        ));
    }
    if factories_destroyed > 0 {
        parts.push(unit_count_text(
            factories_destroyed as u32,
            "factory",
            "factories",
        ));
    }
    if stored_goods_destroyed > 0 {
        parts.push(format!("{stored_goods_destroyed} stored production"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" Bombardment also destroyed {}.", join_report_parts(&parts))
    }
}

fn stardock_scan_summary(planet: &nc_data::PlanetRecord) -> String {
    use nc_data::ProductionItemKind;

    let mut parts = Vec::new();
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
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
        FleetOrderValidationError::DuplicateFriendlyColonizeTarget {
            target,
            conflicting_fleet_record_index_1_based,
        } => format!(
            "another of our fleets is already set to colonize ({:02},{:02}) (fleet record #{conflicting_fleet_record_index_1_based})",
            target[0], target[1]
        ),
        FleetOrderValidationError::InvalidJoinHost => {
            "the target fleet no longer exists or does not belong to this empire".to_string()
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
            format!(
                "fleet with only scouts, transports, and ETACs used ROE {roe}; support-only fleets must use ROE 0"
            )
        }
    }
}

fn planet_input_validation_reason_text(reason: PlanetPlayerInputValidationError) -> String {
    match reason {
        PlanetPlayerInputValidationError::InvalidBuildKind(kind) => {
            format!("the build queue contains unknown item kind {kind:#04x}")
        }
        PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
            kind_raw,
            points_remaining_raw,
        } => {
            format!(
                "the build queue stores invalid points {points_remaining_raw} for item kind {kind_raw:#04x}"
            )
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
// Report entry generation — classic RESULTS.DAT
// ---------------------------------------------------------------------------

/// Controls which players should see a classic RESULTS.DAT review prompt.
#[derive(Debug, Clone, Copy)]
enum ReportTarget {
    /// Goes into RESULTS.DAT and is visible to all occupied empires.
    ResultsOnly,
    /// Goes into RESULTS.DAT and marks `recipient` as the intended reviewer.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum NarrativePhase {
    MovementPrelude,
    IntelObservation,
    ContactIdentify,
    BattleResolution,
    DefenderAftermath,
    AttackerAftermath,
    CombatFollowOn,
    Generic,
}

fn stardate_week_from_report_text(text: &str) -> u8 {
    text.split("Stardate: ")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|week| week.trim().parse::<u8>().ok())
        .unwrap_or(1)
}

fn narrative_phase_for_report_text(text: &str) -> NarrativePhase {
    if text.contains("Sensor contact") && text.contains("detected and identified")
        || text.contains("We have located and identified the alien fleet")
        || text.contains("we are avoiding this enemy fleet")
    {
        NarrativePhase::ContactIdentify
    } else if text.contains("We lost all contact")
        || text.contains("We successfully intercepted")
        || text.contains("We were attacked by")
        || text.contains("We attempted to disengage")
    {
        NarrativePhase::BattleResolution
    } else if text.contains("We have been bombarded")
        || text.contains("We have been invaded and captured")
    {
        NarrativePhase::DefenderAftermath
    } else if text.contains("Bombardment mission report:")
        || text.contains("Invasion mission report:")
        || text.contains("Blitz mission report:")
    {
        if text.contains("preparing for bombardment")
            || text.contains("preparing to begin the invasion")
            || text.contains("preparing to launch the assault")
        {
            NarrativePhase::MovementPrelude
        } else if text.contains("Hostile action stripped us")
            || text.contains("Enemy ground batteries prevented a landing")
        {
            NarrativePhase::CombatFollowOn
        } else {
            NarrativePhase::AttackerAftermath
        }
    } else if text.contains("Viewing mission report:") || text.contains("Scouting mission report:")
    {
        if text.contains("completed a long range viewing analysis")
            || text.contains("compiled the following data")
        {
            NarrativePhase::IntelObservation
        } else if text.contains("We were attacked before")
            || text.contains("forced us to break off")
            || text.contains("Hostile action forced us to abort")
        {
            NarrativePhase::CombatFollowOn
        } else if text.contains("Sensor contact") && text.contains("detected and identified") {
            NarrativePhase::ContactIdentify
        } else if text.contains("We have located and identified") {
            NarrativePhase::ContactIdentify
        } else {
            NarrativePhase::MovementPrelude
        }
    } else if text.contains("Move mission report:")
        || text.contains("Guard Starbase mission report:")
        || text.contains("Guard/Blockade World mission report:")
        || text.contains("Patrol mission report:")
        || text.contains("Seek-Home mission report:")
        || text.contains("Rendezvous mission report:")
        || text.contains("Colonization mission report:")
        || text.contains("Salvage mission report:")
    {
        if text.contains("Hostile action forced") {
            NarrativePhase::CombatFollowOn
        } else {
            NarrativePhase::MovementPrelude
        }
    } else {
        NarrativePhase::Generic
    }
}

fn matching_planet_intel_event<'a>(
    events: &'a MaintenanceEvents,
    event: &nc_data::MissionEvent,
) -> Option<&'a PlanetIntelEvent> {
    let source = match event.kind {
        Mission::ViewWorld => PlanetIntelSource::ViewWorld,
        Mission::ScoutSolarSystem => PlanetIntelSource::ScoutSolarSystem,
        _ => return None,
    };
    events.planet_intel_events.iter().find(|intel_event| {
        intel_event.viewer_empire_raw == event.owner_empire_raw
            && intel_event.source == source
            && intel_event.source_fleet_idx == Some(event.fleet_idx)
    })
}

fn owner_clause_from_snapshot(snapshot: &PlanetIntelSnapshot, game_data: &CoreGameData) -> String {
    match snapshot.known_owner_empire_id {
        Some(0) => "unowned".to_string(),
        Some(owner) => format!("owned by {}", classic_empire_clause(game_data, owner)),
        None => "of unknown ownership".to_string(),
    }
}

fn stardock_scan_summary_from_snapshot(snapshot: &PlanetIntelSnapshot) -> String {
    match snapshot.known_docked_summary.as_deref() {
        None | Some("Nothing") => "The planet's stardock appears to be empty.".to_string(),
        Some(summary) => format!("Scanning the planet's stardock, we detected {summary}."),
    }
}

/// Generate all player-visible report entries from a completed maintenance turn.
///
/// Each entry carries:
/// - the formatted text (with `Stardate: week/year` right-justified on first line)
/// - the binary record kind/tail
/// - the intended RESULTS.DAT review audience
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
            event.attacker_fleet_number,
            event.attacker_empire_raw,
        )
        .unwrap_or_else(|| empire_label(game_data, event.attacker_empire_raw));
        let production_damage = bombardment_collateral_damage_sentence(
            event.stardock_items_destroyed,
            event.stored_goods_destroyed,
            event.factories_destroyed,
        );
        let body = format!(
            " We have been bombarded by {}. The attacking fleet initially appeared to contain {}. Our defenses initially contained {}. We observed losses of {} ground batteries and {} armies.{}",
            attacker,
            fleet_force_summary(event.attacker_initial, 0),
            planet_defense_summary(
                event.defender_batteries_initial,
                event.defender_armies_initial
            ),
            event.defender_battery_losses,
            event.defender_army_losses,
            production_damage,
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
    let destroyed_fleet_report_keys: BTreeSet<(u8, u8)> = events
        .fleet_destroyed_events
        .iter()
        .filter_map(|event| {
            (event.fleet_number != 0).then_some((event.reporting_empire_raw, event.fleet_number))
        })
        .collect();
    for event in &events.fleet_battle_events {
        if event.reporting_fleet_number.is_some_and(|fleet_number| {
            destroyed_fleet_report_keys.contains(&(event.reporting_empire_raw, fleet_number))
        }) {
            continue;
        }
        let enemy_list = join_report_parts(
            &event
                .enemy_empires_raw
                .iter()
                .map(|empire| classic_empire_clause(game_data, *empire))
                .collect::<Vec<_>>(),
        );
        let [x, y] = event.coords;
        let source =
            owned_fleet_source_clause(event.reporting_fleet_number, &format!("System({x},{y})"));
        let header = report_header(&source, event.stardate_week, year);
        let enemy = if event.enemy_empires_raw.len() == 1 {
            classic_enemy_reference(
                game_data,
                event.primary_enemy_fleet_number,
                event.enemy_empires_raw[0],
            )
        } else {
            format!("hostile fleets belonging to {enemy_list}")
        };
        let prefix = event
            .reporting_mission
            .map(mission_report_prefix)
            .unwrap_or_default();
        let friendly_initial =
            fleet_force_summary(event.friendly_initial, event.friendly_loaded_armies_initial);
        let enemy_initial = fleet_force_summary_with_starbases(
            event.enemy_initial,
            event.enemy_loaded_armies_initial,
            event.enemy_initial_starbases,
        );
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
                known_hostile_fleet_label(game_data, event.primary_enemy_fleet_number, empire)
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
            " We lost all contact with the {} shortly after it {} {} in System({x},{y}). Records show the {} was composed of {}. According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            fleet_label(event.fleet_number),
            verb,
            enemy,
            fleet_label(event.fleet_number),
            fleet_force_summary(event.friendly_initial, event.friendly_loaded_armies_initial),
            fleet_force_summary(event.enemy_initial, event.enemy_loaded_armies_initial),
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
                known_hostile_fleet_label(game_data, event.primary_enemy_fleet_number, empire)
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
            fleet_label(event.fleet_number),
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
            owned_fleet_source_clause(event.attacker_fleet_number, &format!("System({x},{y})"));
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
    // Deduplicate: one report per enemy per location per viewer per turn.
    let mut seen_contacts: std::collections::HashSet<(u8, u8, [u8; 2])> =
        std::collections::HashSet::new();
    for event in &events.scout_contact_events {
        // Suppress empty fleet contacts — no intelligence value.
        if event.small_vessels == 0 && event.medium_vessels == 0 && event.large_vessels == 0 {
            continue;
        }
        // Deduplicate fleet-source contacts; starbase contacts always pass through
        // as they carry unique intelligence about the starbase's detection.
        if !matches!(event.source, ContactReportSource::Starbase(_)) {
            let contact_key = (
                event.viewer_empire_raw,
                event.target_empire_raw,
                event.coords,
            );
            if !seen_contacts.insert(contact_key) {
                continue;
            }
        }
        let [x, y] = event.coords;
        let fleet_description = contact_fleet_description(event);
        match event.source {
            ContactReportSource::FleetMission(kind) => {
                let label = mission_report_label(kind);
                let location = mission_location_phrase(kind, event.coords);
                let source = owned_fleet_source_clause(event.reporting_fleet_number, &location);
                let header = report_header(&source, event.stardate_week, year);
                let body = if let Some(enemy) = known_hostile_fleet_label(
                    game_data,
                    event.target_fleet_number,
                    event.target_empire_raw,
                ) {
                    format!(
                        " {label}: Sensor contact \u{2014} detected and identified an alien fleet in {location}. It is {enemy}. Their fleet contains {fleet_description}."
                    )
                } else {
                    format!(
                        " {label}: Sensor contact \u{2014} detected and identified an alien fleet in {location}. It belongs to {}. Their fleet contains {fleet_description}.",
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
            ContactReportSource::Fleet(fleet_id) => {
                let source = owned_fleet_source_clause(Some(fleet_id), &format!("System({x},{y})"));
                let header = report_header(&source, event.stardate_week, year);
                let body = if let Some(enemy) = known_hostile_fleet_label(
                    game_data,
                    event.target_fleet_number,
                    event.target_empire_raw,
                ) {
                    format!(
                        " Sensor contact \u{2014} detected and identified an alien fleet in System({x},{y}). It is {enemy}. Their fleet contains {fleet_description}."
                    )
                } else {
                    format!(
                        " Sensor contact \u{2014} detected and identified an alien fleet in System({x},{y}). It belongs to {}. Their fleet contains {fleet_description}.",
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
            ContactReportSource::Starbase(starbase_id) => {
                let source = format!("From Starbase {starbase_id}, located in System({x},{y}):");
                let header = report_header(&source, event.stardate_week, year);
                let body = if let Some(enemy) = known_hostile_fleet_label(
                    game_data,
                    event.target_fleet_number,
                    event.target_empire_raw,
                ) {
                    format!(
                        " We have located and identified an alien fleet in System({x},{y}). It is {enemy}. Their fleet contains {fleet_description}. We are alerting all fleets in the area."
                    )
                } else {
                    format!(
                        " We have located and identified an alien fleet in System({x},{y}). It is {}. Their fleet contains {fleet_description}. We are alerting all fleets in the area.",
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
            nc_data::ColonizationResolvedEvent::Succeeded {
                planet_idx,
                colonizer_empire_raw,
                stardate_week,
                ..
            } => (planet_idx, colonizer_empire_raw, stardate_week),
            nc_data::ColonizationResolvedEvent::BlockedByOwner {
                planet_idx,
                colonizer_empire_raw,
                stardate_week,
                ..
            } => (planet_idx, colonizer_empire_raw, stardate_week),
            nc_data::ColonizationResolvedEvent::Aborted { .. } => continue,
        };
        let Some(planet) = game_data.planets.records.get(planet_idx) else {
            continue;
        };
        let [x, y] = planet.coords_raw();
        let fleet_idx = match *event {
            nc_data::ColonizationResolvedEvent::Succeeded { fleet_idx, .. } => fleet_idx,
            nc_data::ColonizationResolvedEvent::BlockedByOwner { fleet_idx, .. } => fleet_idx,
            nc_data::ColonizationResolvedEvent::Aborted { .. } => continue,
        };
        let source =
            owned_fleet_source_clause_from_idx(game_data, fleet_idx, &format!("System({x},{y})"));
        let header = report_header(&source, event_week, year);
        let body = match *event {
            nc_data::ColonizationResolvedEvent::Succeeded { .. } => {
                " Colonization mission report: We have arrived at our target world, successfully terraformed it, and have started a new colony. We await new orders...".to_string()
            }
            nc_data::ColonizationResolvedEvent::BlockedByOwner { owner_empire_raw, .. } => format!(
                " Colonization mission report: We have entered System({x},{y}) and have determined that aliens are already living on the world found within! We have gone ahead and performed a long range viewing analysis and have determined that the world is owned by {} and has a potential of {} points. We are aborting our mission and are leaving the alien solar system.",
                classic_empire_clause(game_data, owner_empire_raw),
                planet.potential_production_points_current_known(),
            ),
            nc_data::ColonizationResolvedEvent::Aborted { .. } => continue,
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
    //
    // Pre-pass: group MissionOutcome::Aborted events for the same empire at
    // the same coords into a single batched report when 2+ fleets qualify.
    // Events consumed here are tracked in `batched_abort_indices` and skipped
    // in the main loop below.
    //
    // Suppression rules (applied before grouping):
    //   1. Fleet was destroyed → FleetDestroyedEvent already covers it.
    //   2. Assault succeeded/failed → AssaultReportEvent already covers it
    //      (existing rule, preserved here).
    //
    // Grouping key: (owner_empire_raw, coords, AbortDisposition)
    // Within a group, fleets are listed as "Fleet N (ShortLabel)".
    let mut batched_abort_indices: BTreeSet<usize> = BTreeSet::new();
    {
        // Collect qualifying aborted events by group key.
        // Value: Vec of (event_index, fleet_id, mission_short_label, stardate_week)
        let mut groups: BTreeMap<
            (u8, [u8; 2], AbortDisposition),
            Vec<(usize, u8, &'static str, Option<u8>)>,
        > = BTreeMap::new();
        for (ev_idx, event) in events.mission_events.iter().enumerate() {
            if event.outcome != MissionOutcome::Aborted {
                continue;
            }
            // Suppression 1: fleet was destroyed.
            if mission_event_has_fleet_destroyed(game_data, events, event) {
                batched_abort_indices.insert(ev_idx);
                continue;
            }
            // Suppression 2: assault report covers this (existing rule).
            if mission_event_has_assault_report(events, event) {
                batched_abort_indices.insert(ev_idx);
                continue;
            }
            let Some(fleet) = game_data.fleets.records.get(event.fleet_idx) else {
                continue;
            };
            let coords = event
                .location_coords
                .unwrap_or_else(|| fleet.current_location_coords_raw());
            let disposition = fleet_abort_disposition(fleet);
            let fleet_number = fleet.local_slot_word_raw() as u8;
            let label = mission_short_label(event.kind);
            groups
                .entry((event.owner_empire_raw, coords, disposition))
                .or_default()
                .push((ev_idx, fleet_number, label, event.stardate_week));
        }
        // Emit one batched report per group that has 2+ fleets; mark all as handled.
        // Single-fleet groups are left for the main loop.
        for ((empire_raw, coords, disposition), mut fleet_entries) in groups {
            if fleet_entries.len() < 2 {
                continue;
            }
            // Sort by fleet number for stable, predictable output.
            fleet_entries.sort_by_key(|&(_, fleet_number, _, _)| fleet_number);
            let [x, y] = coords;
            let fleet_list = join_report_parts(
                &fleet_entries
                    .iter()
                    .map(|(_, fleet_number, label, _)| {
                        format!("Fleet {} ({})", fleet_number, label)
                    })
                    .collect::<Vec<_>>(),
            );
            let disposition_text = fleet_abort_disposition_text(disposition);
            let source = "From your Fleet Command Center:".to_string();
            // Use the earliest stardate_week among the group for the header.
            let week = fleet_entries.iter().filter_map(|(_, _, _, w)| *w).min();
            let header = report_header(&source, week, year);
            let body = format!(
                " Hostile action forced {} to abort their missions in System({x},{y}). They are {}.",
                fleet_list, disposition_text,
            );
            entries.push(ReportEntry {
                text: format!("{header}{body}"),
                kind: 0x05,
                tail: RESULTS_TAIL_FLEET,
                target: ReportTarget::Both {
                    recipient: empire_raw,
                },
                repeat_next_pointer: false,
            });
            for (ev_idx, _, _, _) in &fleet_entries {
                batched_abort_indices.insert(*ev_idx);
            }
        }
    }

    let rendezvous_merged_fleet_indices: std::collections::HashSet<usize> = events
        .fleet_merge_events
        .iter()
        .filter(|e| e.kind == Mission::RendezvousSector)
        .map(|e| e.fleet_idx)
        .collect();

    for (ev_idx, event) in events.mission_events.iter().enumerate() {
        // Skip events already handled by the batched-abort pre-pass.
        if batched_abort_indices.contains(&ev_idx) {
            continue;
        }
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
            (Mission::RendezvousSector, MissionOutcome::Arrived) => {
                if rendezvous_merged_fleet_indices.contains(&event.fleet_idx) {
                    continue;
                }
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    source_clause.clone(),
                    " Rendezvous mission report: We have arrived at our rendezvous point and are waiting for more fleets to arrive.".to_string(),
                )
            }
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
            (Mission::ColonizeWorld, MissionOutcome::Aborted) => (
                0x09,
                RESULTS_TAIL_COLONIZATION,
                source_clause.clone(),
                format!(
                    " Colonization mission report: Hostile action forced us to abandon our colony attempt. We are {}.",
                    aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
                ),
            ),
            (Mission::ViewWorld, MissionOutcome::Succeeded) => {
                let body = if let Some(intel_event) = matching_planet_intel_event(events, event) {
                    if let Some(snapshot) = intel_event.observed_snapshot.as_ref() {
                        format!(
                            " Viewing mission report: We have entered System({x},{y}) and have completed a long range viewing analysis of the world found within. The world is {} and has a potential of {} points. Until ordered otherwise, we will be moving out of the solar system.",
                            owner_clause_from_snapshot(snapshot, game_data),
                            snapshot.known_potential_production.unwrap_or(0),
                        )
                    } else if let Some(planet_idx) = event.planet_idx {
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
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    format!(
                        " Viewing mission report: We were attacked before the viewing mission could be completed. We are aborting our assignment and {}.",
                        aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
                    ),
                )
            }
            (Mission::BombardWorld, MissionOutcome::Succeeded) => {
                let bombard_event = events.bombard_events.iter().find(|bombard| {
                    bombard.planet_idx == event.planet_idx.unwrap_or(usize::MAX)
                        && bombard.attacker_empire_raw == event.owner_empire_raw
                });
                let body = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        {
                            let collateral = bombard_event
                                .map(|e| bombardment_collateral_damage_sentence(e.stardock_items_destroyed, e.stored_goods_destroyed, e.factories_destroyed))
                                .unwrap_or_default();
                            let breakthrough = bombard_event.is_some_and(|e| e.breakthrough);
                            let status = if breakthrough {
                                " We broke through planetary defenses and struck the world's infrastructure."
                            } else {
                                " Planetary batteries absorbed our bombardment. The world's infrastructure remains shielded."
                            };
                            format!(
                                " Bombardment mission report: We have just concluded a bombing run against planet \"{}\". The target world was defended by {}. {} We managed to destroy {} ground batteries and {} armies.{}{} We are maintaining bombardment position and will continue next turn.",
                                planet.planet_name(),
                                bombard_event
                                    .map(|e| planet_defense_summary(e.defender_batteries_initial, e.defender_armies_initial))
                                    .unwrap_or_else(|| "unknown defenses".to_string()),
                                bombard_event
                                    .map(|e| friendly_losses_sentence(e.attacker_losses))
                                    .unwrap_or_else(|| "We suffered no ship losses.".to_string()),
                                bombard_event.map(|e| e.defender_battery_losses).unwrap_or(0),
                                bombard_event.map(|e| e.defender_army_losses).unwrap_or(0),
                                if breakthrough { &collateral } else { "" },
                                status,
                            )
                        }
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
            (Mission::BombardWorld, MissionOutcome::Aborted) => (
                0x08,
                RESULTS_TAIL_BOMBARD,
                source_clause.clone(),
                format!(
                    " Bombardment mission report: Hostile action stripped us of our bombardment capability. We are aborting the mission and {}.",
                    aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
                ),
            ),
            (Mission::InvadeWorld, MissionOutcome::Aborted) => (
                0x0c,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                format!(
                    " Invasion mission report: Hostile action stripped us of our invasion capability before the landing could begin. We are aborting the mission and {}.",
                    aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
                ),
            ),
            (Mission::BlitzWorld, MissionOutcome::Aborted) => (
                0x0c,
                RESULTS_TAIL_INVASION,
                source_clause.clone(),
                format!(
                    " Blitz mission report: Hostile action stripped us of our assault capability before the landing could begin. We are aborting the mission and {}.",
                    aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
                ),
            ),
            (Mission::InvadeWorld, _) | (Mission::BlitzWorld, _) => continue,
            (Mission::ScoutSector, MissionOutcome::Arrived) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                source_clause.clone(),
                " Scouting mission report: We have arrived at our destination and are beginning to scout this sector.".to_string(),
            ),
            (Mission::ScoutSector, MissionOutcome::Succeeded) => {
                // On-station scouts only report when they detect something.
                // No news is good news — suppress the repeating status message.
                continue;
            }
            (Mission::ScoutSector, MissionOutcome::Aborted) => {
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    format!(
                        " Scouting mission report: Hostile action forced us to abort our scouting mission and {}.",
                        aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
                    ),
                )
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Arrived)
            | (Mission::ScoutSolarSystem, MissionOutcome::Succeeded) => {
                if let Some(intel_event) = matching_planet_intel_event(events, event) {
                    if let Some(snapshot) = intel_event.observed_snapshot.as_ref() {
                        let owner = match snapshot.known_owner_empire_id {
                            Some(0) => "Unowned world".to_string(),
                            Some(owner) => classic_empire_clause(game_data, owner),
                            None => "Unknown".to_string(),
                        };
                        let body = format!(
                            " Scouting mission report: We are in extended orbit around planet \"{}\" and have compiled the following data:\n  Owned by: {}\n  Potential production: {} points\n  Estimated present production: {} points\n  Estimated amount of stored goods: {} points\n  Number of armies: {}\n  Number of ground batteries: {}\n  {}",
                            snapshot.known_name.as_deref().unwrap_or("Unknown"),
                            owner,
                            snapshot.known_potential_production.unwrap_or(0),
                            snapshot.known_current_production.unwrap_or(snapshot.known_potential_production.unwrap_or(0) as u8),
                            snapshot.known_stored_points.unwrap_or(0),
                            snapshot.known_armies.unwrap_or(0),
                            snapshot.known_ground_batteries.unwrap_or(0),
                            stardock_scan_summary_from_snapshot(snapshot),
                        );
                        (
                            0x0Bu8,
                            RESULTS_TAIL_SCOUTING,
                            source_clause.clone(),
                            body,
                        )
                    } else if let Some(planet) = game_data
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
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    source_clause.clone(),
                    format!(
                        " Scouting mission report: We were forced to break off our close reconnaissance and {}.",
                        aborted_mission_follow_on_text(game_data, fleet, event.owner_empire_raw)
                    ),
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
            nc_data::SalvageResolvedEvent::Succeeded {
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
            nc_data::SalvageResolvedEvent::Failed {
                fleet_idx,
                owner_empire_raw,
                planet_idx: Some(planet_idx),
                coords,
                reason: nc_data::SalvageFailureReason::PlanetNotOwned,
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
            nc_data::SalvageResolvedEvent::Failed {
                fleet_idx,
                owner_empire_raw,
                coords,
                reason: nc_data::SalvageFailureReason::NoPlanetAtTarget,
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
    // Deduplicate NoEngagement: one avoidance report per enemy per location per turn.
    let mut seen_avoidance: std::collections::HashSet<(u8, u8, [u8; 2])> =
        std::collections::HashSet::new();
    for event in &events.encounter_disposition_events {
        let (owner_empire_raw, event_week, source, body) = match *event {
            nc_data::EncounterDispositionEvent::NoEngagement {
                fleet_idx,
                owner_empire_raw,
                mission,
                coords,
                target_empire_raw,
                target_fleet_number,
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
                        known_hostile_fleet_label(game_data, target_fleet_number, target_empire_raw)
                    {
                        format!("It is {enemy}.")
                    } else {
                        format!(
                            "It belongs to {}.",
                            classic_empire_clause(game_data, target_empire_raw)
                        )
                    };
                    let size_summary = contact_size_summary_from_counts(
                        small_vessels,
                        medium_vessels,
                        large_vessels,
                    );
                    let fleet_desc =
                        if small_vessels == 0 && medium_vessels == 0 && large_vessels == 0 {
                            size_summary
                        } else {
                            format!("{size_summary} of unknown type")
                        };
                    format!(
                        "{prefix} We have located and identified the alien fleet in System({},{}) {} Their fleet contains {fleet_desc}. In accordance to our ROE, we are avoiding this enemy fleet...",
                        coords[0], coords[1], enemy,
                    )
                },
            ),
            nc_data::EncounterDispositionEvent::Retreated {
                fleet_idx,
                owner_empire_raw,
                mission,
                coords,
                target_empire_raw,
                target_fleet_number,
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
                        classic_enemy_reference(game_data, target_fleet_number, target_empire_raw),
                        fleet_force_summary(enemy_initial, 0),
                        retreat_target_coords[0],
                        retreat_target_coords[1],
                        ship_loss_summary(losses_sustained),
                        enemy_losses_sentence(enemy_losses_inflicted),
                    )
                },
            ),
            nc_data::EncounterDispositionEvent::PursuitFire {
                fleet_idx,
                owner_empire_raw,
                mission,
                coords,
                target_empire_raw,
                target_fleet_number,
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
                        "{prefix} We attempted to disengage from {} but were intercepted by {} and suffered pursuit fire. We withdrew toward System({},{}) after suffering losses of {}. {}",
                        classic_enemy_reference(game_data, target_fleet_number, target_empire_raw),
                        classic_enemy_reference(game_data, target_fleet_number, target_empire_raw),
                        retreat_target_coords[0],
                        retreat_target_coords[1],
                        ship_loss_summary(losses_sustained),
                        enemy_losses_sentence(enemy_losses_inflicted),
                    )
                },
            ),
        };
        // Deduplicate NoEngagement: one avoidance report per enemy per location.
        if let nc_data::EncounterDispositionEvent::NoEngagement {
            target_empire_raw,
            coords,
            ..
        } = event
        {
            if !seen_avoidance.insert((owner_empire_raw, *target_empire_raw, *coords)) {
                continue;
            }
        }
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
            nc_data::InvalidPlayerStateEvent::FleetMission {
                fleet_idx,
                owner_empire_raw,
                order_code_raw,
                coords,
                reason,
            } => {
                let order_name = nc_data::Order::from_raw(order_code_raw)
                    .display_label()
                    .to_lowercase();
                (
                    owner_empire_raw,
                    owned_fleet_source_clause_from_idx(
                        game_data,
                        fleet_idx,
                        &format!("Sector({},{})", coords[0], coords[1]),
                    ),
                    format!(
                        " Maintenance canceled this fleet's {order_name} order because {}. The fleet is holding position and awaiting new orders.",
                        fleet_order_validation_reason_text(reason)
                    ),
                )
            }
            nc_data::InvalidPlayerStateEvent::FleetInput {
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
            nc_data::InvalidPlayerStateEvent::PlanetInput {
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
            nc_data::InvalidPlayerStateEvent::PlayerTaxRate {
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
            nc_data::InvalidPlayerStateEvent::DiplomacyInput {
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
    // Join events: one consolidated report per host fleet.
    {
        let mut join_groups: std::collections::BTreeMap<(u8, [u8; 2], u8), Vec<u8>> =
            std::collections::BTreeMap::new();
        let mut join_meta: std::collections::HashMap<(u8, [u8; 2], u8), Option<u8>> =
            std::collections::HashMap::new();
        for event in events
            .fleet_merge_events
            .iter()
            .filter(|e| e.kind == Mission::JoinAnotherFleet)
        {
            let key = (
                event.owner_empire_raw,
                event.coords,
                event.host_fleet_number,
            );
            join_groups
                .entry(key)
                .or_default()
                .push(event.absorbed_fleet_number);
            join_meta.entry(key).or_insert(event.stardate_week);
        }
        for (key, absorbed_numbers) in &join_groups {
            let (owner_empire_raw, coords, host_fleet_number) = *key;
            let [x, y] = coords;
            let stardate_week = join_meta[key];
            let source =
                owned_fleet_source_clause(Some(host_fleet_number), &format!("System({x},{y})"));
            let fleet_list = absorbed_numbers
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>();
            let body = if fleet_list.len() == 1 {
                format!(
                    " Join mission report: Fleet {} has merged with us.",
                    fleet_list[0]
                )
            } else {
                format!(
                    " Join mission report: Fleets {} have merged with us.",
                    join_report_parts(&fleet_list.iter().map(|s| s.clone()).collect::<Vec<_>>())
                )
            };
            let header = report_header(&source, stardate_week, year);
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
    }

    // Rendezvous events: one consolidated report per survivor fleet.
    {
        let rendezvous_arrived: std::collections::HashSet<usize> = events
            .mission_events
            .iter()
            .filter(|e| e.kind == Mission::RendezvousSector && e.outcome == MissionOutcome::Arrived)
            .map(|e| e.fleet_idx)
            .collect();

        let mut rendezvous_groups: std::collections::BTreeMap<(u8, [u8; 2], u8), Vec<u8>> =
            std::collections::BTreeMap::new();
        let mut rendezvous_meta: std::collections::HashMap<(u8, [u8; 2], u8), (usize, Option<u8>)> =
            std::collections::HashMap::new();
        for event in events
            .fleet_merge_events
            .iter()
            .filter(|e| e.kind == Mission::RendezvousSector && e.survivor_side)
        {
            let key = (
                event.owner_empire_raw,
                event.coords,
                event.host_fleet_number,
            );
            rendezvous_groups
                .entry(key)
                .or_default()
                .push(event.absorbed_fleet_number);
            rendezvous_meta
                .entry(key)
                .or_insert((event.fleet_idx, event.stardate_week));
        }

        for (key, absorbed_numbers) in &rendezvous_groups {
            let (owner_empire_raw, coords, host_fleet_number) = *key;
            let [x, y] = coords;
            let (host_fleet_idx, stardate_week) = rendezvous_meta[key];
            let source =
                owned_fleet_source_clause(Some(host_fleet_number), &format!("Sector({x},{y})"));
            let absorbed_list = absorbed_numbers
                .iter()
                .map(|n| format!("the {}", fleet_label(*n)))
                .collect::<Vec<_>>();
            let absorbed_text = join_report_parts(&absorbed_list);
            let arrived_this_turn = rendezvous_arrived.contains(&host_fleet_idx);
            let body = if arrived_this_turn {
                format!(
                    " Rendezvous mission report: We have arrived at our rendezvous point and are absorbing {absorbed_text}."
                )
            } else {
                format!(
                    " Rendezvous mission report: We are on station at our rendezvous point and are absorbing {absorbed_text}."
                )
            };
            let header = report_header(&source, stardate_week, year);
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
    }

    // ----- Join host events -----
    for event in &events.join_host_events {
        let (recipient, source, body) = match *event {
            nc_data::JoinMissionHostEvent::Retargeted {
                fleet_idx,
                owner_empire_raw,
                previous_host_fleet_number,
                new_host_fleet_number,
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
                        fleet_label(previous_host_fleet_number),
                        fleet_label(new_host_fleet_number)
                    ),
                )
            }
            nc_data::JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire_raw,
                destroyed_host_fleet_number,
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
                        fleet_label(destroyed_host_fleet_number)
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
            nc_data::MissionRetargetEvent::Retargeted {
                fleet_idx,
                new_target_coords,
                ..
            } => owned_fleet_source_clause_from_idx(
                game_data,
                fleet_idx,
                &format!("Sector({},{})", new_target_coords[0], new_target_coords[1]),
            ),
            nc_data::MissionRetargetEvent::Abandoned {
                fleet_idx, coords, ..
            } => owned_fleet_source_clause_from_idx(
                game_data,
                fleet_idx,
                &format!("Sector({},{})", coords[0], coords[1]),
            ),
        };
        let (recipient, body) = match *event {
            nc_data::MissionRetargetEvent::Retargeted {
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
            nc_data::MissionRetargetEvent::Abandoned {
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
            nc_data::MissionRetargetEvent::Retargeted {
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
            nc_data::MissionRetargetEvent::Abandoned {
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
            nc_data::MissionRetargetEvent::Retargeted {
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

    entries.sort_by_key(|entry| {
        (
            stardate_week_from_report_text(&entry.text),
            narrative_phase_for_report_text(&entry.text),
        )
    });
    entries
}

// ---------------------------------------------------------------------------
// Public builders
// ---------------------------------------------------------------------------

/// Build the RESULTS.DAT binary from current game data and maintenance events.
#[allow(dead_code)]
pub fn build_results_dat(game_data: &mut CoreGameData, events: &MaintenanceEvents) -> Vec<u8> {
    build_results_rows_with_review(game_data, events)
        .into_iter()
        .flat_map(|row| row.raw_bytes.unwrap_or_default())
        .collect()
}

pub fn build_results_report_blocks(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<ReportBlockRow> {
    build_results_rows_with_review(game_data, events)
}

struct ResultsReviewPlan {
    broadcast_entries: Vec<ReportEntry>,
    viewer_entries: BTreeMap<u8, Vec<ReportEntry>>,
    viewers_with_results: BTreeSet<u8>,
}

fn build_results_rows_with_review(
    game_data: &mut CoreGameData,
    events: &MaintenanceEvents,
) -> Vec<ReportBlockRow> {
    let result_entries = generate_report_entries(game_data, events);
    let year = game_data.conquest.game_year();
    let ResultsReviewPlan {
        broadcast_entries,
        viewer_entries,
        viewers_with_results,
    } = results_review_plan(game_data, &result_entries);
    let mut rows = build_rows_for_viewer(0, &broadcast_entries, year);
    for (viewer_empire_id, entries) in viewer_entries {
        rows.extend(build_rows_for_viewer(viewer_empire_id, &entries, year));
    }
    debug_assert!(rows.iter().all(|row| {
        row.viewer_empire_id == 0 || viewers_with_results.contains(&row.viewer_empire_id)
    }));
    rows
}

fn results_review_plan(
    game_data: &mut CoreGameData,
    result_entries: &[ReportEntry],
) -> ResultsReviewPlan {
    let occupied_viewers = occupied_result_viewers(game_data);
    let mut viewers_with_results = BTreeSet::new();
    let mut broadcast_entries = Vec::new();
    let mut viewer_entries = BTreeMap::<u8, Vec<ReportEntry>>::new();

    for entry in result_entries {
        match entry.target {
            ReportTarget::Both { recipient } if recipient != 0 => {
                viewers_with_results.insert(recipient);
                viewer_entries
                    .entry(recipient)
                    .or_default()
                    .push(clone_report_entry(entry));
            }
            ReportTarget::ResultsOnly => {
                if !occupied_viewers.is_empty() {
                    viewers_with_results.extend(occupied_viewers.iter().copied());
                    broadcast_entries.push(clone_report_entry(entry));
                }
            }
            _ => {}
        }
    }

    for (idx, player) in game_data.player.records.iter_mut().enumerate() {
        let viewer_empire_id = (idx + 1) as u8;
        let visible_record_count =
            visible_report_record_count(&broadcast_entries, viewer_entries.get(&viewer_empire_id));
        let has_results =
            viewers_with_results.contains(&viewer_empire_id) && visible_record_count > 0;
        player.set_classic_results_review_state_present(has_results);
        player.set_classic_results_chain_state(
            has_results,
            if has_results {
                visible_record_count as u16
            } else {
                0
            },
        );
    }

    ResultsReviewPlan {
        broadcast_entries,
        viewer_entries,
        viewers_with_results,
    }
}

fn build_rows_for_viewer(
    viewer_empire_id: u8,
    entries: &[ReportEntry],
    year: u16,
) -> Vec<ReportBlockRow> {
    let record_counts = entries
        .iter()
        .map(|entry| classic_results_record_count(&entry.text, entry.kind))
        .collect::<Vec<_>>();
    let mut header_record_indexes = Vec::with_capacity(record_counts.len());
    let mut next_header_record_index = 0usize;
    for record_count in &record_counts {
        header_record_indexes.push(next_header_record_index);
        next_header_record_index += *record_count;
    }

    entries
        .iter()
        .enumerate()
        .map(|(block_index, entry)| {
            let chain_id = if block_index == 0 {
                0
            } else {
                (header_record_indexes[block_index - 1] + 1) as u16
            };
            let next_chain_id = if block_index + 1 < header_record_indexes.len() {
                (header_record_indexes[block_index + 1] + 1) as u16
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
            let mut raw_bytes = Vec::new();
            push_classic_results_chunked(
                &mut raw_bytes,
                header_tail,
                continuation_tail,
                &entry.text,
            );
            let mut lines = classic_results_lines(&entry.text);
            lines.push(RESULTS_END_OF_TRANSMISSION.to_string());
            ReportBlockRow {
                viewer_empire_id,
                block_index,
                decoded_text: lines.join("\n"),
                raw_bytes: Some(raw_bytes),
                recipient_deleted: false,
            }
        })
        .collect()
}

fn occupied_result_viewers(game_data: &CoreGameData) -> Vec<u8> {
    game_data
        .player
        .records
        .iter()
        .enumerate()
        .filter_map(|(idx, player)| {
            (player.owner_mode_raw() == (idx + 1) as u8).then_some((idx + 1) as u8)
        })
        .collect()
}

fn visible_report_record_count(
    broadcast_entries: &[ReportEntry],
    viewer_entries: Option<&Vec<ReportEntry>>,
) -> usize {
    broadcast_entries
        .iter()
        .chain(
            viewer_entries
                .into_iter()
                .flat_map(|entries| entries.iter()),
        )
        .map(|entry| classic_results_record_count(&entry.text, entry.kind))
        .sum()
}

fn clone_report_entry(entry: &ReportEntry) -> ReportEntry {
    ReportEntry {
        text: entry.text.clone(),
        kind: entry.kind,
        tail: entry.tail,
        target: entry.target,
        repeat_next_pointer: entry.repeat_next_pointer,
    }
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
#[allow(dead_code)]
pub(crate) fn build_rankings_text(game_data: &CoreGameData) -> String {
    let year = game_data.conquest.game_year();
    let stardate = crate::maint::timing::format_rankings_stardate(year);
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
