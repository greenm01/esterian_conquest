use crossterm::event::KeyEvent;

use crate::app::Action;
use crate::screen::layout::{
    draw_centered_text, draw_command_prompt, draw_title_bar, new_playfield,
};
use crate::screen::{CommandMenu, PlayfieldBuffer, Screen, ScreenFrame, command_menu_label};
use crate::theme::classic;

pub struct EmpireProfileScreen;

impl EmpireProfileScreen {
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
        let economy = frame.game_data.empire_economy_summary(player_idx);
        let active = frame.game_data.empire_active_duty_summary(player_idx);
        let stardock = frame.game_data.empire_stardock_summary(player_idx);

        let mut buffer = new_playfield();
        draw_title_bar(
            &mut buffer,
            0,
            &format!(
                "Empire Name: {}",
                display_or_unknown(&frame.player.empire_name)
            ),
        );

        write_stat_pair(
            &mut buffer,
            2,
            "Number of Planets",
            &economy.owned_planets.to_string(),
            "Rank by Number of Planets",
            &ordinal_rank(economy.rank_by_planets),
        );
        write_stat_pair(
            &mut buffer,
            3,
            "Present Production",
            &economy.present_production.to_string(),
            "Rank by Present Production",
            &ordinal_rank(economy.rank_by_present_production),
        );
        write_stat_pair(
            &mut buffer,
            4,
            "Potential Production",
            &economy.potential_production.to_string(),
            "Tax Rate of Empire",
            &format!("{:.1}%", economy.tax_rate as f64),
        );
        write_stat_pair(
            &mut buffer,
            5,
            "Total Available Points",
            &economy.total_available_points.to_string(),
            "Maximum # of Fleets & Bases",
            &economy.max_fleets_and_bases.to_string(),
        );
        write_stat_pair(
            &mut buffer,
            6,
            "Efficiency of Empire",
            &format!("{:.3}%", economy.efficiency_percent),
            "Current # of Fleets & Bases",
            &economy.current_fleets_and_bases.to_string(),
        );

        buffer.write_text(
            8,
            0,
            "SHIPS & OTHER UNITS ON ACTIVE DUTY",
            classic::menu_hotkey_style(),
        );
        buffer.write_text(
            8,
            38,
            "SHIPS & OTHER UNITS IN STARDOCK",
            classic::menu_hotkey_style(),
        );

        write_unit_line(
            &mut buffer,
            9,
            "Destroyers",
            active.destroyers,
            stardock.destroyers,
        );
        write_unit_line(
            &mut buffer,
            10,
            "Cruisers",
            active.cruisers,
            stardock.cruisers,
        );
        write_unit_line(
            &mut buffer,
            11,
            "Battleships",
            active.battleships,
            stardock.battleships,
        );
        write_unit_line(&mut buffer, 12, "Scouts", active.scouts, stardock.scouts);
        write_unit_line(
            &mut buffer,
            13,
            "Transports",
            active.transports,
            stardock.transports,
        );
        write_unit_line(&mut buffer, 14, "ETACs", active.etacs, stardock.etacs);
        write_unit_line(
            &mut buffer,
            15,
            "StarBases",
            active.starbases,
            stardock.starbases,
        );
        buffer.write_text(
            16,
            0,
            &format!("Armies           : {}", active.armies),
            classic::status_value_style(),
        );
        buffer.write_text(
            17,
            0,
            &format!("Ground Batteries : {}", active.ground_batteries),
            classic::status_value_style(),
        );

        draw_centered_text(
            &mut buffer,
            18,
            &format!(
                "Autopilot is {}.",
                if player.autopilot_flag() != 0 {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            classic::status_value_style(),
        );
        draw_command_prompt(&mut buffer, 19, command_menu_label(menu), "SLAP A KEY");
        Ok(buffer)
    }
}

impl Screen for EmpireProfileScreen {
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

fn write_stat_pair(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    left_label: &str,
    left_value: &str,
    right_label: &str,
    right_value: &str,
) {
    buffer.write_text(row, 0, left_label, classic::status_value_style());
    buffer.write_text(row, 22, ": ", classic::status_value_style());
    buffer.write_text(row, 24, left_value, classic::status_value_style());
    buffer.write_text(row, 40, right_label, classic::status_value_style());
    buffer.write_text(row, 68, ": ", classic::status_value_style());
    buffer.write_text(row, 70, right_value, classic::status_value_style());
}

fn write_unit_line(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    active_value: u32,
    stardock_value: u32,
) {
    let line = format!(
        "{label:<17}: {active_value:<20}{label:<17}: {stardock_value}",
        label = label,
        active_value = active_value,
        stardock_value = stardock_value,
    );
    buffer.write_text(row, 0, &line, classic::status_value_style());
}

fn ordinal_rank(rank: usize) -> String {
    let suffix = match rank % 100 {
        11..=13 => "th",
        _ => match rank % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{rank}{suffix}")
}

fn display_or_unknown(value: &str) -> &str {
    if value.is_empty() { "<unnamed>" } else { value }
}
