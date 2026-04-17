use crate::PlayfieldBuffer;
use crate::buffer::CellStyle;
use crate::geometry::ScreenGeometry;
use crate::input::{KeyCode, KeyEvent, MouseEvent};
use crate::native::{self, NativeApp};
use crate::startup::NativeLaunchOptions;
use crate::theme;
use crate::ui::UiScene;
use nc_nostr::state_sync::{
    GameState, HostedDiplomacyState, HostedFleetShips, HostedOwnedFleet, HostedOwnedPlanet,
    HostedPlayerRosterEntry, HostedPlayerState, HostedQueuedMail, HostedReportBlock,
    HostedStardockSlot, HostedStarmapState, HostedStatePayload, HostedWorldState,
};
use std::fmt::Write as _;
use std::time::Instant;

pub(crate) fn run_glyphon_cursor_motion_native_repro(
    native_options: NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    native::run(CursorMotionReproApp::new(), native_options)
}

pub(crate) fn run_glyphon_glyph_grid_native_repro(
    native_options: NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    native::run(
        StaticPlayfieldReproApp::new(
            "nc-dash glyphon glyph grid repro",
            ScreenGeometry::new(92, 26),
            build_static_glyph_grid_playfield,
        ),
        native_options,
    )
}

pub(crate) fn run_glyphon_starmap_native_repro(
    native_options: NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    native::run(
        StaticPlayfieldReproApp::new(
            "nc-dash glyphon starmap repro",
            ScreenGeometry::new(96, 30),
            build_static_starmap_playfield,
        ),
        native_options,
    )
}

pub(crate) fn sample_hosted_dashboard_snapshot() -> GameState {
    GameState {
        game_id: "wayland-repro".to_string(),
        turn: 4,
        year: 3004,
        player_seat: 1,
        player_name: "Terran Union".to_string(),
        state_hash: "wayland-repro-hash".to_string(),
        state: HostedStatePayload {
            player: HostedPlayerState {
                seat: 1,
                empire_name: "Terran Union".to_string(),
                handle: Some("StarRider".to_string()),
                mode: "active".to_string(),
                tax_rate: 33,
                planet_count: 1,
                starbase_count: 1,
                homeworld_planet_index: 1,
                last_run_year: 3004,
                diplomacy: vec![HostedDiplomacyState {
                    empire_id: 2,
                    relation: "enemy".to_string(),
                }],
            },
            roster: vec![
                HostedPlayerRosterEntry {
                    empire_id: 1,
                    empire_name: "Terran Union".to_string(),
                    is_self: true,
                },
                HostedPlayerRosterEntry {
                    empire_id: 2,
                    empire_name: "Rigel Empire".to_string(),
                    is_self: false,
                },
            ],
            starmap: HostedStarmapState {
                map_width: 18,
                map_height: 18,
                viewer_empire_id: 1,
                year: 3004,
                worlds: vec![
                    HostedWorldState {
                        planet_index: 1,
                        coords: [8, 8],
                        intel_tier: "owned".to_string(),
                        known_name: Some("Sol".to_string()),
                        known_owner_empire_id: Some(1),
                        known_owner_empire_name: Some("Terran Union".to_string()),
                        known_potential_production: Some(100),
                        known_armies: Some(20),
                        known_ground_batteries: Some(5),
                        known_starbase_count: Some(1),
                        known_current_production: Some(40),
                        known_stored_points: Some(12),
                        known_docked_summary: None,
                        known_orbit_summary: None,
                    },
                    HostedWorldState {
                        planet_index: 2,
                        coords: [10, 10],
                        intel_tier: "partial".to_string(),
                        known_name: Some("Rigel".to_string()),
                        known_owner_empire_id: Some(2),
                        known_owner_empire_name: Some("Rigel Empire".to_string()),
                        known_potential_production: Some(80),
                        known_armies: None,
                        known_ground_batteries: None,
                        known_starbase_count: None,
                        known_current_production: None,
                        known_stored_points: None,
                        known_docked_summary: None,
                        known_orbit_summary: Some("1 hostile fleet".to_string()),
                    },
                    HostedWorldState {
                        planet_index: 3,
                        coords: [13, 6],
                        intel_tier: "unknown".to_string(),
                        known_name: None,
                        known_owner_empire_id: None,
                        known_owner_empire_name: None,
                        known_potential_production: None,
                        known_armies: None,
                        known_ground_batteries: None,
                        known_starbase_count: None,
                        known_current_production: None,
                        known_stored_points: None,
                        known_docked_summary: None,
                        known_orbit_summary: None,
                    },
                ],
            },
            owned_planets: vec![HostedOwnedPlanet {
                planet_index: 1,
                name: "Sol".to_string(),
                coords: [8, 8],
                potential_production: 100,
                current_production: 40,
                stored_points: 12,
                armies: 20,
                ground_batteries: 5,
                starbase_count: 1,
                stardock: vec![HostedStardockSlot {
                    slot: 1,
                    kind: "destroyer".to_string(),
                    count: 2,
                }],
            }],
            owned_fleets: vec![
                HostedOwnedFleet {
                    fleet_id: 1,
                    local_slot: 1,
                    coords: [12, 9],
                    target_coords: [10, 10],
                    order: "move".to_string(),
                    order_summary: "Move fleet to Sector (10,10)".to_string(),
                    rules_of_engagement: 4,
                    current_speed: 5,
                    max_speed: 6,
                    ships: HostedFleetShips {
                        scout: 1,
                        battleship: 0,
                        cruiser: 2,
                        destroyer: 0,
                        transport: 0,
                        army: 0,
                        etac: 0,
                        total_starships: 3,
                        summary: "1 SC 2 CA".to_string(),
                    },
                },
                HostedOwnedFleet {
                    fleet_id: 2,
                    local_slot: 2,
                    coords: [10, 10],
                    target_coords: [10, 10],
                    order: "hold".to_string(),
                    order_summary: "Hold at Rigel".to_string(),
                    rules_of_engagement: 4,
                    current_speed: 4,
                    max_speed: 4,
                    ships: HostedFleetShips {
                        scout: 0,
                        battleship: 0,
                        cruiser: 1,
                        destroyer: 1,
                        transport: 0,
                        army: 0,
                        etac: 0,
                        total_starships: 2,
                        summary: "1 CA 1 DD".to_string(),
                    },
                },
            ],
        },
        queued_mail: vec![HostedQueuedMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3004,
            subject: "Scout".to_string(),
            body: "Hostiles near Rigel.".to_string(),
        }],
        report_blocks: vec![HostedReportBlock {
            viewer_empire_id: 1,
            block_index: 1,
            decoded_text: "Battle report".to_string(),
        }],
    }
}

