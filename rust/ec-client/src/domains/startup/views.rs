use crate::app::state::App;
use crate::screen::{
    PlayfieldBuffer, Screen, ScreenFrame, ScreenId, render_first_time_homeworld_confirm,
    render_first_time_homeworld_name, render_first_time_join_name,
    render_first_time_join_name_confirm, render_first_time_join_no_pending,
    render_first_time_join_summary,
};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        planet_intel_snapshots: &app.planet_intel_snapshots,
    };
    match app.current_screen {
        ScreenId::Startup(phase) => app.startup.render_phase(
            &frame,
            phase,
            app.startup_state.splash_page,
            app.startup_state.intro_page,
            app.startup_state.results_block,
            app.startup_state.results_page,
            app.startup_state.results_mode,
            app.startup_state.results_nonstop,
            app.startup_state.messages_block,
            app.startup_state.messages_page,
            app.startup_state.messages_mode,
            app.startup_state.messages_nonstop,
            app.startup_state.results_deleted_any,
            app.startup_state.messages_deleted_any,
            app.game_data.conquest.game_year(),
        ),
        ScreenId::FirstTimeMenu => app
            .first_time_menu
            .render(app.startup_state.first_time_status.as_deref()),
        ScreenId::FirstTimeHelp => app.first_time_help.render(&frame),
        ScreenId::FirstTimeEmpires => app
            .first_time_empires
            .render_rows(&app.first_time_empire_rows()),
        ScreenId::FirstTimeIntro => app
            .first_time_intro
            .render_page(app.startup_state.first_time_intro_page),
        ScreenId::FirstTimePreloadedRenamePrompt => {
            crate::screen::render_preloaded_first_login_rename_prompt(&app.player.empire_name)
        }
        ScreenId::FirstTimeJoinEmpireName => render_first_time_join_name(
            app.startup_state.first_time_rename_preloaded_empire,
            &app.player.empire_name,
            &app.startup_state.first_time_input,
            app.startup_state.first_time_status.as_deref(),
        ),
        ScreenId::FirstTimeJoinEmpireConfirm => render_first_time_join_name_confirm(
            app.startup_state.first_time_rename_preloaded_empire,
            &app.startup_state.first_time_empire_name,
        ),
        ScreenId::FirstTimeJoinSummary => render_first_time_join_summary(
            &app.startup_state.first_time_empire_name,
            app.player.record_index_1_based,
            app.game_data.conquest.game_year(),
        ),
        ScreenId::FirstTimeJoinNoPending => render_first_time_join_no_pending(),
        ScreenId::FirstTimeHomeworldName => {
            let (coords, present, potential) = app.first_time_homeworld_summary()?;
            render_first_time_homeworld_name(
                coords,
                present,
                potential,
                app.player.classic_login_state
                    == crate::model::ClassicLoginState::MatchedPreloadedFirstLogin,
                &app.startup_state.first_time_input,
                app.startup_state.first_time_status.as_deref(),
            )
        }
        ScreenId::FirstTimeHomeworldConfirm => {
            let (coords, present, potential) = app.first_time_homeworld_summary()?;
            render_first_time_homeworld_confirm(
                coords,
                present,
                potential,
                app.player.classic_login_state
                    == crate::model::ClassicLoginState::MatchedPreloadedFirstLogin,
                &app.startup_state.first_time_homeworld_name,
            )
        }
        ScreenId::ColonyWorldName => {
            let (coords, present, potential) = app.colony_world_summary()?;
            crate::screen::render_colony_world_name(
                coords,
                present,
                potential,
                &app.startup_state.first_time_input,
                app.startup_state.first_time_status.as_deref(),
            )
        }
        ScreenId::ColonyWorldConfirm => {
            let (coords, _, _) = app.colony_world_summary()?;
            crate::screen::render_colony_world_confirm(coords, &app.startup_state.colony_world_name)
        }
        ScreenId::MainMenu => app.main_menu.render_with_notice(
            app.command_menu_notice.as_deref(),
            app.expert_mode,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::Main,
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
        ),
        ScreenId::MainHelp => app.main_help.render(&frame),
        ScreenId::GeneralMenu => app.general_menu.render_with_notice(
            &frame,
            app.command_menu_notice.as_deref(),
            app.expert_mode,
            app.messaging.delete_reviewables_prompt_active,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::General,
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
        ),
        ScreenId::GeneralHelp => app.general_help.render(&frame),
        ScreenId::Reports => app
            .reports
            .render_with_menu(&frame, app.command_return_menu),
        _ => unreachable!("startup views called for non-startup screen"),
    }
}
