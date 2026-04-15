use crate::geometry::ScreenGeometry;
use crate::native_grid::{
    CellGridWindowRenderer, cell_position_at_pixel, crossterm_key_event_from_winit,
    terminal_grid_for_pixels,
};
use crate::rendered::RenderedUi;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{Event, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, WindowBuilder};

pub(crate) trait NativeApp {
    fn window_title(&self) -> &'static str;
    fn geometry(&self) -> ScreenGeometry;
    fn dispatch_key_event(&mut self, key: crossterm::event::KeyEvent);
    fn dispatch_mouse_event(&mut self, mouse: MouseEvent);
    fn resize_canvas(&mut self, cols: u16, rows: u16);
    fn render_ui(&self) -> Result<RenderedUi, Box<dyn std::error::Error>>;
    fn on_idle(&mut self) -> bool {
        false
    }
    fn is_dragging_surface(&self) -> bool {
        false
    }
    fn note_user_activity(&mut self, _now: Instant) {}
    fn next_wakeup(&self) -> Option<Instant> {
        None
    }
    fn should_quit(&self) -> bool;
    fn set_should_quit(&mut self, should_quit: bool);
}

const OUTSIDE_MOUSE_COORD: u16 = u16::MAX;
const DRAG_REDRAW_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, Debug, PartialEq)]
enum PendingPointer {
    Outside,
    Cell(u16, u16),
}

#[derive(Clone, Copy, Debug)]
enum NativeMsg {
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
enum NativeEffect {
    Exit,
    RequestRedraw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RedrawSchedule {
    None,
    Immediate,
    Deferred(Instant),
}

struct NativeShell<T: NativeApp> {
    app: T,
    window_pixel_width: u32,
    window_pixel_height: u32,
    modifiers: ModifiersState,
    pending_pointer: Option<PendingPointer>,
    current_pointer: Option<PendingPointer>,
    left_mouse_down: bool,
    needs_redraw: bool,
    redraw_requested: bool,
    drag_redraw_pending: bool,
    last_drag_redraw_at: Option<Instant>,
}

impl<T: NativeApp> NativeShell<T> {
    fn new(app: T, window_pixel_width: u32, window_pixel_height: u32) -> Self {
        Self {
            app,
            window_pixel_width: window_pixel_width.max(1),
            window_pixel_height: window_pixel_height.max(1),
            modifiers: ModifiersState::empty(),
            pending_pointer: None,
            current_pointer: None,
            left_mouse_down: false,
            needs_redraw: true,
            redraw_requested: false,
            drag_redraw_pending: false,
            last_drag_redraw_at: None,
        }
    }