struct StaticPlayfieldReproApp {
    title: &'static str,
    geometry: ScreenGeometry,
    should_quit: bool,
    render_playfield: fn() -> PlayfieldBuffer,
}

impl StaticPlayfieldReproApp {
    fn new(
        title: &'static str,
        geometry: ScreenGeometry,
        render_playfield: fn() -> PlayfieldBuffer,
    ) -> Self {
        Self {
            title,
            geometry,
            should_quit: false,
            render_playfield,
        }
    }
}

impl NativeApp for StaticPlayfieldReproApp {
    fn window_title(&self) -> &'static str {
        self.title
    }

    fn geometry(&self) -> ScreenGeometry {
        self.geometry
    }

    fn dispatch_key_event(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
            self.should_quit = true;
        }
    }

    fn dispatch_mouse_event(&mut self, _mouse: MouseEvent) -> bool {
        false
    }

    fn resize_canvas(&mut self, _cols: u16, _rows: u16) {}

    fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>> {
        Ok(UiScene::from((self.render_playfield)()))
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn set_should_quit(&mut self, should_quit: bool) {
        self.should_quit = should_quit;
    }

    fn debug_render_signature(&self) -> Option<String> {
        Some(format!(
            "static_repro title={} geometry={}x{}",
            self.title,
            self.geometry.width(),
            self.geometry.height()
        ))
    }
}

struct CursorMotionReproApp {
    geometry: ScreenGeometry,
    should_quit: bool,
    motion_count: u32,
    last_pointer: Option<(u16, u16)>,
    last_activity_at: Option<Instant>,
}

impl CursorMotionReproApp {
    fn new() -> Self {
        Self {
            geometry: ScreenGeometry::new(88, 24),
            should_quit: false,
            motion_count: 0,
            last_pointer: None,
            last_activity_at: None,
        }
    }
}

