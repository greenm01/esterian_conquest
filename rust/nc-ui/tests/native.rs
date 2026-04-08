use nc_ui::native::{cell_position_at_pixel, terminal_grid_for_pixels};
use winit::dpi::PhysicalPosition;

#[test]
fn terminal_grid_for_pixels_uses_shared_cell_metrics() {
    assert_eq!(terminal_grid_for_pixels(100, 54), (10, 3));
    assert_eq!(terminal_grid_for_pixels(9, 17), (1, 1));
}

#[test]
fn cell_position_at_pixel_accounts_for_centered_grid_offset() {
    let position = PhysicalPosition::new(25.0, 27.0);
    assert_eq!(cell_position_at_pixel(4, 3, 60, 54, position), Some((1, 1)));
}

#[test]
fn cell_position_at_pixel_rejects_window_margins() {
    let position = PhysicalPosition::new(2.0, 10.0);
    assert_eq!(cell_position_at_pixel(4, 3, 60, 54, position), None);
}
