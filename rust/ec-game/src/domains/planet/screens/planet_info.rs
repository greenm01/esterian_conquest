use crossterm::event::KeyEvent;
use ec_data::{
    IntelTier, PlanetIntelSnapshot, ProductionItemKind,
    build_player_starmap_projection_from_snapshots,
};
use ec_engine::yearly_tax_revenue;

use crate::app::Action;
use crate::screen::layout::{
    aligned_label_width, dismiss_prompt_row, draw_aligned_detail_line, draw_aligned_status_line,
    draw_dismiss_prompt, draw_title_bar, new_playfield,
};
use crate::screen::{
    CommandMenu, PlanetBuildOrder, PlayfieldBuffer, ScreenFrame, format_sector_coords_zero_padded,
};

pub struct PlanetInfoScreen;

impl PlanetInfoScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        planet_idx: usize,
        planet_scorch_orders: &BTreeSet<usize>,
        _menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let planet = frame
            .game_data
            .planets
            .records
            .get(planet_idx)
            .ok_or("planet detail missing")?;
        if planet.owner_empire_slot_raw() as usize != frame.player.record_index_1_based {
            return self.render_intel_detail(frame, planet_idx);
        }

        let [x, y] = planet.coords_raw();
        let owner_empire_raw = planet.owner_empire_slot_raw();
        let owner_label = owner_summary(frame, owner_empire_raw);
        let state_label = owner_state_summary(frame, owner_empire_raw);
        let present = planet.present_production_points().unwrap_or(0);
        let potential = planet.potential_production_points();
        let efficiency = if potential == 0 {
            0.0
        } else {
            (present as f64 / potential as f64) * 100.0
        };
        let tax_rate =
            frame.game_data.player.records[frame.player.record_index_1_based - 1].tax_rate();
        let expected_revenue = yearly_tax_revenue(present, tax_rate);
        let top_label_width = aligned_label_width([
            "Coordinates",
            "Planet",
            "Owner",
            "State",
            "Present Production",
            "Potential Production",
            "Stored Production Points",
            "Efficiency",
            "Expected Revenue",
        ]);
        let detail_label_width =
            aligned_label_width(["Armies", "Ground Batteries", "Space Forces", "Status"]);
        let bottom_label_width = aligned_label_width(["Build Queue", "Stardock"]);

        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "INFO ABOUT A PLANET:");
        draw_aligned_status_line(
            &mut buffer,
            2,
            top_label_width,
            "Coordinates",
            &format_sector_coords_zero_padded([x, y]),
        );
        draw_aligned_status_line(
            &mut buffer,
            3,
            top_label_width,
            "Planet",
            &planet.status_or_name_summary(),
        );
        draw_aligned_status_line(&mut buffer, 4, top_label_width, "Owner", &owner_label);
        draw_aligned_status_line(&mut buffer, 5, top_label_width, "State", &state_label);
        draw_aligned_status_line(
            &mut buffer,
            6,
            top_label_width,
            "Present Production",
            &present.to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            7,
            top_label_width,
            "Potential Production",
            &potential.to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            8,
            top_label_width,
            "Stored Production Points",
            &planet.stored_production_points().to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            9,
            top_label_width,
            "Efficiency",
            &format!("{efficiency:.1}%"),
        );
        draw_aligned_status_line(
            &mut buffer,
            10,
            top_label_width,
            "Expected Revenue",
            &format!("{expected_revenue} points"),
        );
        draw_aligned_status_line(
            &mut buffer,
            12,
            detail_label_width,
            "Armies",
            &planet.army_count_raw().to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            13,
            detail_label_width,
            "Ground Batteries",
            &planet.ground_batteries_raw().to_string(),
        );
        draw_aligned_status_line(
            &mut buffer,
            14,
            detail_label_width,
            "Space Forces",
            &format_owned_orbit_summary(frame, [x, y]),
        );
        draw_aligned_status_line(
            &mut buffer,
            15,
            detail_label_width,
            "Status",
            &owned_status_summary(frame, planet_idx, [x, y], planet_scorch_orders),
        );
        draw_aligned_detail_line(
            &mut buffer,
            17,
            bottom_label_width,
            "Build Queue",
            "  ",
            &format_build_queue_summary(planet),
        );
        draw_aligned_detail_line(
            &mut buffer,
            18,
            bottom_label_width,
            "Stardock",
            "  ",
            &format_stardock_summary(planet),
        );
        draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(18));
        Ok(buffer)
    }

    fn render_intel_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        planet_idx: usize,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection_from_snapshots(
            frame.game_data,
            frame.planet_intel_snapshots,
            frame.player.record_index_1_based as u8,
        );
        let world = projection
            .worlds
            .into_iter()
            .find(|world| world.planet_record_index_1_based == planet_idx + 1)
            .ok_or("planet intel detail missing")?;
        let owner_label = world
            .known_owner_empire_name
            .clone()
            .or_else(|| {
                world
                    .known_owner_empire_id
                    .map(|id| format!("Empire #{id}"))
            })
            .unwrap_or_else(|| "?".to_string());
        let top_label_width = aligned_label_width([
            "Coordinates",
            "Planet",
            "Owner",
            "State",
            "Last Viewed/Scouted",
            "Present Production",
            "Potential Production",
            "Efficiency",
            "Stored Production Points",
            "Armies",
            "Ground Batteries",
            "Space Forces",
            "Intel Tier",
        ]);
        let bottom_label_width = aligned_label_width(["Docked"]);

        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "INFO ABOUT A PLANET:");
        draw_aligned_status_line(
            &mut buffer,
            2,
            top_label_width,
            "Coordinates",
            &format_sector_coords_zero_padded(world.coords),
        );
        draw_aligned_status_line(
            &mut buffer,
            3,
            top_label_width,
            "Planet",
            world.known_name.as_deref().unwrap_or("?"),
        );
        draw_aligned_status_line(&mut buffer, 4, top_label_width, "Owner", &owner_label);
        draw_aligned_status_line(&mut buffer, 5, top_label_width, "State", "?");
        let intel_snapshot = frame.planet_intel_snapshots.get(&(planet_idx + 1));
        draw_aligned_status_line(
            &mut buffer,
            6,
            top_label_width,
            "Last Viewed/Scouted",
            &intel_snapshot
                .and_then(|snapshot| snapshot.last_intel_year)
                .map(|year| format!("Y{year}"))
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            7,
            top_label_width,
            "Present Production",
            &world
                .known_current_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            8,
            top_label_width,
            "Potential Production",
            &world
                .known_potential_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            9,
            top_label_width,
            "Efficiency",
            &intel_efficiency_label(
                world.known_current_production,
                world.known_potential_production,
            ),
        );
        draw_aligned_status_line(
            &mut buffer,
            10,
            top_label_width,
            "Stored Production Points",
            &world
                .known_stored_points
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            12,
            top_label_width,
            "Armies",
            &world
                .known_armies
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            13,
            top_label_width,
            "Ground Batteries",
            &world
                .known_ground_batteries
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_aligned_status_line(
            &mut buffer,
            14,
            top_label_width,
            "Space Forces",
            world.known_orbit_summary.as_deref().unwrap_or("?"),
        );
        draw_aligned_status_line(
            &mut buffer,
            15,
            top_label_width,
            "Intel Tier",
            intel_tier_label(intel_snapshot, &world),
        );
        draw_aligned_detail_line(
            &mut buffer,
            17,
            bottom_label_width,
            "Docked",
            "  ",
            world.known_docked_summary.as_deref().unwrap_or("?"),
        );
        draw_dismiss_prompt(&mut buffer, dismiss_prompt_row(17));
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, _key: KeyEvent) -> Action {
        Action::Noop
    }

    pub fn handle_detail_key(&self, _key: KeyEvent) -> Action {
        Action::ReturnToCommandMenu
    }
}