impl NativeApp for CursorMotionReproApp {
    fn window_title(&self) -> &'static str {
        "nc-dash glyphon cursor motion repro"
    }

    fn geometry(&self) -> ScreenGeometry {
        self.geometry
    }

    fn dispatch_key_event(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q')) {
            self.should_quit = true;
        }
    }

    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
        let pointer = is_live_pointer(mouse.column, mouse.row).then_some((mouse.column, mouse.row));
        if pointer == self.last_pointer {
            return false;
        }
        self.last_pointer = pointer;
        self.motion_count += 1;
        self.last_activity_at = Some(Instant::now());
        true
    }

    fn resize_canvas(&mut self, cols: u16, rows: u16) {
        self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
    }

    fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>> {
        let mut buffer = PlayfieldBuffer::new(
            self.geometry.width(),
            self.geometry.height(),
            theme::body_style(),
        );
        draw_motion_repro_grid(&mut buffer);
        buffer.write_text(1, 2, "glyphon native motion repro", theme::title_style());
        buffer.write_text(
            2,
            2,
            "Move the pointer. Esc or q exits.",
            theme::body_style(),
        );
        buffer.write_text(
            3,
            2,
            &format!("motion events: {}", self.motion_count),
            theme::table_header_style(),
        );
        let pointer_text = self
            .last_pointer
            .map(|(column, row)| format!("last cell: ({column},{row})"))
            .unwrap_or_else(|| "last cell: outside".to_string());
        buffer.write_text(4, 2, &pointer_text, theme::body_style());
        let mut signature = String::new();
        let _ = write!(
            signature,
            "signature: marker=@ guides=· box=─│┼ glyphs △ ⨁ ◊ αβγδε"
        );
        buffer.write_text(5, 2, &signature, theme::body_style());
        if let Some((column, row)) = self.last_pointer {
            if usize::from(column) < self.geometry.width()
                && usize::from(row) < self.geometry.height()
            {
                buffer.set_cell(
                    row as usize,
                    column as usize,
                    '@',
                    CellStyle::new(theme::body_style().bg, theme::value_style().fg, true),
                );
                buffer.set_cursor(column, row);
            }
        }
        Ok(UiScene::from(buffer))
    }

    fn note_user_activity(&mut self, now: Instant) {
        self.last_activity_at = Some(now);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn set_should_quit(&mut self, should_quit: bool) {
        self.should_quit = should_quit;
    }

    fn debug_render_signature(&self) -> Option<String> {
        Some(format!(
            "cursor_motion motions={} pointer={:?}",
            self.motion_count, self.last_pointer
        ))
    }
}

fn build_static_glyph_grid_playfield() -> PlayfieldBuffer {
    let geometry = ScreenGeometry::new(92, 26);
    let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), theme::body_style());
    buffer.write_text(1, 2, "glyphon glyph grid repro", theme::title_style());
    buffer.write_text(
        3,
        2,
        "Greek: α β γ δ ε ζ η θ  λ μ ξ π σ φ ω  Δ Σ Ω",
        theme::body_style(),
    );
    buffer.write_text(
        5,
        2,
        "Fleet/ICD markers: △  ⨁  ◊  @  #  ?  *  ·",
        theme::body_style(),
    );
    buffer.write_text(
        7,
        2,
        "Box drawing: ┌────────────┬────────────┐",
        theme::body_style(),
    );
    buffer.write_text(
        8,
        2,
        "             │ ┼ crosshair │   route    │",
        theme::body_style(),
    );
    buffer.write_text(
        9,
        2,
        "             └────────────┴────────────┘",
        theme::body_style(),
    );
    buffer.write_text(11, 2, "Blocks/shades: ░ ▒ ▓ █ ▀ ▄ ▌ ▐", theme::body_style());
    buffer.write_text(
        13,
        2,
        "Mixed row: Sol · Rigel △ route ⨁ status ◊ axis ┼ alpha α beta β",
        theme::body_style(),
    );
    buffer.write_text(
        15,
        2,
        "The forcing set here is intended for the new glyphon/wgpu path only.",
        theme::dim_style(),
    );
    buffer
}

fn draw_motion_repro_grid(buffer: &mut PlayfieldBuffer) {
    let width = buffer.width();
    let height = buffer.height();
    if width < 4 || height < 4 {
        return;
    }

    let border_style = theme::table_header_style();
    let guide_style = theme::dim_style();
    let axis_style = theme::table_header_style();

    draw_box(buffer, 0, 0, width - 1, height - 1, border_style);

    for row in 1..height - 1 {
        for col in 1..width - 1 {
            let ch = if row % 4 == 0 || col % 8 == 0 {
                '·'
            } else {
                ' '
            };
            buffer.set_cell(row, col, ch, guide_style);
        }
    }

    for col in (8..width.saturating_sub(1)).step_by(8) {
        let label = format!("{:02}", col);
        if col + label.len() < width - 1 {
            buffer.write_text(0, col, &label, axis_style);
        }
    }

    for row in (4..height.saturating_sub(1)).step_by(4) {
        let label = format!("{:02}", row);
        if label.len() < width - 2 {
            buffer.write_text(row, 1, &label, axis_style);
        }
    }
}

