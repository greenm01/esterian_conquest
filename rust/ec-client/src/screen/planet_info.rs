use crossterm::event::KeyEvent;
use ec_data::{IntelTier, PlanetIntelSnapshot, build_player_starmap_projection};

use crate::app::Action;
use crate::screen::layout::{
    draw_command_line_default_input, draw_command_prompt, draw_status_line, draw_title_bar,
    new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, ScreenFrame, command_menu_label};

pub struct PlanetInfoScreen;

impl PlanetInfoScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_prompt(
        &mut self,
        default_coords: [u8; 2],
        input: &str,
        error: Option<&str>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "INFO ABOUT A PLANET:");
        buffer.write_text(
            2,
            0,
            "Enter coordinates of the planet to view.",
            crate::theme::classic::body_style(),
        );
        if let Some(error) = error {
            draw_status_line(&mut buffer, 4, "Error: ", error);
        }
        draw_command_line_default_input(
            &mut buffer,
            command_menu_label(menu),
            "Planet coords ",
            &format!("{},{}", default_coords[0], default_coords[1]),
            input,
        );
        Ok(buffer)
    }

    pub fn render_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        planet_idx: usize,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        if menu != CommandMenu::Planet {
            return self.render_intel_detail(frame, planet_idx, menu);
        }

        let planet = frame
            .game_data
            .planets
            .records
            .get(planet_idx)
            .ok_or("planet detail missing")?;
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
        let stardock_units: u32 = (0..10)
            .map(|slot| u32::from(planet.stardock_count_raw(slot)))
            .sum();
        let has_starbase = owner_empire_raw != 0
            && frame
                .game_data
                .planet_has_friendly_starbase(owner_empire_raw, [x, y]);

        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "INFO ABOUT A PLANET:");
        draw_status_line(&mut buffer, 2, "Coordinates: ", &format!("X={x}, Y={y}"));
        draw_status_line(&mut buffer, 3, "Planet: ", &planet.status_or_name_summary());
        draw_status_line(&mut buffer, 4, "Owner: ", &owner_label);
        draw_status_line(&mut buffer, 5, "State: ", &state_label);
        draw_status_line(&mut buffer, 7, "Present Production: ", &present.to_string());
        draw_status_line(
            &mut buffer,
            8,
            "Potential Production: ",
            &potential.to_string(),
        );
        draw_status_line(&mut buffer, 9, "Efficiency: ", &format!("{efficiency:.1}%"));
        draw_status_line(
            &mut buffer,
            10,
            "Stored Production Points: ",
            &planet.stored_production_points().to_string(),
        );
        draw_status_line(
            &mut buffer,
            12,
            "Armies: ",
            &planet.army_count_raw().to_string(),
        );
        draw_status_line(
            &mut buffer,
            13,
            "Ground Batteries: ",
            &planet.ground_batteries_raw().to_string(),
        );
        draw_status_line(
            &mut buffer,
            14,
            "Stardock Units: ",
            &stardock_units.to_string(),
        );
        draw_status_line(
            &mut buffer,
            15,
            "Starbase in Orbit: ",
            if has_starbase { "YES" } else { "NO" },
        );
        draw_command_prompt(&mut buffer, 17, command_menu_label(menu), "SLAP A KEY");
        Ok(buffer)
    }

    fn render_intel_detail(
        &mut self,
        frame: &ScreenFrame<'_>,
        planet_idx: usize,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let projection = build_player_starmap_projection(
            frame.game_data,
            frame.database,
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

        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "INFO ABOUT A PLANET:");
        draw_status_line(
            &mut buffer,
            2,
            "Coordinates: ",
            &format!("X={}, Y={}", world.coords[0], world.coords[1]),
        );
        draw_status_line(
            &mut buffer,
            3,
            "Planet: ",
            world.known_name.as_deref().unwrap_or("?"),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Owner: ",
            &owner_label,
        );
        draw_status_line(&mut buffer, 5, "State: ", "?");
        let intel_snapshot = frame.planet_intel_snapshots.get(&(planet_idx + 1));
        draw_status_line(
            &mut buffer,
            6,
            "Last Intel: ",
            &intel_snapshot
                .and_then(|snapshot| snapshot.last_intel_year)
                .map(|year| format!("Y{year}"))
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_status_line(
            &mut buffer,
            7,
            "Present Production: ",
            &world
                .known_potential_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_status_line(
            &mut buffer,
            8,
            "Potential Production: ",
            &world
                .known_potential_production
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_status_line(&mut buffer, 9, "Efficiency: ", "?");
        draw_status_line(&mut buffer, 10, "Stored Production Points: ", "?");
        draw_status_line(
            &mut buffer,
            12,
            "Armies: ",
            &world
                .known_armies
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_status_line(
            &mut buffer,
            13,
            "Ground Batteries: ",
            &world
                .known_ground_batteries
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        );
        draw_status_line(&mut buffer, 14, "Stardock Units: ", "?");
        draw_status_line(&mut buffer, 15, "Starbase in Orbit: ", "?");
        draw_status_line(
            &mut buffer,
            16,
            "Intel Tier: ",
            intel_tier_label(intel_snapshot, &world),
        );
        draw_command_prompt(&mut buffer, 17, command_menu_label(menu), "SLAP A KEY");
        Ok(buffer)
    }

    pub fn handle_prompt_key(&self, _key: KeyEvent) -> Action {
        Action::Noop
    }

    pub fn handle_detail_key(&self, _key: KeyEvent) -> Action {
        Action::ReturnToCommandMenu
    }
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
            || world.known_potential_production.is_some() => "partial",
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
