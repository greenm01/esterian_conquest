use std::sync::{Mutex, MutexGuard, OnceLock};

use nc_game::domains::fleet::screens::fleet::FleetMenuScreen;
use nc_game::domains::planet::screens::planet_menu::PlanetMenuScreen;
use nc_game::screen::GameColor;
use nc_game::theme;
use nc_game::theme::classic;

static MENU_THEME_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn menu_theme_test_guard() -> MutexGuard<'static, ()> {
    MENU_THEME_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
}

#[test]
fn featured_menu_label_cells_are_used_for_planet_and_fleet_lists() {
    let _guard = menu_theme_test_guard();
    theme::apply_default_theme();

    let featured_style = classic::menu_featured_label_style();
    let normal_style = classic::menu_hotkey_style();
    assert_ne!(featured_style, normal_style);
    assert_eq!(featured_style.fg, GameColor::Rgb(125, 207, 255));

    let mut planet_menu = PlanetMenuScreen::new();
    let planet_buffer = planet_menu
        .render_with_notice(
            None,
            false,
            false,
            [0, 0],
            "",
            None,
            false,
            "",
            "",
            None,
            None,
            false,
            None,
            &[],
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("planet menu should render");
    let planet_line = planet_buffer.plain_line(3);
    let planet_col = planet_line.find("P>lanet List").expect("planet list entry");
    assert_eq!(planet_buffer.row(3)[planet_col].style, normal_style);
    assert_eq!(planet_buffer.row(3)[planet_col + 2].style, featured_style);
    let auto_col = planet_line
        .find("A>UTO-COMMISSION")
        .expect("auto-commission entry");
    assert_eq!(planet_buffer.row(3)[auto_col].style, normal_style);
    assert_eq!(
        planet_buffer.row(3)[auto_col + 2].style,
        classic::menu_style()
    );

    let mut fleet_menu = FleetMenuScreen::new();
    let fleet_buffer = fleet_menu
        .render_with_notice(
            None,
            false,
            false,
            None,
            None,
            "",
            "",
            None,
            None,
            None,
            [0, 0],
            "",
            None,
        )
        .expect("fleet menu should render");
    let fleet_line = fleet_buffer.plain_line(4);
    let fleet_col = fleet_line.find("F>leet List").expect("fleet list entry");
    assert_eq!(fleet_buffer.row(4)[fleet_col].style, normal_style);
    assert_eq!(fleet_buffer.row(4)[fleet_col + 2].style, featured_style);
    let detach_col = fleet_line.find("D>etach Ships").expect("detach entry");
    assert_eq!(fleet_buffer.row(4)[detach_col].style, normal_style);
    assert_eq!(
        fleet_buffer.row(4)[detach_col + 2].style,
        classic::menu_style()
    );
}
