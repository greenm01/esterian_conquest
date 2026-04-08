use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use nc_ui::native::{
    CellGridWindowRenderer, cell_position_at_pixel, crossterm_key_event_from_winit,
    terminal_grid_for_pixels,
};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{Event, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, WindowBuilder};

use crate::app::state::DashApp;

const WINDOW_TITLE: &str = "Nostrian Conquest Dashboard";
const OUTSIDE_MOUSE_COORD: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, PartialEq)]
enum PendingPointer {
    Outside,
    Cell(u16, u16),
}

#[derive(Clone, Copy, Debug)]
enum DashMsg {
    CloseRequested,
    KeyInput(crossterm::event::KeyEvent),
    ModifiersChanged(ModifiersState),
    MouseButton {
        button: WinitMouseButton,
        pressed: bool,
    },
    QueuePointer(PendingPointer),
    FlushPointer,
    WindowResized {
        pixel_width: u32,
        pixel_height: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DashEffect {
    Exit,
    RequestRedraw,
}

struct NativeDashShell {
    app: DashApp,
    window_pixel_width: u32,
    window_pixel_height: u32,
    modifiers: ModifiersState,
    pending_pointer: Option<PendingPointer>,
    current_pointer: Option<PendingPointer>,
    left_mouse_down: bool,
    needs_redraw: bool,
}

impl NativeDashShell {
    fn new(app: DashApp, window_pixel_width: u32, window_pixel_height: u32) -> Self {
        Self {
            app,
            window_pixel_width: window_pixel_width.max(1),
            window_pixel_height: window_pixel_height.max(1),
            modifiers: ModifiersState::empty(),
            pending_pointer: None,
            current_pointer: None,
            left_mouse_down: false,
            needs_redraw: true,
        }
    }

    fn update(&mut self, msg: DashMsg) -> Vec<DashEffect> {
        let mut effects = Vec::new();
        match msg {
            DashMsg::CloseRequested => {
                self.app.should_quit = true;
                effects.push(DashEffect::Exit);
            }
            DashMsg::KeyInput(key) => {
                self.app.dispatch_key_event(key);
                self.push_state_effects(&mut effects, true);
            }
            DashMsg::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
            }
            DashMsg::MouseButton { button, pressed } => {
                if let Some(pointer_effects) = self.flush_pointer() {
                    effects.extend(pointer_effects);
                }
                if let Some(mouse_button) = map_mouse_button(button) {
                    if mouse_button == MouseButton::Left {
                        self.left_mouse_down = pressed;
                    }
                    self.app.dispatch_mouse_event(MouseEvent {
                        kind: if pressed {
                            MouseEventKind::Down(mouse_button)
                        } else {
                            MouseEventKind::Up(mouse_button)
                        },
                        column: self.pointer_column(),
                        row: self.pointer_row(),
                        modifiers: key_modifiers(self.modifiers),
                    });
                    self.push_state_effects(&mut effects, true);
                }
            }
            DashMsg::QueuePointer(pointer) => {
                coalesce_pointer_move(&mut self.pending_pointer, pointer);
                if self.left_mouse_down {
                    if let Some(pointer_effects) = self.flush_pointer() {
                        effects.extend(pointer_effects);
                    }
                }
            }
            DashMsg::FlushPointer => {
                if let Some(pointer_effects) = self.flush_pointer() {
                    effects.extend(pointer_effects);
                }
            }
            DashMsg::WindowResized {
                pixel_width,
                pixel_height,
            } => {
                if self.resize_to_window_pixels(pixel_width, pixel_height) {
                    self.push_state_effects(&mut effects, true);
                }
            }
        }
        effects
    }

    fn flush_pointer(&mut self) -> Option<Vec<DashEffect>> {
        let pending = next_pointer_dispatch(self.current_pointer, self.pending_pointer.take())?;
        if self.current_pointer == Some(pending) {
            return None;
        }
        self.current_pointer = Some(pending);
        let kind = pointer_event_kind(self.left_mouse_down);
        let (column, row) = pointer_coords(Some(pending));
        self.app.dispatch_mouse_event(MouseEvent {
            kind,
            column,
            row,
            modifiers: key_modifiers(self.modifiers),
        });
        let mut effects = Vec::new();
        self.push_state_effects(&mut effects, true);
        Some(effects)
    }

    fn pointer_column(&self) -> u16 {
        pointer_coords(self.current_pointer).0
    }

    fn pointer_row(&self) -> u16 {
        pointer_coords(self.current_pointer).1
    }

    fn push_state_effects(&mut self, effects: &mut Vec<DashEffect>, redraw: bool) {
        if redraw {
            self.needs_redraw = true;
            effects.push(DashEffect::RequestRedraw);
        }
        if self.app.should_quit {
            effects.push(DashEffect::Exit);
        }
    }

    fn resize_to_window_pixels(&mut self, pixel_width: u32, pixel_height: u32) -> bool {
        let pixel_width = pixel_width.max(1);
        let pixel_height = pixel_height.max(1);
        let (cols, rows) = terminal_grid_for_pixels(pixel_width, pixel_height);
        let geometry_changed = self.app.geometry.width() != cols as usize
            || self.app.geometry.height() != rows as usize;
        if self.window_pixel_width == pixel_width
            && self.window_pixel_height == pixel_height
            && !geometry_changed
        {
            return false;
        }

        self.window_pixel_width = pixel_width;
        self.window_pixel_height = pixel_height;
        self.app.resize_canvas(cols, rows);
        true
    }
}

pub fn run(app: DashApp) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let logical_width = (app.geometry.width() * nc_ui::native::DEFAULT_CELL_WIDTH) as f64;
    let logical_height = (app.geometry.height() * nc_ui::native::DEFAULT_CELL_HEIGHT) as f64;
    let window = Box::new(
        WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(logical_width, logical_height))
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .with_resizable(true)
            .build(&event_loop)?,
    );
    let window: &'static winit::window::Window = Box::leak(window);
    let mut renderer = CellGridWindowRenderer::new(window)?;
    let initial_size = window.inner_size();
    let mut shell = NativeDashShell::new(app, initial_size.width, initial_size.height);
    dispatch(
        &mut shell,
        window,
        DashMsg::WindowResized {
            pixel_width: initial_size.width,
            pixel_height: initial_size.height,
        },
        false,
    );

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    dispatch(&mut shell, window, DashMsg::CloseRequested, true);
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    dispatch(
                        &mut shell,
                        window,
                        DashMsg::ModifiersChanged(modifiers.state()),
                        false,
                    );
                }
                WindowEvent::Resized(size) => {
                    dispatch(
                        &mut shell,
                        window,
                        DashMsg::WindowResized {
                            pixel_width: size.width,
                            pixel_height: size.height,
                        },
                        false,
                    );
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let size = window.inner_size();
                    dispatch(
                        &mut shell,
                        window,
                        DashMsg::WindowResized {
                            pixel_width: size.width,
                            pixel_height: size.height,
                        },
                        false,
                    );
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pointer = pointer_from_position(&shell, position);
                    dispatch(&mut shell, window, DashMsg::QueuePointer(pointer), false);
                }
                WindowEvent::CursorLeft { .. } => {
                    dispatch(
                        &mut shell,
                        window,
                        DashMsg::QueuePointer(PendingPointer::Outside),
                        false,
                    );
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    dispatch(
                        &mut shell,
                        window,
                        DashMsg::MouseButton {
                            button,
                            pressed: state.is_pressed(),
                        },
                        true,
                    );
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    let Some(key) = crossterm_key_event_from_winit(&event, shell.modifiers) else {
                        return;
                    };
                    dispatch(&mut shell, window, DashMsg::KeyInput(key), true);
                }
                WindowEvent::RedrawRequested => {
                    sync_window_size(&mut shell, window);
                    let size = window.inner_size();
                    match shell.app.render_playfield() {
                        Ok(buffer) => {
                            if let Err(err) = renderer.render(&buffer, size.width, size.height) {
                                crate::show_fatal_error(&format!(
                                    "unable to render nc-dash window: {err}"
                                ));
                                elwt.exit();
                            } else {
                                shell.needs_redraw = false;
                            }
                        }
                        Err(err) => {
                            crate::show_fatal_error(&err.to_string());
                            elwt.exit();
                        }
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                sync_window_size(&mut shell, window);
                dispatch(&mut shell, window, DashMsg::FlushPointer, false);
                if shell.app.should_quit {
                    elwt.exit();
                } else if shell.needs_redraw {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

fn dispatch(
    shell: &mut NativeDashShell,
    window: &winit::window::Window,
    msg: DashMsg,
    exit_immediately: bool,
) {
    let effects = shell.update(msg);
    apply_effects(shell, window, effects, exit_immediately);
}

fn apply_effects(
    shell: &mut NativeDashShell,
    window: &winit::window::Window,
    effects: Vec<DashEffect>,
    exit_immediately: bool,
) {
    for effect in effects {
        match effect {
            DashEffect::RequestRedraw => shell.needs_redraw = true,
            DashEffect::Exit if exit_immediately => {
                shell.app.should_quit = true;
            }
            DashEffect::Exit => {}
        }
    }
    if shell.needs_redraw {
        window.request_redraw();
    }
}

fn sync_window_size(shell: &mut NativeDashShell, window: &winit::window::Window) {
    let size = window.inner_size();
    if shell.resize_to_window_pixels(size.width, size.height) {
        shell.needs_redraw = true;
    }
}

fn pointer_from_position(
    shell: &NativeDashShell,
    position: PhysicalPosition<f64>,
) -> PendingPointer {
    cell_position_at_pixel(
        shell.app.geometry.width(),
        shell.app.geometry.height(),
        shell.window_pixel_width,
        shell.window_pixel_height,
        position,
    )
    .map(|(column, row)| PendingPointer::Cell(column, row))
    .unwrap_or(PendingPointer::Outside)
}

fn pointer_event_kind(left_mouse_down: bool) -> MouseEventKind {
    if left_mouse_down {
        MouseEventKind::Drag(MouseButton::Left)
    } else {
        MouseEventKind::Moved
    }
}

fn pointer_coords(pointer: Option<PendingPointer>) -> (u16, u16) {
    match pointer {
        Some(PendingPointer::Cell(column, row)) => (column, row),
        Some(PendingPointer::Outside) | None => (OUTSIDE_MOUSE_COORD, OUTSIDE_MOUSE_COORD),
    }
}

fn coalesce_pointer_move(pending: &mut Option<PendingPointer>, pointer: PendingPointer) {
    *pending = Some(pointer);
}

fn next_pointer_dispatch(
    current: Option<PendingPointer>,
    pending: Option<PendingPointer>,
) -> Option<PendingPointer> {
    let pending = pending?;
    (current != Some(pending)).then_some(pending)
}

fn map_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

fn key_modifiers(modifiers: ModifiersState) -> KeyModifiers {
    let mut mapped = KeyModifiers::empty();
    if modifiers.shift_key() {
        mapped.insert(KeyModifiers::SHIFT);
    }
    if modifiers.control_key() {
        mapped.insert(KeyModifiers::CONTROL);
    }
    if modifiers.alt_key() {
        mapped.insert(KeyModifiers::ALT);
    }
    mapped
}

#[cfg(test)]
mod tests {
    use super::{
        NativeDashShell, PendingPointer, coalesce_pointer_move, next_pointer_dispatch,
        pointer_coords, pointer_event_kind,
    };
    use crossterm::event::MouseEventKind;
    use nc_data::GameStateBuilder;
    use nc_ui::ScreenGeometry;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use crate::app::state::DashApp;

    #[test]
    fn outside_pointer_maps_to_sentinel_coords() {
        assert_eq!(pointer_coords(None), (u16::MAX, u16::MAX));
        assert_eq!(
            pointer_coords(Some(PendingPointer::Outside)),
            (u16::MAX, u16::MAX)
        );
    }

    #[test]
    fn drag_kind_only_applies_while_left_button_is_held() {
        assert_eq!(pointer_event_kind(false), MouseEventKind::Moved);
        assert_eq!(
            pointer_event_kind(true),
            MouseEventKind::Drag(crossterm::event::MouseButton::Left)
        );
    }

    #[test]
    fn later_pointer_move_replaces_earlier_pending_move() {
        let mut pending = None;
        coalesce_pointer_move(&mut pending, PendingPointer::Cell(10, 4));
        coalesce_pointer_move(&mut pending, PendingPointer::Cell(22, 11));

        assert_eq!(pending, Some(PendingPointer::Cell(22, 11)));
    }

    #[test]
    fn unchanged_pointer_position_does_not_dispatch_again() {
        assert_eq!(
            next_pointer_dispatch(
                Some(PendingPointer::Cell(12, 8)),
                Some(PendingPointer::Cell(12, 8)),
            ),
            None
        );
        assert_eq!(
            next_pointer_dispatch(
                Some(PendingPointer::Cell(12, 8)),
                Some(PendingPointer::Outside),
            ),
            Some(PendingPointer::Outside)
        );
    }

    #[test]
    fn resize_updates_app_geometry_even_when_cached_pixels_match() {
        let mut shell = test_shell(ScreenGeometry::new(1, 1), 100, 54);

        assert!(shell.resize_to_window_pixels(100, 54));
        assert_eq!(shell.app.geometry, ScreenGeometry::new(10, 3));
    }

    #[test]
    fn resize_noops_when_pixels_and_grid_are_already_current() {
        let mut shell = test_shell(ScreenGeometry::new(10, 3), 100, 54);

        assert!(!shell.resize_to_window_pixels(100, 54));
        assert_eq!(shell.app.geometry, ScreenGeometry::new(10, 3));
    }

    fn test_shell(
        app_geometry: ScreenGeometry,
        window_pixel_width: u32,
        window_pixel_height: u32,
    ) -> NativeDashShell {
        let app = DashApp::new_for_tests(
            PathBuf::from("."),
            GameStateBuilder::new()
                .with_player_count(4)
                .build_initialized_baseline()
                .expect("baseline"),
            BTreeMap::new(),
            BTreeSet::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            app_geometry,
            ScreenGeometry::new(0, 0),
            1,
        );
        NativeDashShell::new(app, window_pixel_width, window_pixel_height)
    }
}
