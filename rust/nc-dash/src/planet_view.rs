use std::collections::BTreeMap;

use nc_data::{
    CampaignState, CompactUnitSummaryStyle, IntelTier, OwnedPlanetStatus, PlanetIntelSnapshot,
    PlanetRecord, PlayerStarmapWorld, active_starbase_count_at,
    build_player_starmap_projection_from_snapshots,
    format_build_queue_summary as shared_build_queue_summary,
    format_owned_orbit_summary as shared_owned_orbit_summary,
    format_stardock_summary as shared_stardock_summary, owned_orbit_presence, owned_planet_status,
    yearly_tax_revenue,
};

use crate::app::state::DashApp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DetailLine {
    pub label: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectedPlanetDetail {
    pub planet_record_index_1_based: usize,
    pub widget_fields: Vec<DetailLine>,
    pub popup_lines: Vec<DetailLine>,
}

pub(crate) fn selected_planet_detail(app: &DashApp) -> Option<SelectedPlanetDetail> {
    projected_sector_details(app)
        .into_iter()
        .find(|detail| detail.planet_record_index_1_based == selected_planet_record_index(app))
}

pub(crate) fn projected_sector_details(app: &DashApp) -> Vec<SelectedPlanetDetail> {
    let viewer_empire_id = app.player_record_index_1_based as u8;
    let snapshot_map = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let projection = build_player_starmap_projection_from_snapshots(
        &app.game_data,
        &snapshot_map,
        viewer_empire_id,
    );
    projection
        .worlds
        .iter()
        .filter_map(|world| {
            let planet_index_0_based = world.planet_record_index_1_based.checked_sub(1)?;
            let planet = app.game_data.planets.records.get(planet_index_0_based)?;
            Some(if planet.owner_empire_slot_raw() == viewer_empire_id {
                owned_planet_detail(app, planet_index_0_based, planet)
            } else {
                intel_planet_detail(
                    app,
                    world,
                    snapshot_map.get(&world.planet_record_index_1_based),
                )
            })
        })
        .collect()
}

pub(crate) fn preferred_sector_detail_body_width(app: &DashApp) -> usize {
    projected_sector_details(app)
        .into_iter()
        .flat_map(|detail| detail.widget_fields.into_iter())
        .map(|field| preferred_widget_field_width(&field))
        .max()
        .unwrap_or_else(|| "empty sector".chars().count())
}

fn owned_planet_detail(
    app: &DashApp,
    planet_index_0_based: usize,
    planet: &PlanetRecord,
) -> SelectedPlanetDetail {
    let viewer_empire_id = app.player_record_index_1_based as u8;
    let coords = planet.coords_raw();
    let present = planet.present_production_points().unwrap_or(0);
    let potential = planet.potential_production_points();
    let tax_rate = app
        .game_data
        .player
        .records
        .get(app.player_record_index_1_based.saturating_sub(1))
        .map(|player| player.tax_rate())
        .unwrap_or(0);
    let owned_since = app
        .owned_planet_years
        .get(&(planet_index_0_based + 1))
        .map(|year| format!("Y{year}"))
        .unwrap_or_else(|| String::from("?"));
    let budget = app
        .game_data
        .empire_planet_economy_rows(app.player_record_index_1_based)
        .into_iter()
        .find(|row| row.planet_record_index_1_based == planet_index_0_based + 1)
        .map(|row| u32::from(row.build_capacity).min(row.stored_production_points))
        .unwrap_or_else(|| planet.stored_production_points().min(u32::from(present)));
    let popup_lines = vec![
        detail_line("Coordinates", coords_label(coords)),
        detail_line("Planet", planet.status_or_name_summary()),
        detail_line(
            "Owner",
            owned_owner_detail_label(&app.game_data, viewer_empire_id),
        ),
        detail_line(
            "State",
            owner_state_detail_label(&app.game_data, viewer_empire_id),
        ),
        detail_line("Owned Since", owned_since),
        detail_line("Production", present.to_string()),
        detail_line("Potential Production", potential.to_string()),
        detail_line("Treasury", planet.stored_production_points().to_string()),
        detail_line("Budget", budget.to_string()),
        detail_line("Efficiency", owned_efficiency_label(present, potential)),
        detail_line(
            "Expected Revenue",
            format!("{} points", yearly_tax_revenue(present, tax_rate)),
        ),
        detail_line("Armies", planet.army_count_raw().to_string()),
        detail_line(
            "Ground Batteries",
            planet.ground_batteries_raw().to_string(),
        ),
        detail_line(
            "Space Forces",
            format_owned_orbit_summary(app, coords, viewer_empire_id),
        ),
        detail_line(
            "Status",
            owned_status_detail_label(app, planet_index_0_based, planet),
        ),
        detail_line("Building", format_build_queue_summary(planet)),
        detail_line("Docked", format_stardock_summary(planet)),
    ];
    let widget_fields = vec![
        widget_field("Planet", planet.status_or_name_summary()),
        widget_field("Owner", String::from("You")),
        widget_field(
            "State",
            owned_status_widget_label(app, planet_index_0_based, planet),
        ),
        widget_field("Production", present.to_string()),
        widget_field("Potential Production", potential.to_string()),
        widget_field("Treasury", planet.stored_production_points().to_string()),
        widget_field("Budget", budget.to_string()),
        widget_field("Armies", planet.army_count_raw().to_string()),
        widget_field(
            "Ground Batteries",
            planet.ground_batteries_raw().to_string(),
        ),
        widget_field(
            "Starbases",
            active_starbase_count_at(&app.game_data, coords).to_string(),
        ),
        widget_field(
            "Orbit",
            format_owned_orbit_summary(app, coords, viewer_empire_id),
        ),
        widget_field("Building", format_build_queue_summary(planet)),
        widget_field("Docked", format_stardock_summary(planet)),
    ];

    SelectedPlanetDetail {
        planet_record_index_1_based: planet_index_0_based + 1,
        widget_fields,
        popup_lines,
    }
}

fn intel_planet_detail(
    app: &DashApp,
    world: &PlayerStarmapWorld,
    snapshot: Option<&PlanetIntelSnapshot>,
) -> SelectedPlanetDetail {
    let popup_lines = vec![
        detail_line("Coordinates", coords_label(world.coords)),
        detail_line(
            "Planet",
            world
                .known_name
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        detail_line("Owner", intel_owner_detail_label(&app.game_data, world)),
        detail_line("State", String::from("?")),
        detail_line(
            "Last Viewed/Scouted",
            snapshot
                .and_then(|row| row.last_intel_year)
                .map(|year| format!("Y{year}"))
                .unwrap_or_else(|| String::from("?")),
        ),
        detail_line("Production", known_u8(world.known_current_production)),
        detail_line(
            "Potential Production",
            known_u16(world.known_potential_production),
        ),
        detail_line(
            "Efficiency",
            efficiency_label(
                world.known_current_production,
                world.known_potential_production,
            ),
        ),
        detail_line("Treasury", known_u16(world.known_stored_points)),
        detail_line("Armies", known_u8(world.known_armies)),
        detail_line("Ground Batteries", known_u8(world.known_ground_batteries)),
        detail_line(
            "Space Forces",
            world
                .known_orbit_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        detail_line("Intel Tier", intel_tier_label(snapshot, world).to_string()),
        detail_line(
            "Docked",
            world
                .known_docked_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
    ];
    let intel_year = snapshot
        .and_then(|row| row.last_intel_year)
        .map(|year| format!("Y{year}"))
        .unwrap_or_else(|| String::from("Y?"));
    let widget_fields = vec![
        widget_field(
            "Planet",
            world
                .known_name
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        widget_field("Owner", intel_owner_widget_label(&app.game_data, world)),
        widget_field("State", String::from("?")),
        widget_field(
            "Intel",
            format!("{intel_year} {}", intel_tier_code(snapshot, world)),
        ),
        widget_field("Production", known_u8(world.known_current_production)),
        widget_field(
            "Potential Production",
            known_u16(world.known_potential_production),
        ),
        widget_field("Treasury", known_u16(world.known_stored_points)),
        widget_field("Armies", known_u8(world.known_armies)),
        widget_field("Ground Batteries", known_u8(world.known_ground_batteries)),
        widget_field("Starbases", known_u8(world.known_starbase_count)),
        widget_field(
            "Orbit",
            world
                .known_orbit_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
        widget_field(
            "Docked",
            world
                .known_docked_summary
                .clone()
                .unwrap_or_else(|| String::from("?")),
        ),
    ];

    SelectedPlanetDetail {
        planet_record_index_1_based: world.planet_record_index_1_based,
        widget_fields,
        popup_lines,
    }
}

fn detail_line(label: &'static str, value: String) -> DetailLine {
    DetailLine { label, value }
}

fn widget_field(label: &'static str, value: String) -> DetailLine {
    DetailLine { label, value }
}

pub(crate) fn widget_label_for_width(field: &DetailLine, body_width: usize) -> &'static str {
    let Some(variants) = widget_label_variants(field.label) else {
        return field.label;
    };

    variants
        .iter()
        .copied()
        .find(|label| label.chars().count() + 2 + field.value.chars().count() <= body_width)
        .unwrap_or_else(|| variants.last().copied().unwrap_or(field.label))
}

fn preferred_widget_field_width(field: &DetailLine) -> usize {
    widget_label_variants(field.label)
        .and_then(|variants| variants.first().copied())
        .unwrap_or(field.label)
        .chars()
        .count()
        + 2
        + field.value.chars().count()
}

fn widget_label_variants(label: &'static str) -> Option<&'static [&'static str]> {
    Some(match label {
        "Potential Production" => &["Pot Prod"],
        "Production" => &["Production", "Prod"],
        "Treasury" => &["Treasury", "Trsry"],
        "Budget" => &["Budget", "Bdgt"],
        "Ground Batteries" => &["Grnd Batt", "GBs"],
        "Armies" => &["Armies", "ARs"],
        "Starbases" => &["Starbases", "SBs"],
        _ => return None,
    })
}

fn coords_label(coords: [u8; 2]) -> String {
    format!("({:02},{:02})", coords[0], coords[1])
}

fn selected_planet_record_index(app: &DashApp) -> usize {
    let viewer_empire_id = app.player_record_index_1_based as u8;
    let snapshot_map = app
        .planet_intel_snapshots
        .iter()
        .cloned()
        .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
        .collect::<BTreeMap<_, _>>();
    let projection = build_player_starmap_projection_from_snapshots(
        &app.game_data,
        &snapshot_map,
        viewer_empire_id,
    );
    projection
        .worlds
        .iter()
        .find(|world| world.coords == [app.crosshair_x, app.crosshair_y])
        .map(|world| world.planet_record_index_1_based)
        .unwrap_or(0)
}

fn known_u8(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}

fn known_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| String::from("?"))
}

fn efficiency_label(current: Option<u8>, potential: Option<u16>) -> String {
    match (current, potential) {
        (Some(current), Some(potential)) if potential != 0 => {
            format!(
                "{:.1}%",
                (f64::from(current) / f64::from(potential)) * 100.0
            )
        }
        _ => String::from("?"),
    }
}

fn owned_efficiency_label(current: u16, potential: u16) -> String {
    if potential == 0 {
        String::from("?")
    } else {
        format!(
            "{:.1}%",
            (f64::from(current) / f64::from(potential)) * 100.0
        )
    }
}

fn owned_owner_detail_label(game_data: &nc_data::CoreGameData, owner_empire_raw: u8) -> String {
    if owner_empire_raw == 0 {
        return String::from("Unowned");
    }

    let Some(player) = game_data
        .player
        .records
        .get(owner_empire_raw.saturating_sub(1) as usize)
    else {
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

fn owner_state_detail_label(game_data: &nc_data::CoreGameData, owner_empire_raw: u8) -> String {
    match game_data.empire_campaign_state(owner_empire_raw) {
        Some(CampaignState::Stable) => String::from("Stable"),
        Some(CampaignState::MarginalExistence) => String::from("Marginal Existence"),
        Some(CampaignState::DefectionRisk) => String::from("Defection Risk"),
        Some(CampaignState::Defeated) => String::from("Defeated"),
        Some(CampaignState::CivilDisorder) => String::from("In Civil Disorder"),
        Some(CampaignState::Rogue) => String::from("Rogue"),
        None if owner_empire_raw == 0 => String::from("Unowned"),
        None => String::from("Unknown"),
    }
}

fn intel_owner_detail_label(
    game_data: &nc_data::CoreGameData,
    world: &PlayerStarmapWorld,
) -> String {
    match world.known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => {
            if game_data.empire_campaign_state(owner) == Some(CampaignState::CivilDisorder) {
                String::from("In Civil Disorder")
            } else {
                world
                    .known_owner_empire_name
                    .as_deref()
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Empire #{owner}"))
            }
        }
        None => String::from("?"),
    }
}

fn intel_owner_widget_label(
    game_data: &nc_data::CoreGameData,
    world: &PlayerStarmapWorld,
) -> String {
    match world.known_owner_empire_id {
        Some(0) => String::from("Unowned"),
        Some(owner) => {
            if game_data.empire_campaign_state(owner) == Some(CampaignState::CivilDisorder) {
                String::from("ICD")
            } else {
                format!("#{owner}")
            }
        }
        None => String::from("?"),
    }
}

fn intel_tier_label<'a>(
    snapshot: Option<&'a PlanetIntelSnapshot>,
    world: &'a PlayerStarmapWorld,
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

fn intel_tier_code(
    snapshot: Option<&PlanetIntelSnapshot>,
    world: &PlayerStarmapWorld,
) -> &'static str {
    match intel_tier_label(snapshot, world) {
        "owned" => "own",
        "full" => "full",
        "partial" => "part",
        _ => "unk",
    }
}

fn format_stardock_summary(planet: &PlanetRecord) -> String {
    shared_stardock_summary(planet, CompactUnitSummaryStyle::JoinedCodes)
}

fn format_build_queue_summary(planet: &PlanetRecord) -> String {
    shared_build_queue_summary(planet, CompactUnitSummaryStyle::JoinedCodes)
}

fn format_owned_orbit_summary(app: &DashApp, coords: [u8; 2], viewer_empire_id: u8) -> String {
    shared_owned_orbit_summary(owned_orbit_presence(
        &app.game_data,
        viewer_empire_id,
        coords,
    ))
}

fn owned_status_widget_label(
    app: &DashApp,
    planet_index_0_based: usize,
    _planet: &PlanetRecord,
) -> String {
    match owned_planet_status(
        &app.game_data,
        app.player_record_index_1_based as u8,
        planet_index_0_based,
        &app.planet_scorch_orders,
    ) {
        OwnedPlanetStatus::Scorched => String::from("Scorched"),
        OwnedPlanetStatus::Homeworld => String::from("Homeworld"),
        OwnedPlanetStatus::StarbasePresent => String::from("Starbase"),
        OwnedPlanetStatus::FactoriesDestroyed => String::from("Destroyed"),
        OwnedPlanetStatus::FactoriesDamaged => String::from("Damaged"),
        OwnedPlanetStatus::FactoriesFunctional => String::from("Normal"),
    }
}

fn owned_status_detail_label(
    app: &DashApp,
    planet_index_0_based: usize,
    _planet: &PlanetRecord,
) -> String {
    match owned_planet_status(
        &app.game_data,
        app.player_record_index_1_based as u8,
        planet_index_0_based,
        &app.planet_scorch_orders,
    ) {
        OwnedPlanetStatus::Scorched => String::from("Planet is scorched!"),
        OwnedPlanetStatus::Homeworld => String::from("Homeworld - fully developed"),
        OwnedPlanetStatus::StarbasePresent => String::from("Regular planet - starbase present"),
        OwnedPlanetStatus::FactoriesDestroyed => {
            String::from("Regular planet - industry destroyed")
        }
        OwnedPlanetStatus::FactoriesDamaged => String::from("Regular planet - industry damaged"),
        OwnedPlanetStatus::FactoriesFunctional => String::from("Regular planet - industry intact"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_data::{GameStateBuilder, PlanetRecord};

    #[test]
    fn widget_rows_use_split_owned_world_fields() {
        let mut app = crate::app::state::DashApp::new_for_tests(
            std::path::PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            std::collections::BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            nc_ui::ScreenGeometry::new(160, 40),
            nc_ui::ScreenGeometry::new(108, 26),
            1,
        );
        app.crosshair_x = app.game_data.planets.records[0].coords_raw()[0];
        app.crosshair_y = app.game_data.planets.records[0].coords_raw()[1];

        let detail = selected_planet_detail(&app).expect("selected world");
        assert_eq!(
            detail
                .widget_fields
                .iter()
                .map(|field| field.label)
                .collect::<Vec<_>>(),
            vec![
                "Planet",
                "Owner",
                "State",
                "Production",
                "Potential Production",
                "Treasury",
                "Budget",
                "Armies",
                "Ground Batteries",
                "Starbases",
                "Orbit",
                "Building",
                "Docked",
            ]
        );
    }

    #[test]
    fn widget_field_format_prefers_fuller_labels_when_they_fit() {
        let current = DetailLine {
            label: "Production",
            value: String::from("9"),
        };
        let potential = DetailLine {
            label: "Potential Production",
            value: String::from("10"),
        };
        let stored = DetailLine {
            label: "Treasury",
            value: String::from("125"),
        };
        let batteries = DetailLine {
            label: "Ground Batteries",
            value: String::from("4"),
        };

        assert_eq!(widget_label_for_width(&current, 19), "Production");
        assert_eq!(widget_label_for_width(&potential, 19), "Pot Prod");
        assert_eq!(widget_label_for_width(&stored, 19), "Treasury");
        assert_eq!(widget_label_for_width(&batteries, 19), "Grnd Batt");
        assert_eq!(widget_label_for_width(&current, 14), "Production");
        assert_eq!(widget_label_for_width(&stored, 14), "Treasury");
        assert_eq!(widget_label_for_width(&batteries, 14), "Grnd Batt");
        assert_eq!(widget_label_for_width(&batteries, 6), "GBs");
    }

    #[test]
    fn compact_unit_summary_uses_tui_style_without_dashes() {
        let mut planet = PlanetRecord::new_zeroed();
        planet.set_build_kind_raw(0, 3);
        planet.set_build_count_raw(0, 90);
        planet.set_stardock_kind_raw(0, 2);
        planet.set_stardock_count_raw(0, 5);

        assert_eq!(format_build_queue_summary(&planet), "2BB");
        assert_eq!(format_stardock_summary(&planet), "5CA");
    }
}
