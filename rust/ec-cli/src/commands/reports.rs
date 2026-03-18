use ec_data::{
    ContactReportSource, CoreGameData, DatabaseDat, EmpireProductionRankingSort,
    FleetOrderValidationError, FleetPlayerInputValidationError, MaintenanceEvents, Mission,
    MissionOutcome, PlanetDat, PlanetPlayerInputValidationError, PlayerDiplomacyValidationError,
    QueuedPlayerMail, ShipLosses,
};
use ec_data::maint::timing::format_report_first_line;

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
    let mut new_database =
        DatabaseDat::generate_from_planets_and_year(&planet_names, year, player_count, template);

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
        _ => "Scouting mission report",
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

fn planet_defense_summary(batteries: u8, armies: u8) -> String {
    format!("{batteries} ground battery(ies) and {armies} army(ies)")
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
    MessagesOnly { recipient: u8 },
    /// Goes into both; RESULTS.DAT gets unrouted text, MESSAGES.DAT routes to `recipient`.
    Both { recipient: u8 },
}

struct ReportEntry {
    text: String,
    kind: u8,
    tail: [u8; 8],
    target: ReportTarget,
}

/// Build the right-justified Stardate first-line header for a report entry.
///
/// `source_clause` should end with `:` (e.g. `"From your fleet in System(1,2):"`)
/// and the week/year are formatted as `Stardate: week/year` right-justified to
/// column 75.
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
        let source = format!("From planet \"{}\" in System({x},{y}):", planet.planet_name());
        let header = report_header(&source, event.stardate_week, year);
        let body = format!(
            " We have been bombarded by {}. The attacking fleet initially appeared to contain {}. Our defenses initially contained {}. We observed losses of {} ground batteries and {} armies.",
            empire_label(game_data, event.attacker_empire_raw),
            ship_loss_summary(event.attacker_initial),
            planet_defense_summary(event.defender_batteries_initial, event.defender_armies_initial),
            event.defender_battery_losses,
            event.defender_army_losses,
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x08,
            tail: RESULTS_TAIL_BOMBARD,
            target: ReportTarget::Both { recipient: event.defender_empire_raw },
        });
    }

    // ----- Fleet battle events -----
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
        let source = format!("From your fleet in System({x},{y}):");
        let header = report_header(&source, event.stardate_week, year);
        let body = format!(
            " Fleet battle report. We engaged hostile forces belonging to {enemies}. Initial observed hostile composition: {}. Friendly losses: {}. Observed enemy losses: {}. {outcome}",
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.friendly_losses),
            ship_loss_summary(event.enemy_losses),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient: event.reporting_empire_raw },
        });
    }

    // ----- Fleet destroyed events -----
    for event in &events.fleet_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .map(|empire| empire_label(game_data, empire))
            .unwrap_or_else(|| "an alien fleet".to_string());
        let verb = if event.was_intercepting { "intercepted" } else { "was attacked by" };
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " We lost all contact with the {}th Fleet shortly after it {} {} in System({x},{y}). Records show the fleet was composed of {} and carried {} armies. According to a burnt flight recorder we recovered, the alien force initially contained {}. The flight recorder recorded alien ship casualties of {}.",
            event.fleet_id,
            verb,
            enemy,
            ship_loss_summary(event.friendly_initial),
            event.friendly_armies,
            ship_loss_summary(event.enemy_initial),
            ship_loss_summary(event.enemy_losses),
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient: event.reporting_empire_raw },
        });
    }

    // ----- Starbase destroyed events -----
    for event in &events.starbase_destroyed_events {
        let [x, y] = event.coords;
        let enemy = event
            .primary_enemy_empire_raw
            .map(|empire| empire_label(game_data, empire))
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
            target: ReportTarget::Both { recipient: event.reporting_empire_raw },
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
            target: ReportTarget::Both { recipient: event.reporting_empire_raw },
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
        });
    }

    // ----- Fleet defection events -----
    for event in &events.fleet_defection_events {
        let source = "From your Fleet Command Center:";
        let header = report_header(source, event.stardate_week, year);
        let body = format!(
            " We have lost all contact with the {}th Fleet. In the chaos of civil disorder, the surviving crews have defected and no longer answer to central command.",
            event.fleet_id,
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient: event.reporting_empire_raw },
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
            " Our cover fire failed to suppress the defending batteries before the descent.".to_string()
        };
        let source = format!("From your fleet in System({x},{y}):");
        let header = report_header(&source, event.stardate_week, year);
        let body = match (event.kind, event.outcome) {
            (Mission::InvadeWorld, MissionOutcome::Succeeded) => format!(
                " Invasion mission report: Our armies have captured planet \"{}\". The defending world initially contained {}. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet.planet_name(),
                planet_defense_summary(event.defender_batteries_initial, event.defender_armies_initial),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Failed) => format!(
                " Invasion mission report: The landing was repulsed. The defending world initially contained {}. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet_defense_summary(event.defender_batteries_initial, event.defender_armies_initial),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::InvadeWorld, MissionOutcome::Aborted) => format!(
                " Invasion mission report: Enemy ground batteries prevented a landing. The defending world initially contained {}. Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.",
                planet_defense_summary(event.defender_batteries_initial, event.defender_armies_initial),
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
            ),
            (Mission::BlitzWorld, MissionOutcome::Succeeded) => format!(
                " Blitz mission report: We have seized planet \"{}\" in a fast assault. The defending world initially contained {}.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                planet.planet_name(),
                planet_defense_summary(event.defender_batteries_initial, event.defender_armies_initial),
                blitz_cover_note,
                ship_losses,
                event.attacker_army_losses,
                event.defender_battery_losses,
                event.defender_army_losses,
                transport_note,
            ),
            (Mission::BlitzWorld, MissionOutcome::Failed) => format!(
                " Blitz mission report: The blitz attack failed. The defending world initially contained {}.{} Friendly losses: {} and {} armies. Enemy losses: {} ground batteries and {} armies.{}",
                planet_defense_summary(event.defender_batteries_initial, event.defender_armies_initial),
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
            target: ReportTarget::Both { recipient: event.attacker_empire_raw },
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
                let source = format!("From your fleet in {location}:");
                let header = report_header(&source, event.stardate_week, year);
                let contact_body = format!(
                    " {label}: Sensor contact shows an alien fleet in {location} traveling at sublight speed. Closing to check it out..."
                );
                entries.push(ReportEntry {
                    text: format!("{header}{contact_body}"),
                    kind: 0x07,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both { recipient: event.viewer_empire_raw },
                });
                let identified_body = format!(
                    " {label}: We have located and identified the alien fleet in {location}. It belongs to {}. Their fleet contains {size_summary} of unknown type. Ignoring alien fleet...",
                    empire_label(game_data, event.target_empire_raw),
                );
                entries.push(ReportEntry {
                    text: format!("{header}{identified_body}"),
                    kind: 0x07,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both { recipient: event.viewer_empire_raw },
                });
            }
            ContactReportSource::Fleet(fleet_id) => {
                let source = format!("From Fleet {fleet_id} in System({x},{y}):");
                let header = report_header(&source, event.stardate_week, year);
                let body = format!(
                    " Contact report: We have encountered an alien fleet in System({x},{y}). It belongs to {}. Their fleet contains {size_summary} of unknown type.",
                    empire_label(game_data, event.target_empire_raw),
                );
                entries.push(ReportEntry {
                    text: format!("{header}{body}"),
                    kind: 0x07,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both { recipient: event.viewer_empire_raw },
                });
            }
            ContactReportSource::Starbase(starbase_id) => {
                let source = format!("From Starbase {starbase_id}, located in System({x},{y}):");
                let header = report_header(&source, event.stardate_week, year);
                let body = format!(
                    " We have located and identified an alien fleet in System({x},{y}). It is {}. Their fleet contains {size_summary} of unknown type. We are alerting all fleets in the area.",
                    empire_label(game_data, event.target_empire_raw),
                );
                entries.push(ReportEntry {
                    text: format!("{header}{body}"),
                    kind: 0x07,
                    tail: RESULTS_TAIL_SCOUTING,
                    target: ReportTarget::Both { recipient: event.viewer_empire_raw },
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
            empire_label(game_data, event.previous_owner_empire_raw)
        };
        let source = format!("From planet \"{}\" in System({x},{y}):", planet.planet_name());
        let header = report_header(&source, event.stardate_week, year);
        let body = format!(
            " We have been invaded and captured by {} from {}.",
            empire_label(game_data, event.new_owner_empire_raw),
            from
        );
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x0c,
            tail: RESULTS_TAIL_INVASION,
            target: ReportTarget::Both { recipient: event.reporting_empire_raw },
        });
    }

    // ----- Colonization events -----
    for event in &events.colonization_events {
        let (planet_idx, colonizer_empire_raw, event_week) = match *event {
            ec_data::ColonizationResolvedEvent::Succeeded { planet_idx, colonizer_empire_raw, stardate_week, .. } => {
                (planet_idx, colonizer_empire_raw, stardate_week)
            }
            ec_data::ColonizationResolvedEvent::BlockedByOwner { planet_idx, colonizer_empire_raw, stardate_week, .. } => {
                (planet_idx, colonizer_empire_raw, stardate_week)
            }
        };
        let Some(planet) = game_data.planets.records.get(planet_idx) else {
            continue;
        };
        let [x, y] = planet.coords_raw();
        let source = format!("From colony mission in System({x},{y}):");
        let header = report_header(&source, event_week, year);
        let body = match *event {
            ec_data::ColonizationResolvedEvent::Succeeded { .. } => format!(
                " We have successfully established a colony on planet \"{}\" for {}.",
                planet.planet_name(),
                empire_label(game_data, colonizer_empire_raw),
            ),
            ec_data::ColonizationResolvedEvent::BlockedByOwner { owner_empire_raw, .. } => format!(
                " {} could not establish a colony on planet \"{}\" because it is already occupied by {}.",
                empire_label(game_data, colonizer_empire_raw),
                planet.planet_name(),
                empire_label(game_data, owner_empire_raw),
            ),
        };
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x09,
            tail: RESULTS_TAIL_COLONIZATION,
            target: ReportTarget::Both { recipient: colonizer_empire_raw },
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
        let (kind, tail, source, body) = match (event.kind, event.outcome) {
            (Mission::MoveOnly, MissionOutcome::Succeeded) => (
                0x05u8,
                RESULTS_TAIL_FLEET,
                format!("From your fleet in {}:", mission_location_phrase(event.kind, coords)),
                " Move mission report: We have arrived at our destination and await new orders.".to_string(),
            ),
            (Mission::RendezvousSector, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_FLEET,
                format!("From your fleet in Sector({x},{y}):"),
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
                    format!("From your fleet in System({x},{y}):"),
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
                    format!("From your fleet in System({x},{y}):"),
                    body,
                )
            }
            (Mission::PatrolSector, MissionOutcome::Arrived) => (
                0x05,
                RESULTS_TAIL_FLEET,
                format!("From your fleet in Sector({x},{y}):"),
                " Patrol mission report: We have arrived in our assigned sector and are beginning patrol operations.".to_string(),
            ),
            (Mission::SeekHome, MissionOutcome::Succeeded) => (
                0x05,
                RESULTS_TAIL_FLEET,
                format!("From your fleet in System({x},{y}):"),
                " Seek Home mission report: We have reached a friendly world and are awaiting new orders.".to_string(),
            ),
            (Mission::BombardWorld, MissionOutcome::Arrived) => (
                0x08,
                RESULTS_TAIL_BOMBARD,
                format!("From your fleet in System({x},{y}):"),
                " Bombardment mission report: We have arrived at our target world and are preparing for bombardment.".to_string(),
            ),
            (Mission::InvadeWorld, MissionOutcome::Arrived) => (
                0x08,
                RESULTS_TAIL_INVASION,
                format!("From your fleet in System({x},{y}):"),
                " Invasion mission report: We have arrived at our target world and are preparing to begin the invasion.".to_string(),
            ),
            (Mission::BlitzWorld, MissionOutcome::Arrived) => (
                0x08,
                RESULTS_TAIL_INVASION,
                format!("From your fleet in System({x},{y}):"),
                " Blitz mission report: We have arrived at our target world and are preparing to launch the assault.".to_string(),
            ),
            (Mission::MoveOnly, MissionOutcome::Aborted) => {
                let destination = fleet.standing_order_target_coords_raw();
                let [dx, dy] = destination;
                (
                    0x05,
                    RESULTS_TAIL_FLEET,
                    format!("From your fleet in {}:", mission_location_phrase(event.kind, coords)),
                    format!(" Move mission report: Hostile action forced us to abort our mission and seek safety in System({dx},{dy})."),
                )
            }
            (Mission::ViewWorld, MissionOutcome::Succeeded) => {
                let body = if let Some(planet_idx) = event.planet_idx {
                    if let Some(planet) = game_data.planets.records.get(planet_idx) {
                        let ownership = if planet.owner_empire_slot_raw() == 0 {
                            "unowned".to_string()
                        } else {
                            format!("owned by {}", empire_label(game_data, planet.owner_empire_slot_raw()))
                        };
                        format!(
                            " Viewing mission report: We have entered System({x},{y}) and completed a long range analysis of planet \"{}\". The world is {} and has a potential of {} points. Until ordered otherwise, we will be moving out of the solar system.",
                            planet.planet_name(),
                            ownership,
                            u16::from_le_bytes(planet.potential_production_raw()),
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
                    format!("From your fleet in System({x},{y}):"),
                    body,
                )
            }
            (Mission::ViewWorld, MissionOutcome::Failed) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                format!("From your fleet in System({x},{y}):"),
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
                    format!("From your fleet in System({x},{y}):"),
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
                            " Bombardment mission report: We have concluded our bombing run against planet \"{}\". The defending world initially contained {}. Friendly losses: {}. Observed enemy losses: {} ground batteries and {} armies.",
                            planet.planet_name(),
                            bombard_event
                                .map(|e| planet_defense_summary(e.defender_batteries_initial, e.defender_armies_initial))
                                .unwrap_or_else(|| "unknown defenses".to_string()),
                            bombard_event.map(|e| ship_loss_summary(e.attacker_losses)).unwrap_or_else(|| "no ship losses".to_string()),
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
                    format!("From your fleet in System({x},{y}):"),
                    body,
                )
            }
            (Mission::InvadeWorld, _) | (Mission::BlitzWorld, _) => continue,
            (Mission::ScoutSector, MissionOutcome::Succeeded) => (
                0x07,
                RESULTS_TAIL_SCOUTING,
                format!("From your fleet in Sector({x},{y}):"),
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
                    format!("From your fleet in Sector({x},{y}):"),
                    format!(" Scouting mission report: Hostile action forced us to abort our scouting mission and withdraw toward {retreat}."),
                )
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Succeeded) => {
                let body = if let Some(planet) = game_data
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
                    let stardock_summary = if (0..10).any(|slot| planet.stardock_count_raw(slot) > 0) {
                        "The planet's stardock contains ships."
                    } else {
                        "The planet's stardock appears to be empty."
                    };
                    format!(
                        " Scouting mission report: We are in extended orbit around planet \"{}\". Owner: {}. Potential production: {} points. Stored goods: {} points. Armies: {}. Ground batteries: {}. {}",
                        planet.planet_name(),
                        owner,
                        planet.potential_production_raw()[0],
                        planet.stored_goods_raw(),
                        planet.army_count_raw(),
                        planet.ground_batteries_raw(),
                        stardock_summary,
                    )
                } else {
                    format!(" Scouting mission report: We have arrived at our destination and are beginning to scout this solar system.")
                };
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    format!("From your fleet in System({x},{y}):"),
                    body,
                )
            }
            (Mission::ScoutSolarSystem, MissionOutcome::Aborted) => {
                let retreat = event
                    .target_coords
                    .map(|coords| nearest_owned_destination_text(game_data, event.owner_empire_raw, coords))
                    .unwrap_or_else(|| "the nearest friendly controlled solar system".to_string());
                (
                    0x07,
                    RESULTS_TAIL_SCOUTING,
                    format!("From your fleet in System({x},{y}):"),
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
            target: ReportTarget::Both { recipient: event.owner_empire_raw },
        });
    }

    // ----- Salvage events -----
    for event in &events.salvage_events {
        let (owner_empire_raw, event_week, source, body) = match *event {
            ec_data::SalvageResolvedEvent::Succeeded {
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
                    format!("From your fleet in System({x},{y}):"),
                    format!(" Salvage mission report: We have arrived at planet \"{planet_name}\" in System({x},{y}) and have begun salvaging our fleet. We estimate that our fleet will yield {recovered_points} production point(s)."),
                )
            }
            ec_data::SalvageResolvedEvent::Failed {
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
                    format!("From your fleet in System({x},{y}):"),
                    format!(" Salvage mission report: We have arrived at planet \"{planet_name}\" in System({x},{y}), but it is not under our control so we cannot salvage our fleet there."),
                )
            }
            ec_data::SalvageResolvedEvent::Failed {
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
                    format!("From your fleet in System({x},{y}):"),
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
            target: ReportTarget::Both { recipient: owner_empire_raw },
        });
    }

    // ----- Encounter disposition events (ROE) -----
    for event in &events.encounter_disposition_events {
        let (owner_empire_raw, event_week, source, body) = match *event {
            ec_data::EncounterDispositionEvent::NoEngagement {
                owner_empire_raw,
                coords,
                target_empire_raw,
                small_vessels,
                medium_vessels,
                large_vessels,
                stardate_week,
                ..
            } => (
                owner_empire_raw,
                stardate_week,
                format!("From your fleet in Sector({},{}) :", coords[0], coords[1]),
                format!(
                    " Fleet encounter report: We detected hostile forces from {} but declined battle under our current ROE. Initial observed hostile composition: {}.",
                    empire_label(game_data, target_empire_raw),
                    contact_size_summary_from_counts(small_vessels, medium_vessels, large_vessels)
                ),
            ),
            ec_data::EncounterDispositionEvent::Retreated {
                owner_empire_raw,
                coords,
                target_empire_raw,
                enemy_initial,
                retreat_target_coords,
                losses_sustained,
                enemy_losses_inflicted,
                stardate_week,
                ..
            } => (
                owner_empire_raw,
                stardate_week,
                format!("From your fleet in Sector({},{}) :", coords[0], coords[1]),
                format!(
                    " Fleet encounter report: After engaging hostile forces from {}, we withdrew under our ROE toward System({},{}) after suffering losses of {}. Initial observed hostile composition: {}. We observed enemy losses of {}.",
                    empire_label(game_data, target_empire_raw),
                    retreat_target_coords[0],
                    retreat_target_coords[1],
                    ship_loss_summary(losses_sustained),
                    ship_loss_summary(enemy_initial),
                    ship_loss_summary(enemy_losses_inflicted)
                ),
            ),
        };
        let header = report_header(&source, event_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient: owner_empire_raw },
        });
    }

    // ----- Invalid player state events -----
    for event in &events.invalid_player_state_events {
        let (owner_empire_raw, source, body) = match *event {
            ec_data::InvalidPlayerStateEvent::FleetMission { owner_empire_raw, coords, reason, .. } => (
                owner_empire_raw,
                format!("From your fleet in Sector({},{}) :", coords[0], coords[1]),
                format!(
                    " Order validation report: Maintenance canceled this fleet's orders because {}. The fleet is holding position awaiting new orders.",
                    fleet_order_validation_reason_text(reason)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::FleetInput { owner_empire_raw, coords, reason, .. } => (
                owner_empire_raw,
                format!("From your fleet in Sector({},{}) :", coords[0], coords[1]),
                format!(
                    " Fleet readiness report: Maintenance corrected invalid fleet input because {}.",
                    fleet_player_input_validation_reason_text(reason)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::PlanetInput { owner_empire_raw, coords, reason, .. } => (
                owner_empire_raw.max(1),
                format!("From planet in System({},{}) :", coords[0], coords[1]),
                format!(
                    " Administration report: Maintenance cleared invalid player input because {}.",
                    planet_input_validation_reason_text(reason)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::PlayerTaxRate { owner_empire_raw, tax_rate, .. } => (
                owner_empire_raw,
                "From your central administration:".to_string(),
                format!(
                    " Tax rate input {}% for {} was invalid and has been clamped to 100%.",
                    tax_rate,
                    empire_label(game_data, owner_empire_raw)
                ),
            ),
            ec_data::InvalidPlayerStateEvent::DiplomacyInput { owner_empire_raw, reason, .. } => (
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
            target: ReportTarget::Both { recipient: owner_empire_raw },
        });
    }

    // ----- Fleet merge events -----
    for event in &events.fleet_merge_events {
        let [x, y] = event.coords;
        let (source, body) = match event.kind {
            Mission::JoinAnotherFleet => (
                format!("From your fleet in System({x},{y}):"),
                format!(
                    " Join mission report: We have joined the {}th Fleet and are now merging with them.",
                    event.host_fleet_id
                ),
            ),
            Mission::RendezvousSector if event.survivor_side => (
                format!("From your fleet in Sector({x},{y}):"),
                format!(
                    " Rendezvous mission report: We have arrived at the our rendezvous point and are absorbing the {}th Fleet.",
                    event.absorbed_fleet_id
                ),
            ),
            Mission::RendezvousSector => (
                format!("From your fleet in Sector({x},{y}):"),
                format!(
                    " Rendezvous mission report: We have arrived at the our rendezvous point and are merging with the {}th Fleet.",
                    event.host_fleet_id
                ),
            ),
            _ => continue,
        };
        let header = report_header(&source, event.stardate_week, year);
        entries.push(ReportEntry {
            text: format!("{header}{body}"),
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient: event.owner_empire_raw },
        });
    }

    // ----- Join host events -----
    for event in &events.join_host_events {
        let (recipient, source, body) = match *event {
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
                    format!("From your fleet in Sector({x},{y}):"),
                    format!(
                        " Join mission report: Our intended host fleet ({}th Fleet) has moved. We are now joining the {}th Fleet instead.",
                        previous_host_fleet_id, new_host_fleet_id
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
                    format!("From your fleet in Sector({x},{y}):"),
                    format!(
                        " Join mission report: In light of the destruction of the {destroyed_host_fleet_id}th Fleet, we are holding our current position in Sector({x},{y}) and are awaiting new orders."
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
        });
    }

    // ----- Mission retarget events -----
    for event in &events.mission_retarget_events {
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
                    "Fleet mission report: Our original refuge at Sector({},{}) is no longer suitable, so we are now seeking home at Sector({},{}) instead.",
                    previous_target_coords[0], previous_target_coords[1],
                    new_target_coords[0], new_target_coords[1]
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
                    "Fleet mission report: With no owned planets remaining, we are holding our current position in Sector({},{}) and are awaiting new orders.",
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
                    previous_target_coords[0], previous_target_coords[1],
                    new_target_coords[0], new_target_coords[1]
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
                    previous_target_coords[0], previous_target_coords[1],
                    new_target_coords[0], new_target_coords[1]
                ),
            ),
            _ => continue,
        };
        // Retarget events have no source clause; emit body directly without Stardate header.
        entries.push(ReportEntry {
            text: body,
            kind: 0x05,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::Both { recipient },
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
            target: ReportTarget::MessagesOnly { recipient: event.left_empire_raw },
        });
        let right_body = format!(
            " Hostile action has escalated our relations with {} to enemy status.",
            empire_label(game_data, event.left_empire_raw),
        );
        entries.push(ReportEntry {
            text: format!("{left_header}{right_body}"),
            kind: 0x06,
            tail: RESULTS_TAIL_FLEET,
            target: ReportTarget::MessagesOnly { recipient: event.right_empire_raw },
        });
    }

    entries
}

// ---------------------------------------------------------------------------
// Public builders
// ---------------------------------------------------------------------------

/// Build the RESULTS.DAT binary from current game data and maintenance events.
pub(crate) fn build_results_dat(game_data: &CoreGameData, events: &MaintenanceEvents) -> Vec<u8> {
    let mut results = Vec::new();
    for entry in generate_report_entries(game_data, events) {
        if !matches!(entry.target, ReportTarget::MessagesOnly { .. }) {
            push_results_chunked(&mut results, entry.kind, entry.tail, &entry.text);
        }
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
    let mut messages = Vec::new();

    for entry in generate_report_entries(game_data, events) {
        let recipient = match entry.target {
            ReportTarget::ResultsOnly => continue,
            ReportTarget::MessagesOnly { recipient } => recipient,
            ReportTarget::Both { recipient } => recipient,
        };
        push_routed_message_chunked(
            &mut messages,
            game_data,
            recipient,
            entry.kind,
            entry.tail,
            &entry.text,
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