fn build_static_starmap_playfield() -> PlayfieldBuffer {
    let geometry = ScreenGeometry::new(96, 30);
    let mut buffer = PlayfieldBuffer::new(geometry.width(), geometry.height(), theme::body_style());
    buffer.write_text(1, 2, "glyphon static starmap repro", theme::title_style());
    buffer.write_text(
        2,
        2,
        "Forcing glyphs: △ empty fleet, ⨁ fleet on world, ◊ icd, · dense map, ─│┼ crosshair",
        theme::body_style(),
    );
    draw_box(&mut buffer, 4, 2, 91, 22, theme::table_header_style());

    for row in 5..25 {
        for col in 4..91 {
            if (row + col) % 2 == 0 {
                buffer.set_cell(row, col, '·', theme::dim_style());
            }
        }
    }

    for col in 6..89 {
        buffer.set_cell(14, col, '─', theme::map_crosshair_style());
    }
    for row in 6..24 {
        buffer.set_cell(row, 46, '│', theme::map_crosshair_style());
    }
    buffer.set_cell(14, 46, '┼', theme::map_center_style());

    buffer.set_cell(8, 18, '◊', theme::icd_style());
    buffer.set_cell(10, 30, '#', theme::value_style());
    buffer.set_cell(12, 58, '⨁', theme::value_style());
    buffer.set_cell(18, 70, '△', theme::value_style());
    buffer.set_cell(20, 24, '?', theme::body_style());
    buffer.set_cell(22, 60, '*', theme::body_style());

    buffer.write_text(25, 4, "Legend:", theme::table_header_style());
    buffer.write_text(
        26,
        4,
        "◊ icd   ⨁ fleet-on-world   △ fleet-empty",
        theme::body_style(),
    );
    buffer.write_text(
        27,
        4,
        "# known world   ? unknown   * partial",
        theme::body_style(),
    );
    buffer
}

fn draw_box(
    buffer: &mut PlayfieldBuffer,
    top: usize,
    left: usize,
    right: usize,
    bottom: usize,
    style: CellStyle,
) {
    for col in left + 1..right {
        buffer.set_cell(top, col, '─', style);
        buffer.set_cell(bottom, col, '─', style);
    }
    for row in top + 1..bottom {
        buffer.set_cell(row, left, '│', style);
        buffer.set_cell(row, right, '│', style);
    }
    buffer.set_cell(top, left, '┌', style);
    buffer.set_cell(top, right, '┐', style);
    buffer.set_cell(bottom, left, '└', style);
    buffer.set_cell(bottom, right, '┘', style);
}

fn is_live_pointer(column: u16, row: u16) -> bool {
    column != u16::MAX && row != u16::MAX
}

#[cfg(test)]
mod tests {
    use super::{
        build_static_glyph_grid_playfield, build_static_starmap_playfield,
        sample_hosted_dashboard_snapshot,
    };

    #[test]
    fn glyph_grid_repro_contains_forcing_glyphs() {
        let buffer = build_static_glyph_grid_playfield();
        let joined = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>()
            .join("\n");

        for ch in ['α', '△', '⨁', '◊', '·', '┼', '░', '▒', '▓'] {
            assert!(joined.contains(ch), "missing glyph {ch}");
        }
    }

    #[test]
    fn static_starmap_repro_contains_forcing_glyphs() {
        let buffer = build_static_starmap_playfield();
        let joined = (0..buffer.height())
            .map(|row| buffer.plain_line(row))
            .collect::<Vec<_>>()
            .join("\n");

        for ch in ['△', '⨁', '◊', '·', '─', '│', '┼'] {
            assert!(joined.contains(ch), "missing glyph {ch}");
        }
    }

    #[test]
    fn hosted_snapshot_fixture_contains_empty_and_world_fleet_repro_cases() {
        let snapshot = sample_hosted_dashboard_snapshot();
        assert_eq!(snapshot.state.owned_fleets.len(), 2);
        assert!(
            snapshot
                .state
                .owned_fleets
                .iter()
                .any(|fleet| fleet.coords == [12, 9])
        );
        assert!(
            snapshot
                .state
                .owned_fleets
                .iter()
                .any(|fleet| fleet.coords == [10, 10])
        );
    }
}
