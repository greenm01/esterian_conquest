use crate::app::state::App;
use crate::screen::{PlayfieldBuffer, Screen, ScreenFrame, ScreenId};

pub fn render(app: &mut App) -> Result<PlayfieldBuffer, Box<dyn std::error::Error>> {
    let frame = ScreenFrame {
        game_dir: &app.game_dir,
        game_data: &app.game_data,
        player: &app.player,
        campaign_seed: app.campaign_seed,
        planet_intel_snapshots: &app.planet_intel_snapshots,
    };
    match app.current_screen {
        ScreenId::StarbaseMenu => app.starbase_menu.render_with_notice(
            app.command_menu_notice.as_deref(),
            app.expert_mode,
            app.planet.info_prompt_active
                && app.command_return_menu == crate::screen::CommandMenu::Starbase,
            app.default_planet_prompt_coords(),
            &app.planet.info_input,
            app.planet.info_error.as_deref(),
            app.starbase_move_prompt_mode(),
            app.starbase_move_prompt_label(),
            &app.starbase.move_prompt_default_value,
            &app.starbase.move_prompt_input,
            app.starbase.move_prompt_status.as_deref(),
        ),
        ScreenId::StarbaseHelp => app.starbase_help.render(&frame),
        ScreenId::StarbaseList => app.starbase_list.render(
            &app.starbase_rows(),
            app.starbase.scroll_offset,
            app.starbase.cursor,
        ),
        ScreenId::StarbaseReviewSelect => app.starbase_review.render_select(
            &app.starbase_rows(),
            app.starbase.scroll_offset,
            app.starbase.cursor,
            &app.starbase.review_input,
            app.starbase.review_status.as_deref(),
        ),
        ScreenId::StarbaseReview => {
            let rows = app.starbase_rows();
            let row = rows
                .get(app.starbase.review_index)
                .ok_or("starbase review row missing")?;
            app.starbase_review.render_detail(row)
        }
        _ => unreachable!("starbase views called for non-starbase screen"),
    }
}