    fn update(&mut self, msg: NativeMsg) -> Vec<NativeEffect> {
        let mut effects = Vec::new();
        match msg {
            NativeMsg::CloseRequested => {
                self.app.set_should_quit(true);
                effects.push(NativeEffect::Exit);
            }
            NativeMsg::KeyInput(key) => {
                self.app.dispatch_key_event(key);
                self.push_state_effects(&mut effects, true);
            }
            NativeMsg::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
            }
            NativeMsg::MouseButton { button, pressed } => {
                if self.flush_pointer(false) {
                    self.push_state_effects(&mut effects, false);
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
                    if mouse_button == MouseButton::Left && !pressed {
                        self.clear_drag_redraw_state();
                    }
                    self.push_state_effects(&mut effects, true);
                }
            }
            NativeMsg::QueuePointer(pointer) => {
                coalesce_pointer_move(&mut self.pending_pointer, pointer);
                if self.is_dragging_surface()
                    && next_pointer_dispatch(self.current_pointer, self.pending_pointer).is_some()
                {
                    self.drag_redraw_pending = true;
                    self.needs_redraw = true;
                } else if self.left_mouse_down
                    && next_pointer_dispatch(self.current_pointer, self.pending_pointer).is_some()
                {
                    self.push_state_effects(&mut effects, true);
                }
            }
            NativeMsg::FlushPointer => {
                if self.flush_pointer(true) {
                    self.push_state_effects(&mut effects, true);
                }
            }
            NativeMsg::WindowResized {
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

    fn flush_pointer(&mut self, request_redraw: bool) -> bool {
        let Some(pending) =
            next_pointer_dispatch(self.current_pointer, self.pending_pointer.take())
        else {
            return false;
        };
        self.current_pointer = Some(pending);
        let kind = pointer_event_kind(self.left_mouse_down);
        let (column, row) = pointer_coords(Some(pending));
        self.app.dispatch_mouse_event(MouseEvent {
            kind,
            column,
            row,
            modifiers: key_modifiers(self.modifiers),
        });
        if request_redraw {
            self.needs_redraw = true;
        }
        true
    }

    fn pointer_column(&self) -> u16 {
        pointer_coords(self.current_pointer).0
    }

    fn pointer_row(&self) -> u16 {
        pointer_coords(self.current_pointer).1
    }

    fn push_state_effects(&mut self, effects: &mut Vec<NativeEffect>, redraw: bool) {
        if redraw {
            self.needs_redraw = true;
            effects.push(NativeEffect::RequestRedraw);
        }
        if self.app.should_quit() {
            effects.push(NativeEffect::Exit);
        }
    }

    fn resize_to_window_pixels(&mut self, pixel_width: u32, pixel_height: u32) -> bool {
        let pixel_width = pixel_width.max(1);
        let pixel_height = pixel_height.max(1);
        let (cols, rows) = terminal_grid_for_pixels(pixel_width, pixel_height);
        let geometry = self.app.geometry();
        let geometry_changed =
            geometry.width() != cols as usize || geometry.height() != rows as usize;
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

    fn is_dragging_surface(&self) -> bool {
        self.left_mouse_down && self.app.is_dragging_surface()
    }

    fn clear_drag_redraw_state(&mut self) {
        self.drag_redraw_pending = false;
        self.last_drag_redraw_at = None;
    }

    fn drag_redraw_deadline(&self) -> Option<Instant> {
        if self.drag_redraw_pending && self.is_dragging_surface() {
            Some(
                self.last_drag_redraw_at
                    .map(|last| last + DRAG_REDRAW_INTERVAL)
                    .unwrap_or_else(Instant::now),
            )
        } else {
            None
        }
    }

    fn next_redraw_schedule(&self, now: Instant) -> RedrawSchedule {
        if self.redraw_requested || !self.needs_redraw {
            return RedrawSchedule::None;
        }
        if let Some(deadline) = self.drag_redraw_deadline() {
            if deadline <= now {
                RedrawSchedule::Immediate
            } else {
                RedrawSchedule::Deferred(deadline)
            }
        } else {
            RedrawSchedule::Immediate
        }
    }

    fn note_rendered_drag_frame(&mut self, now: Instant) {
        if self.drag_redraw_pending {
            self.drag_redraw_pending = false;
            self.last_drag_redraw_at = Some(now);
        }
    }
}

pub fn run<T: NativeApp>(app: T) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    let geometry = app.geometry();
    let logical_width = (geometry.width() * crate::native_grid::DEFAULT_CELL_WIDTH) as f64;
    let logical_height = (geometry.height() * crate::native_grid::DEFAULT_CELL_HEIGHT) as f64;
    let window = Box::new(
        WindowBuilder::new()
            .with_title(app.window_title())
            .with_inner_size(LogicalSize::new(logical_width, logical_height))
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .with_resizable(true)
            .build(&event_loop)?,
    );
    let window: &'static winit::window::Window = Box::leak(window);
    let mut renderer = CellGridWindowRenderer::new(window)?;
    let initial_size = window.inner_size();
    let mut shell = NativeShell::new(app, initial_size.width, initial_size.height);
    dispatch(
        &mut shell,
        window,
        NativeMsg::WindowResized {
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
                    dispatch(&mut shell, window, NativeMsg::CloseRequested, true);
                }
                WindowEvent::ModifiersChanged(modifiers) => {
                    dispatch(
                        &mut shell,
                        window,
                        NativeMsg::ModifiersChanged(modifiers.state()),
                        false,
                    );
                }
                WindowEvent::Resized(size) => {
                    dispatch(
                        &mut shell,
                        window,
                        NativeMsg::WindowResized {
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
                        NativeMsg::WindowResized {
                            pixel_width: size.width,
                            pixel_height: size.height,
                        },
                        false,
                    );
                }
                WindowEvent::CursorMoved { position, .. } => {
                    shell.app.note_user_activity(Instant::now());
                    let pointer = pointer_from_position(&shell, position);
                    dispatch(&mut shell, window, NativeMsg::QueuePointer(pointer), false);
                }
                WindowEvent::CursorLeft { .. } => {
                    dispatch(
                        &mut shell,
                        window,
                        NativeMsg::QueuePointer(PendingPointer::Outside),
                        false,
                    );
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    shell.app.note_user_activity(Instant::now());
                    dispatch(
                        &mut shell,
                        window,
                        NativeMsg::MouseButton {
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
                    shell.app.note_user_activity(Instant::now());
                    dispatch(&mut shell, window, NativeMsg::KeyInput(key), true);
                }
                WindowEvent::RedrawRequested => {
                    shell.redraw_requested = false;
                    sync_window_size(&mut shell, window);
                    let was_drag_redraw = shell.drag_redraw_pending;
                    let _ = shell.flush_pointer(false);
                    if shell.app.should_quit() {
                        elwt.exit();
                        return;
                    }
                    let size = window.inner_size();
                    match shell.app.render_ui() {
                        Ok(rendered) => {
                            if let Err(err) = renderer.render(&rendered, size.width, size.height) {
                                crate::show_fatal_error(&format!(
                                    "unable to render nc-dash window: {err}"
                                ));
                                elwt.exit();
                            } else {
                                if was_drag_redraw {
                                    shell.note_rendered_drag_frame(Instant::now());
                                }
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
                if !shell.redraw_requested && !shell.is_dragging_surface() {
                    dispatch(&mut shell, window, NativeMsg::FlushPointer, false);
                }
                if shell.app.on_idle() {
                    shell.needs_redraw = true;
                }
                if shell.app.should_quit() {
                    elwt.exit();
                } else {
                    let now = Instant::now();
                    match shell.next_redraw_schedule(now) {
                        RedrawSchedule::Immediate => {
                            window.request_redraw();
                            shell.redraw_requested = true;
                        }
                        RedrawSchedule::Deferred(deadline) => {
                            elwt.set_control_flow(ControlFlow::WaitUntil(
                                combine_deadlines(Some(deadline), shell.app.next_wakeup())
                                    .expect("deferred redraw has a deadline"),
                            ));
                        }
                        RedrawSchedule::None => {
                            if let Some(deadline) = shell.app.next_wakeup() {
                                if deadline <= now {
                                    window.request_redraw();
                                    shell.redraw_requested = true;
                                } else {
                                    elwt.set_control_flow(ControlFlow::WaitUntil(deadline));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

fn dispatch<T: NativeApp>(
    shell: &mut NativeShell<T>,
    window: &winit::window::Window,
    msg: NativeMsg,
    exit_immediately: bool,
) {
    let effects = shell.update(msg);
    apply_effects(shell, window, effects, exit_immediately);
}

fn apply_effects<T: NativeApp>(
    shell: &mut NativeShell<T>,
    window: &winit::window::Window,
    effects: Vec<NativeEffect>,
    exit_immediately: bool,
) {
    for effect in effects {
        match effect {
            NativeEffect::RequestRedraw => shell.needs_redraw = true,
            NativeEffect::Exit if exit_immediately => shell.app.set_should_quit(true),
            NativeEffect::Exit => {}
        }
    }
    if shell.needs_redraw && !shell.redraw_requested {
        window.request_redraw();
        shell.redraw_requested = true;
    }
}

fn sync_window_size<T: NativeApp>(shell: &mut NativeShell<T>, window: &winit::window::Window) {
    let size = window.inner_size();
    if shell.resize_to_window_pixels(size.width, size.height) {
        shell.needs_redraw = true;
    }
}

fn pointer_from_position<T: NativeApp>(
    shell: &NativeShell<T>,
    position: PhysicalPosition<f64>,
) -> PendingPointer {
    let geometry = shell.app.geometry();
    cell_position_at_pixel(
        geometry.width(),
        geometry.height(),
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

fn combine_deadlines(left: Option<Instant>, right: Option<Instant>) -> Option<Instant> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
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
        DRAG_REDRAW_INTERVAL, NativeApp, NativeEffect, NativeMsg, NativeShell, PendingPointer,
        RedrawSchedule, coalesce_pointer_move, next_pointer_dispatch, pointer_coords,
        pointer_event_kind,
    };
    use crate::geometry::ScreenGeometry;
    use crate::RenderedUi;
    use crossterm::event::{MouseEvent, MouseEventKind};
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use crate::app::state::DashApp;
    use crate::theme;

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

    #[test]
    fn drag_queue_requests_redraw_without_immediate_pointer_dispatch() {
        let mut shell = test_shell(ScreenGeometry::new(10, 3), 100, 54);
        shell.left_mouse_down = true;
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));

        let effects = shell.update(NativeMsg::QueuePointer(PendingPointer::Cell(7, 2)));

        assert_eq!(effects, vec![NativeEffect::RequestRedraw]);
        assert_eq!(shell.current_pointer, Some(PendingPointer::Cell(3, 1)));
        assert_eq!(shell.pending_pointer, Some(PendingPointer::Cell(7, 2)));
    }

    #[test]
    fn flushing_pointer_uses_latest_coalesced_drag_position() {
        let mut shell = test_shell(ScreenGeometry::new(10, 3), 100, 54);
        shell.left_mouse_down = true;
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));

        shell.update(NativeMsg::QueuePointer(PendingPointer::Cell(4, 1)));
        shell.update(NativeMsg::QueuePointer(PendingPointer::Cell(8, 2)));

        assert!(shell.flush_pointer(false));
        assert_eq!(shell.current_pointer, Some(PendingPointer::Cell(8, 2)));
        assert_eq!(shell.pending_pointer, None);
    }

    #[test]
    fn unchanged_drag_cell_does_not_request_redraw() {
        let mut shell = test_shell(ScreenGeometry::new(10, 3), 100, 54);
        shell.left_mouse_down = true;
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));

        let effects = shell.update(NativeMsg::QueuePointer(PendingPointer::Cell(3, 1)));

        assert!(effects.is_empty());
        assert_eq!(shell.pending_pointer, Some(PendingPointer::Cell(3, 1)));
    }

    #[test]
    fn dragging_surface_queues_throttled_redraw_instead_of_immediate_request() {
        let mut shell = test_drag_shell(ScreenGeometry::new(10, 3), 100, 54);
        shell.left_mouse_down = true;
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));

        let effects = shell.update(NativeMsg::QueuePointer(PendingPointer::Cell(7, 2)));

        assert!(effects.is_empty());
        assert!(shell.needs_redraw);
        assert!(shell.drag_redraw_pending);
        assert_eq!(shell.pending_pointer, Some(PendingPointer::Cell(7, 2)));
    }

    #[test]
    fn drag_redraw_is_deferred_until_interval_after_previous_drag_frame() {
        let mut shell = test_drag_shell(ScreenGeometry::new(10, 3), 100, 54);
        shell.left_mouse_down = true;
        shell.needs_redraw = true;
        shell.drag_redraw_pending = true;
        let now = Instant::now();
        shell.last_drag_redraw_at = Some(now);

        assert_eq!(
            shell.next_redraw_schedule(now + Duration::from_millis(5)),
            RedrawSchedule::Deferred(now + DRAG_REDRAW_INTERVAL)
        );
        assert_eq!(
            shell.next_redraw_schedule(now + DRAG_REDRAW_INTERVAL),
            RedrawSchedule::Immediate
        );
    }

    #[test]
    fn left_button_release_clears_drag_redraw_state() {
        let mut shell = test_drag_shell(ScreenGeometry::new(10, 3), 100, 54);
        shell.left_mouse_down = true;
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));
        shell.pending_pointer = Some(PendingPointer::Cell(7, 2));
        shell.drag_redraw_pending = true;
        shell.last_drag_redraw_at = Some(Instant::now());

        shell.update(NativeMsg::MouseButton {
            button: winit::event::MouseButton::Left,
            pressed: false,
        });

        assert!(!shell.left_mouse_down);
        assert!(!shell.drag_redraw_pending);
        assert!(shell.last_drag_redraw_at.is_none());
    }

    fn test_shell(
        app_geometry: ScreenGeometry,
        window_pixel_width: u32,
        window_pixel_height: u32,
    ) -> NativeShell<DashApp> {
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
        NativeShell::new(app, window_pixel_width, window_pixel_height)
    }

    fn test_drag_shell(
        app_geometry: ScreenGeometry,
        window_pixel_width: u32,
        window_pixel_height: u32,
    ) -> NativeShell<TestApp> {
        NativeShell::new(
            TestApp {
                geometry: app_geometry,
                dragging_surface: true,
                should_quit: false,
            },
            window_pixel_width,
            window_pixel_height,
        )
    }

    struct TestApp {
        geometry: ScreenGeometry,
        dragging_surface: bool,
        should_quit: bool,
    }

    impl NativeApp for TestApp {
        fn window_title(&self) -> &'static str {
            "test"
        }

        fn geometry(&self) -> ScreenGeometry {
            self.geometry
        }

        fn dispatch_key_event(&mut self, _key: crossterm::event::KeyEvent) {}

        fn dispatch_mouse_event(&mut self, _mouse: MouseEvent) {}

        fn resize_canvas(&mut self, cols: u16, rows: u16) {
            self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
        }

        fn render_ui(&self) -> Result<RenderedUi, Box<dyn std::error::Error>> {
            Ok(RenderedUi::from_playfield(
                &crate::buffer::PlayfieldBuffer::new(
                    self.geometry.width(),
                    self.geometry.height(),
                    theme::body_style(),
                ),
                theme::tui_theme().cursor,
            ))
        }

        fn is_dragging_surface(&self) -> bool {
            self.dragging_surface
        }

        fn should_quit(&self) -> bool {
            self.should_quit
        }

        fn set_should_quit(&mut self, should_quit: bool) {
            self.should_quit = should_quit;
        }
    }
}
