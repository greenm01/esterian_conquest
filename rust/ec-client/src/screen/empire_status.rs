use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{draw_command_prompt, draw_status_line, draw_title_bar, new_playfield};
use crate::screen::{command_menu_label, CommandMenu, PlayfieldBuffer, Screen, ScreenFrame};
pub struct EmpireStatusScreen;

impl EmpireStatusScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render_with_menu(
        &mut self,
        frame: &ScreenFrame<'_>,
        menu: CommandMenu,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        let player_idx = frame.player.record_index_1_based;
        let player = &frame.game_data.player.records[player_idx - 1];
        let empire_raw = player_idx as u8;

        let mut buffer = new_playfield();
        draw_title_bar(&mut buffer, 0, "STATUS, YOUR EMPIRE: ");
        draw_status_line(
            &mut buffer,
            2,
            "Empire: ",
            display_or_unknown(&frame.player.empire_name),
        );
        draw_status_line(
            &mut buffer,
            3,
            "Handle: ",
            display_or_unknown(&frame.player.handle),
        );
        draw_status_line(
            &mut buffer,
            4,
            "Year: ",
            &frame.game_data.conquest.game_year().to_string(),
        );
        draw_status_line(
            &mut buffer,
            5,
            "Campaign state: ",
            campaign_state_label(frame.game_data.empire_campaign_state(empire_raw)),
        );
        draw_status_line(
            &mut buffer,
            7,
            "Tax rate: ",
            &format!("{}%", player.tax_rate()),
        );
        draw_status_line(
            &mut buffer,
            8,
            "Owned planets: ",
            &frame
                .game_data
                .player_owned_planet_count_current_known(player_idx)
                .to_string(),
        );
        draw_status_line(
            &mut buffer,
            9,
            "Owned fleets: ",
            &frame
                .game_data
                .player_owned_fleet_count_current_known(player_idx)
                .to_string(),
        );
        draw_status_line(
            &mut buffer,
            10,
            "Starbases: ",
            &frame
                .game_data
                .player_owned_base_record_count_current_known(player_idx)
                .to_string(),
        );
        draw_status_line(
            &mut buffer,
            11,
            "Autopilot: ",
            if player.autopilot_flag() != 0 {
                "ON"
            } else {
                "OFF"
            },
        );
        draw_command_prompt(&mut buffer, 13, command_menu_label(menu), "SLAP A KEY");
        Ok(buffer)
    }
}

impl Screen for EmpireStatusScreen {
    fn render(
        &mut self,
        frame: &ScreenFrame<'_>,
    ) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
        self.render_with_menu(frame, CommandMenu::General)
    }

    fn handle_key(&self, _key: KeyEvent) -> Action {
        Action::ReturnToCommandMenu
    }
}

fn campaign_state_label(state: Option<ec_data::CampaignState>) -> &'static str {
    match state {
        Some(ec_data::CampaignState::Stable) => "Stable",
        Some(ec_data::CampaignState::MarginalExistence) => "Marginal Existence",
        Some(ec_data::CampaignState::DefectionRisk) => "Defection Risk",
        Some(ec_data::CampaignState::Defeated) => "Defeated",
        Some(ec_data::CampaignState::CivilDisorder) => "In Civil Disorder",
        Some(ec_data::CampaignState::Rogue) => "Rogue",
        None => "Unknown",
    }
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() {
        "<unknown>"
    } else {
        value
    }
}