fn intel_efficiency_label(current: Option<u8>, potential: Option<u16>) -> String {
    match (current, potential) {
        (Some(current), Some(potential)) if potential != 0 => {
            format!(
                "{:.1}%",
                (f64::from(current) / f64::from(potential)) * 100.0
            )
        }
        _ => "?".to_string(),
    }
}

fn format_stardock_summary(planet: &ec_data::PlanetRecord) -> String {
    let mut parts = Vec::new();
    for slot in 0..ec_data::STARDOCK_SLOT_COUNT {
        let count = u32::from(planet.stardock_count_raw(slot));
        if count == 0 {
            continue;
        }
        let kind = planet.stardock_item_kind_current_known(slot);
        parts.push(format!("{count}{}", compact_unit_code(kind)));
    }
    if parts.is_empty() {
        "Nothing".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_build_queue_summary(planet: &ec_data::PlanetRecord) -> String {
    let orders: Vec<PlanetBuildOrder> = (0..10)
        .filter_map(|slot| {
            let points = planet.build_count_raw(slot);
            let kind_raw = planet.build_kind_raw(slot);
            if points == 0 || kind_raw == 0 {
                None
            } else {
                Some(PlanetBuildOrder {
                    kind: ProductionItemKind::from_raw(kind_raw),
                    points_remaining: points,
                })
            }
        })
        .collect();
    if orders.is_empty() {
        "Nothing".to_string()
    } else {
        orders
            .into_iter()
            .map(|order| {
                format!(
                    "{}{}",
                    order.points_remaining,
                    compact_unit_code(order.kind)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_owned_orbit_summary(frame: &ScreenFrame<'_>, coords: [u8; 2]) -> String {
    let fleet_count = frame
        .game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| {
            fleet.current_location_coords_raw() == coords
                && fleet.owner_empire_raw() as usize == frame.player.record_index_1_based
                && fleet_has_any_force(fleet)
        })
        .count();
    let starbase_count = frame
        .game_data
        .bases
        .records
        .iter()
        .filter(|base| {
            base.coords_raw() == coords
                && base.owner_empire_raw() as usize == frame.player.record_index_1_based
                && base.active_flag_raw() != 0
        })
        .count();
    let mut parts = Vec::new();
    if fleet_count > 0 {
        parts.push(format!(
            "{} {}",
            fleet_count,
            if fleet_count == 1 { "fleet" } else { "fleets" }
        ));
    }
    if starbase_count > 0 {
        parts.push(format!(
            "{} {}",
            starbase_count,
            if starbase_count == 1 {
                "starbase"
            } else {
                "starbases"
            }
        ));
    }
    if parts.is_empty() {
        "Nothing".to_string()
    } else {
        parts.join(", ")
    }
}

fn fleet_has_any_force(fleet: &ec_data::FleetRecord) -> bool {
    fleet.scout_count() > 0
        || fleet.battleship_count() > 0
        || fleet.cruiser_count() > 0
        || fleet.destroyer_count() > 0
        || fleet.troop_transport_count() > 0
        || fleet.army_count() > 0
        || fleet.etac_count() > 0
}

fn compact_unit_code(kind: ec_data::ProductionItemKind) -> &'static str {
    match kind {
        ec_data::ProductionItemKind::Destroyer => "DD",
        ec_data::ProductionItemKind::Cruiser => "CA",
        ec_data::ProductionItemKind::Battleship => "BB",
        ec_data::ProductionItemKind::Scout => "SC",
        ec_data::ProductionItemKind::Transport => "TT",
        ec_data::ProductionItemKind::Etac => "ET",
        ec_data::ProductionItemKind::Army => "AR",
        ec_data::ProductionItemKind::GroundBattery => "GB",
        ec_data::ProductionItemKind::Starbase => "SB",
        ec_data::ProductionItemKind::Unknown(_) => "UN",
    }
}

fn owned_status_summary(
    frame: &ScreenFrame<'_>,
    planet_idx: usize,
    coords: [u8; 2],
    planet_scorch_orders: &BTreeSet<usize>,
) -> String {
    if planet_scorch_orders.contains(&(planet_idx + 1)) {
        return "Planet is scorched!".to_string();
    }
    let planet = &frame.game_data.planets.records[planet_idx];
    if planet.is_homeworld_seed_ignoring_name() {
        return "Homeworld - fully developed".to_string();
    }
    if frame
        .game_data
        .planet_has_friendly_starbase(frame.player.record_index_1_based as u8, coords)
    {
        return "Regular planet - starbase present".to_string();
    }
    "Regular planet - factories fully functional".to_string()
}

fn owner_summary(frame: &ScreenFrame<'_>, owner_empire_raw: u8) -> String {
    if owner_empire_raw == 0 {
        return "Unowned".to_string();
    }

    let idx = owner_empire_raw as usize - 1;
    let Some(player) = frame.game_data.player.records.get(idx) else {
        return format!("Empire #{owner_empire_raw}");
    };

    let empire_name = player.controlled_empire_name_summary();
    let legacy_name = player.legacy_status_name_summary();
    if !empire_name.is_empty() && !legacy_name.starts_with("In Civil Disorder") {
        format!("Empire #{owner_empire_raw} ({empire_name})")
    } else if !legacy_name.is_empty() {
        format!("Empire #{owner_empire_raw} ({legacy_name})")
    } else {
        format!("Empire #{owner_empire_raw}")
    }
}

fn owner_state_summary(frame: &ScreenFrame<'_>, owner_empire_raw: u8) -> String {
    match frame.game_data.empire_campaign_state(owner_empire_raw) {
        Some(ec_data::CampaignState::Stable) => "Stable".to_string(),
        Some(ec_data::CampaignState::MarginalExistence) => "Marginal Existence".to_string(),
        Some(ec_data::CampaignState::DefectionRisk) => "Defection Risk".to_string(),
        Some(ec_data::CampaignState::Defeated) => "Defeated".to_string(),
        Some(ec_data::CampaignState::CivilDisorder) => "In Civil Disorder".to_string(),
        Some(ec_data::CampaignState::Rogue) => "Rogue".to_string(),
        None if owner_empire_raw == 0 => "Unowned".to_string(),
        None => "Unknown".to_string(),
    }
}

fn intel_tier_label<'a>(
    snapshot: Option<&'a PlanetIntelSnapshot>,
    world: &'a ec_data::PlayerStarmapWorld,
) -> &'a str {
    match snapshot.map(|snapshot| snapshot.intel_tier) {
        Some(IntelTier::Owned) => "owned",
        Some(IntelTier::Full) => "full",
        Some(IntelTier::Partial) => "partial",
        Some(IntelTier::Unknown) => "unknown",
        None if world.known_armies.is_some() || world.known_ground_batteries.is_some() => "full",
        None if world.known_name.is_some()
            || world.known_owner_empire_id.is_some()
            || world.known_owner_empire_name.is_some()
            || world.known_potential_production.is_some() =>
        {
            "partial"
        }
        None => "unknown",
    }
}

pub fn parse_planet_coords(input: &str) -> Option<[u8; 2]> {
    let parts = input
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }
    let x = parts[0].parse::<u8>().ok()?;
    let y = parts[1].parse::<u8>().ok()?;
    Some([x, y])
}
use std::collections::BTreeSet;
