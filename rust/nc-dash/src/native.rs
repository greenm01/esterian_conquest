use crate::geometry::ScreenGeometry;
use crate::native_grid::{
    CellGridWindowRenderer, cell_position_at_pixel, crossterm_key_event_from_winit,
    logical_window_size_for_grid, terminal_grid_for_pixels,
};
use crate::rendered::RenderedUi;
use crate::startup::{NativeBackendPreference, NativeLaunchOptions, NativeWindowMode};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use nc_log::LogLevel;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::error::EventLoopError;
use winit::event::{MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::wayland::EventLoopBuilderExtWayland;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::x11::EventLoopBuilderExtX11;

pub(crate) trait NativeApp {
    fn window_title(&self) -> &'static str;
    fn geometry(&self) -> ScreenGeometry;
    fn dispatch_key_event(&mut self, key: crossterm::event::KeyEvent);
    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool;
    fn resize_canvas(&mut self, cols: u16, rows: u16);
    fn render_ui(&self) -> Result<RenderedUi, Box<dyn std::error::Error>>;
    fn debug_render_signature(&self) -> Option<String> {
        None
    }
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
const HOVER_REDRAW_INTERVAL: Duration = Duration::from_millis(16);

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

impl NativeMsg {
    fn label(self) -> &'static str {
        match self {
            Self::CloseRequested => "close_requested",
            Self::KeyInput(_) => "key_input",
            Self::ModifiersChanged(_) => "modifiers_changed",
            Self::MouseButton { .. } => "mouse_button",
            Self::QueuePointer(_) => "queue_pointer",
            Self::FlushPointer => "flush_pointer",
            Self::WindowResized { .. } => "window_resized",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeEffect {
    Exit,
    RequestRedraw(NativeRedrawCause),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RedrawSchedule {
    None,
    Immediate,
    Deferred(Instant),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeStage {
    Startup,
    EventLoopBuild,
    WindowCreate,
    RendererInit,
    InitialResize,
    EventLoopWait,
    FirstRedrawRequested,
    FirstFrameRender,
    FirstFrameRendered,
}

impl NativeStage {
    fn label(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::EventLoopBuild => "event_loop_build",
            Self::WindowCreate => "window_create",
            Self::RendererInit => "renderer_init",
            Self::InitialResize => "initial_resize",
            Self::EventLoopWait => "event_loop_wait",
            Self::FirstRedrawRequested => "first_redraw_requested",
            Self::FirstFrameRender => "first_frame_render",
            Self::FirstFrameRendered => "first_frame_rendered",
        }
    }
}

#[derive(Debug, Clone)]
struct NativeDiagnostics {
    stage: NativeStage,
    first_frame_rendered: bool,
    last_input_cause: Option<NativeInputCause>,
    last_redraw_cause: Option<NativeRedrawCause>,
    event_seq: u64,
    redraw_seq: u64,
    render_seq: u64,
    idle_seq: u64,
    diagnostic_mode: bool,
    log_path: Option<std::path::PathBuf>,
    log_init_error: Option<String>,
}

impl NativeDiagnostics {
    fn new(diagnostic_mode: bool) -> Self {
        Self {
            stage: NativeStage::Startup,
            first_frame_rendered: false,
            last_input_cause: None,
            last_redraw_cause: None,
            event_seq: 0,
            redraw_seq: 0,
            render_seq: 0,
            idle_seq: 0,
            diagnostic_mode,
            log_path: None,
            log_init_error: None,
        }
    }

    fn set_stage(&mut self, stage: NativeStage) {
        self.stage = stage;
        if self.diagnostic_mode {
            info!(target: "nc_dash::native", stage = stage.label(), "native stage");
        }
    }

    fn set_last_input_cause(&mut self, cause: NativeInputCause) {
        self.last_input_cause = Some(cause);
        if self.diagnostic_mode {
            info!(
                target: "nc_dash::native",
                input_cause = cause.label(),
                "native input"
            );
        }
    }

    fn set_last_redraw_cause(&mut self, cause: NativeRedrawCause) {
        self.last_redraw_cause = Some(cause);
        if self.diagnostic_mode {
            info!(
                target: "nc_dash::native",
                redraw_cause = cause.label(),
                "native redraw"
            );
        }
    }

    fn next_event_seq(&mut self) -> u64 {
        self.event_seq += 1;
        self.event_seq
    }

    fn next_redraw_seq(&mut self) -> u64 {
        self.redraw_seq += 1;
        self.redraw_seq
    }

    fn next_render_seq(&mut self) -> u64 {
        self.render_seq += 1;
        self.render_seq
    }

    fn next_idle_seq(&mut self) -> u64 {
        self.idle_seq += 1;
        self.idle_seq
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeInputCause {
    MouseMove,
    MouseDownLeft,
    MouseUpLeft,
    MouseDownRight,
    MouseUpRight,
    MouseDownOther,
    MouseUpOther,
    Key,
    Idle,
}

impl NativeInputCause {
    fn label(self) -> &'static str {
        match self {
            Self::MouseMove => "mouse_move",
            Self::MouseDownLeft => "mouse_down_left",
            Self::MouseUpLeft => "mouse_up_left",
            Self::MouseDownRight => "mouse_down_right",
            Self::MouseUpRight => "mouse_up_right",
            Self::MouseDownOther => "mouse_down_other",
            Self::MouseUpOther => "mouse_up_other",
            Self::Key => "key",
            Self::Idle => "idle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeRedrawCause {
    Key,
    Mouse,
    Resize,
    Idle,
    Drag,
}

impl NativeRedrawCause {
    fn label(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Mouse => "mouse",
            Self::Resize => "resize",
            Self::Idle => "idle",
            Self::Drag => "drag",
        }
    }
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
    hover_redraw_pending_since: Option<Instant>,
    pending_redraw_cause: Option<NativeRedrawCause>,
    serialize_redraws: bool,
    render_count: u64,
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
            hover_redraw_pending_since: None,
            pending_redraw_cause: Some(NativeRedrawCause::Resize),
            serialize_redraws: false,
            render_count: 0,
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
                self.push_state_effects(&mut effects, true, NativeRedrawCause::Key);
            }
            NativeMsg::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
            }
            NativeMsg::MouseButton { button, pressed } => {
                if self.flush_pointer(false) {
                    self.push_state_effects(&mut effects, false, NativeRedrawCause::Mouse);
                }
                if let Some(mouse_button) = map_mouse_button(button) {
                    if mouse_button == MouseButton::Left {
                        self.left_mouse_down = pressed;
                    }
                    let changed = self.app.dispatch_mouse_event(MouseEvent {
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
                    self.push_state_effects(&mut effects, changed, NativeRedrawCause::Mouse);
                }
            }
            NativeMsg::QueuePointer(pointer) => {
                coalesce_pointer_move(&mut self.pending_pointer, pointer);
                if self.is_dragging_surface()
                    && next_pointer_dispatch(self.current_pointer, self.pending_pointer).is_some()
                {
                    self.drag_redraw_pending = true;
                    self.needs_redraw = true;
                    self.pending_redraw_cause = Some(NativeRedrawCause::Drag);
                } else if self.left_mouse_down
                    && next_pointer_dispatch(self.current_pointer, self.pending_pointer).is_some()
                {
                    self.push_state_effects(&mut effects, true, NativeRedrawCause::Mouse);
                }
            }
            NativeMsg::FlushPointer => {
                if self.flush_pointer(true) {
                    if self.hover_redraw_pending_since.is_none() {
                        self.push_state_effects(&mut effects, true, NativeRedrawCause::Mouse);
                    }
                }
            }
            NativeMsg::WindowResized {
                pixel_width,
                pixel_height,
            } => {
                if self.resize_to_window_pixels(pixel_width, pixel_height) {
                    self.push_state_effects(&mut effects, true, NativeRedrawCause::Resize);
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
        let changed = self.app.dispatch_mouse_event(MouseEvent {
            kind,
            column,
            row,
            modifiers: key_modifiers(self.modifiers),
        });
        if request_redraw && changed {
            self.needs_redraw = true;
            self.pending_redraw_cause = Some(NativeRedrawCause::Mouse);
            if matches!(kind, MouseEventKind::Moved) {
                self.hover_redraw_pending_since.get_or_insert_with(Instant::now);
            }
        }
        changed
    }

    fn pointer_column(&self) -> u16 {
        pointer_coords(self.current_pointer).0
    }

    fn pointer_row(&self) -> u16 {
        pointer_coords(self.current_pointer).1
    }

    fn push_state_effects(
        &mut self,
        effects: &mut Vec<NativeEffect>,
        redraw: bool,
        cause: NativeRedrawCause,
    ) {
        if redraw {
            self.needs_redraw = true;
            self.pending_redraw_cause = Some(cause);
            effects.push(NativeEffect::RequestRedraw(cause));
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

    fn hover_redraw_deadline(&self) -> Option<Instant> {
        self.hover_redraw_pending_since
            .map(|started| started + HOVER_REDRAW_INTERVAL)
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
        } else if let Some(deadline) = self.hover_redraw_deadline() {
            if deadline <= now {
                RedrawSchedule::Immediate
            } else {
                RedrawSchedule::Deferred(deadline)
            }
        } else {
            RedrawSchedule::Immediate
        }
    }

    fn note_rendered_frame(&mut self, now: Instant) {
        if self.drag_redraw_pending {
            self.drag_redraw_pending = false;
            self.last_drag_redraw_at = Some(now);
        }
        self.hover_redraw_pending_since = None;
    }
}

fn capture_signature<T: NativeApp>(app: &T) -> String {
    app.debug_render_signature()
        .unwrap_or_else(|| "<no-signature>".to_string())
}

struct NativeEventHandler<T: NativeApp> {
    native_options: NativeLaunchOptions,
    session_backend: &'static str,
    window_attrs: WindowAttributes,
    diagnostics: Rc<RefCell<NativeDiagnostics>>,
    window: Option<Arc<Window>>,
    renderer: Option<CellGridWindowRenderer>,
    shell: Option<NativeShell<T>>,
    app_factory: Option<T>,
}

impl<T: NativeApp> ApplicationHandler for NativeEventHandler<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let app = self
            .app_factory
            .take()
            .expect("app_factory consumed in resumed");
        self.diagnostics
            .borrow_mut()
            .set_stage(NativeStage::WindowCreate);
        let window = match event_loop.create_window(self.window_attrs.clone()) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                crate::show_fatal_error(&native_error(
                    "unable to create nc-dash window",
                    self.native_options,
                    self.session_backend,
                    &self.diagnostics.borrow(),
                    &err.to_string(),
                ));
                event_loop.exit();
                return;
            }
        };
        self.diagnostics
            .borrow_mut()
            .set_stage(NativeStage::RendererInit);
        let renderer = match CellGridWindowRenderer::new(window.clone()) {
            Ok(r) => r,
            Err(err) => {
                crate::show_fatal_error(&native_error(
                    "unable to initialize nc-dash renderer",
                    self.native_options,
                    self.session_backend,
                    &self.diagnostics.borrow(),
                    &err.to_string(),
                ));
                event_loop.exit();
                return;
            }
        };
        let initial_size = window.inner_size();
        let mut shell = NativeShell::new(app, initial_size.width, initial_size.height);
        shell.serialize_redraws = self.native_options.serialize_redraws;
        if shell.serialize_redraws && self.native_options.diagnostic_mode {
            info!(
                target: "nc_dash::native",
                "serialize_redraws diagnostic mode enabled"
            );
        }
        self.diagnostics
            .borrow_mut()
            .set_stage(NativeStage::InitialResize);
        dispatch(
            &mut shell,
            window.as_ref(),
            &self.diagnostics,
            NativeMsg::WindowResized {
                pixel_width: initial_size.width,
                pixel_height: initial_size.height,
            },
            false,
        );
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.shell = Some(shell);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(shell) = self.shell.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => {
                dispatch(
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
                    NativeMsg::CloseRequested,
                    true,
                );
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                dispatch(
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
                    NativeMsg::ModifiersChanged(modifiers.state()),
                    false,
                );
            }
            WindowEvent::Resized(size) => {
                dispatch(
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
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
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
                    NativeMsg::WindowResized {
                        pixel_width: size.width,
                        pixel_height: size.height,
                    },
                    false,
                );
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.diagnostics
                    .borrow_mut()
                    .set_last_input_cause(NativeInputCause::MouseMove);
                shell.app.note_user_activity(Instant::now());
                let pointer = pointer_from_position(shell, position);
                dispatch(
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
                    NativeMsg::QueuePointer(pointer),
                    false,
                );
            }
            WindowEvent::CursorLeft { .. } => {
                self.diagnostics
                    .borrow_mut()
                    .set_last_input_cause(NativeInputCause::MouseMove);
                dispatch(
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
                    NativeMsg::QueuePointer(PendingPointer::Outside),
                    false,
                );
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.diagnostics.borrow_mut().set_last_input_cause(
                    match (button, state.is_pressed()) {
                        (WinitMouseButton::Left, true) => NativeInputCause::MouseDownLeft,
                        (WinitMouseButton::Left, false) => NativeInputCause::MouseUpLeft,
                        (WinitMouseButton::Right, true) => NativeInputCause::MouseDownRight,
                        (WinitMouseButton::Right, false) => NativeInputCause::MouseUpRight,
                        (_, true) => NativeInputCause::MouseDownOther,
                        (_, false) => NativeInputCause::MouseUpOther,
                    },
                );
                shell.app.note_user_activity(Instant::now());
                dispatch(
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
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
                self.diagnostics
                    .borrow_mut()
                    .set_last_input_cause(NativeInputCause::Key);
                shell.app.note_user_activity(Instant::now());
                dispatch(
                    shell,
                    window.as_ref(),
                    &self.diagnostics,
                    NativeMsg::KeyInput(key),
                    true,
                );
            }
            WindowEvent::RedrawRequested => {
                shell.redraw_requested = false;
                sync_window_size(shell, window.as_ref());
                let was_drag_redraw = shell.drag_redraw_pending;
                let _ = shell.flush_pointer(false);
                if shell.app.should_quit() {
                    event_loop.exit();
                    return;
                }
                let size = window.inner_size();
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };
                match shell.app.render_ui() {
                    Ok(rendered) => {
                        shell.render_count += 1;
                        let render_seq = {
                            let mut diagnostics = self.diagnostics.borrow_mut();
                            diagnostics.next_render_seq()
                        };
                        if self.native_options.diagnostic_mode {
                            let signature = capture_signature(&shell.app);
                            info!(
                                target: "nc_dash::native",
                                render_seq,
                                render_count = shell.render_count,
                                cause = shell.pending_redraw_cause.map(NativeRedrawCause::label),
                                signature = %signature,
                                "native render begin"
                            );
                        }
                        self.diagnostics
                            .borrow_mut()
                            .set_stage(NativeStage::FirstFrameRender);
                        if let Err(err) = renderer.render(&rendered, size.width, size.height) {
                            crate::show_fatal_error(&native_error(
                                "unable to render nc-dash window",
                                self.native_options,
                                self.session_backend,
                                &self.diagnostics.borrow(),
                                &err.to_string(),
                            ));
                            event_loop.exit();
                        } else {
                            if was_drag_redraw || shell.hover_redraw_pending_since.is_some() {
                                shell.note_rendered_frame(Instant::now());
                            }
                            let mut diagnostics = self.diagnostics.borrow_mut();
                            diagnostics.first_frame_rendered = true;
                            diagnostics.set_stage(NativeStage::FirstFrameRendered);
                            shell.pending_redraw_cause = None;
                            shell.needs_redraw = false;
                            if self.native_options.diagnostic_mode {
                                let signature = capture_signature(&shell.app);
                                info!(
                                    target: "nc_dash::native",
                                    render_seq,
                                    render_count = shell.render_count,
                                    signature = %signature,
                                    "native render end"
                                );
                            }
                        }
                    }
                    Err(err) => {
                        crate::show_fatal_error(&err.to_string());
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(shell) = self.shell.as_mut() else {
            return;
        };
        if !self.diagnostics.borrow().first_frame_rendered {
            self.diagnostics
                .borrow_mut()
                .set_stage(NativeStage::EventLoopWait);
        }
        sync_window_size(shell, window.as_ref());
        if !shell.redraw_requested
            && !shell.is_dragging_surface()
            && shell.pending_pointer.is_some()
        {
            dispatch(
                shell,
                window.as_ref(),
                &self.diagnostics,
                NativeMsg::FlushPointer,
                false,
            );
        }
        let skip_idle =
            shell.serialize_redraws && (shell.needs_redraw || shell.redraw_requested);
        if skip_idle && self.native_options.diagnostic_mode {
            info!(
                target: "nc_dash::native",
                needs_redraw = shell.needs_redraw,
                redraw_requested = shell.redraw_requested,
                "native idle skipped by serialize_redraws"
            );
        } else {
            let before_signature = if self.native_options.diagnostic_mode {
                Some(capture_signature(&shell.app))
            } else {
                None
            };
            let idle_changed = shell.app.on_idle();
            if idle_changed {
                let idle_seq = {
                    let mut diagnostics = self.diagnostics.borrow_mut();
                    diagnostics.set_last_input_cause(NativeInputCause::Idle);
                    diagnostics.next_idle_seq()
                };
                shell.needs_redraw = true;
                shell.pending_redraw_cause = Some(NativeRedrawCause::Idle);
                if self.native_options.diagnostic_mode {
                    let after_signature = capture_signature(&shell.app);
                    info!(
                        target: "nc_dash::native",
                        idle_seq,
                        before = before_signature.as_deref().unwrap_or("<no-signature>"),
                        after = %after_signature,
                        "native idle changed state"
                    );
                }
            } else if self.native_options.diagnostic_mode {
                let idle_seq = self.diagnostics.borrow_mut().next_idle_seq();
                let after_signature = capture_signature(&shell.app);
                info!(
                    target: "nc_dash::native",
                    idle_seq,
                    before = before_signature.as_deref().unwrap_or("<no-signature>"),
                    after = %after_signature,
                    "native idle no-op"
                );
            }
        }
        if shell.app.should_quit() {
            event_loop.exit();
        } else {
            let now = Instant::now();
            match shell.next_redraw_schedule(now) {
                RedrawSchedule::Immediate => {
                    if !self.diagnostics.borrow().first_frame_rendered {
                        self.diagnostics
                            .borrow_mut()
                            .set_stage(NativeStage::FirstRedrawRequested);
                    }
                    if let Some(cause) = shell.pending_redraw_cause {
                        self.diagnostics.borrow_mut().set_last_redraw_cause(cause);
                    }
                    if self.native_options.diagnostic_mode {
                        let redraw_seq = self.diagnostics.borrow_mut().next_redraw_seq();
                        info!(
                            target: "nc_dash::native",
                            redraw_seq,
                            source = "about_to_wait_immediate",
                            cause = shell.pending_redraw_cause.map(NativeRedrawCause::label),
                            "native redraw requested"
                        );
                    }
                    window.request_redraw();
                    shell.redraw_requested = true;
                }
                RedrawSchedule::Deferred(deadline) => {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        combine_deadlines(Some(deadline), shell.app.next_wakeup())
                            .expect("deferred redraw has a deadline"),
                    ));
                }
                RedrawSchedule::None => {
                    if let Some(deadline) = shell.app.next_wakeup() {
                        if deadline <= now {
                            if !self.diagnostics.borrow().first_frame_rendered {
                                self.diagnostics
                                    .borrow_mut()
                                    .set_stage(NativeStage::FirstRedrawRequested);
                            }
                            if let Some(cause) = shell.pending_redraw_cause {
                                self.diagnostics.borrow_mut().set_last_redraw_cause(cause);
                            }
                            if self.native_options.diagnostic_mode {
                                let redraw_seq =
                                    self.diagnostics.borrow_mut().next_redraw_seq();
                                info!(
                                    target: "nc_dash::native",
                                    redraw_seq,
                                    source = "about_to_wait_wakeup",
                                    cause = shell
                                        .pending_redraw_cause
                                        .map(NativeRedrawCause::label),
                                    "native redraw requested"
                                );
                            }
                            window.request_redraw();
                            shell.redraw_requested = true;
                        } else {
                            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                        }
                    }
                }
            }
        }
    }
}

pub fn run<T: NativeApp>(
    app: T,
    native_options: NativeLaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_backend = detect_session_backend();
    let diagnostics = Rc::new(RefCell::new(NativeDiagnostics::new(
        native_options.diagnostic_mode,
    )));
    if native_options.diagnostic_mode {
        let log_path = native_log_path();
        match nc_log::init_file_logging(&log_path, LogLevel::Trace) {
            Ok(()) => {
                diagnostics.borrow_mut().log_path = Some(log_path.clone());
                info!(
                    target: "nc_dash::native",
                    log_path = %log_path.display(),
                    "diagnostic logging enabled"
                );
            }
            Err(err) => {
                diagnostics.borrow_mut().log_init_error = Some(err.to_string());
            }
        }
    }

    diagnostics
        .borrow_mut()
        .set_stage(NativeStage::EventLoopBuild);
    let mut event_loop_builder = EventLoop::builder();
    apply_backend_preference(&mut event_loop_builder, native_options.backend_preference)?;
    let event_loop = event_loop_builder.build().map_err(|err| {
        native_error(
            "unable to create nc-dash event loop",
            native_options,
            session_backend,
            &diagnostics.borrow(),
            &err.to_string(),
        )
    })?;

    let geometry = app.geometry();
    let mut window_attrs = Window::default_attributes()
        .with_title(app.window_title())
        .with_inner_size(logical_window_size_for_grid(
            geometry.width(),
            geometry.height(),
        ))
        .with_decorations(session_backend != "wayland")
        .with_resizable(true);
    window_attrs = match native_options.window_mode {
        NativeWindowMode::MaximizedWindow => window_attrs.with_maximized(true),
        NativeWindowMode::BorderlessFullscreen => {
            window_attrs.with_fullscreen(Some(Fullscreen::Borderless(None)))
        }
    };

    let mut handler = NativeEventHandler {
        native_options,
        session_backend,
        window_attrs,
        diagnostics: diagnostics.clone(),
        window: None,
        renderer: None,
        shell: None,
        app_factory: Some(app),
    };
    event_loop.run_app(&mut handler).map_err(|err| {
        map_event_loop_error(err, native_options, session_backend, &diagnostics.borrow())
    })?;

    Ok(())
}

fn detect_session_backend() -> &'static str {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        "wayland"
    } else if std::env::var_os("DISPLAY").is_some() {
        "x11"
    } else if let Some(session_type) = std::env::var_os("XDG_SESSION_TYPE") {
        if session_type == "wayland" {
            "wayland"
        } else if session_type == "x11" {
            "x11"
        } else {
            "unknown"
        }
    } else {
        "unknown"
    }
}

fn native_log_path() -> std::path::PathBuf {
    nc_client::paths::data_root().join("nc-dash.log")
}

fn apply_backend_preference(
    builder: &mut EventLoopBuilder<()>,
    backend_preference: NativeBackendPreference,
) -> Result<(), Box<dyn std::error::Error>> {
    match backend_preference {
        NativeBackendPreference::Auto => Ok(()),
        NativeBackendPreference::Wayland => apply_wayland_backend(builder),
        NativeBackendPreference::X11 => apply_x11_backend(builder),
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn apply_wayland_backend(
    builder: &mut EventLoopBuilder<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    builder.with_wayland();
    Ok(())
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn apply_wayland_backend(
    _builder: &mut EventLoopBuilder<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("the wayland backend override is not supported on this platform".into())
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn apply_x11_backend(builder: &mut EventLoopBuilder<()>) -> Result<(), Box<dyn std::error::Error>> {
    builder.with_x11();
    Ok(())
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn apply_x11_backend(
    _builder: &mut EventLoopBuilder<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("the x11 backend override is not supported on this platform".into())
}

fn native_error(
    prefix: &str,
    native_options: NativeLaunchOptions,
    session_backend: &str,
    diagnostics: &NativeDiagnostics,
    err: &str,
) -> String {
    let mut message = format!(
        "{prefix} (mode: {}, backend: {}, session: {}, renderer: ratatui-wgpu, stage: {}, first_frame: {}, serialize_redraws: {}, last_input: {}, last_redraw: {}): {err}",
        native_options.window_mode.cli_label(),
        native_options.backend_preference.cli_label(),
        session_backend,
        diagnostics.stage.label(),
        if diagnostics.first_frame_rendered {
            "yes"
        } else {
            "no"
        },
        if native_options.serialize_redraws {
            "yes"
        } else {
            "no"
        },
        diagnostics
            .last_input_cause
            .map(NativeInputCause::label)
            .unwrap_or("unknown"),
        diagnostics
            .last_redraw_cause
            .map(NativeRedrawCause::label)
            .unwrap_or("unknown")
    );
    if let Some(log_path) = diagnostics.log_path.as_ref() {
        message.push_str(&format!(" — log: {}", log_path.display()));
    }
    if let Some(log_init_error) = diagnostics.log_init_error.as_ref() {
        message.push_str(&format!(" — diagnostic log unavailable: {log_init_error}"));
    }
    if native_options.window_mode == NativeWindowMode::BorderlessFullscreen
        && session_backend == "wayland"
    {
        message.push_str(
            " — retry without --fullscreen or launch windowed and let your compositor make it fullscreen",
        );
    }
    message
}

fn map_event_loop_error(
    err: EventLoopError,
    native_options: NativeLaunchOptions,
    session_backend: &str,
    diagnostics: &NativeDiagnostics,
) -> Box<dyn std::error::Error> {
    let err_text = match err {
        EventLoopError::ExitFailure(1) if session_backend == "wayland" => format!(
            "native event loop disconnected before a clean exit; likely Wayland compositor/protocol disconnect inside winit while flushing or dispatching events{}",
            if diagnostics.first_frame_rendered {
                ""
            } else {
                " before the first successful frame"
            }
        ),
        EventLoopError::ExitFailure(code) => {
            format!("native event loop exited with failure code {code}")
        }
        other => other.to_string(),
    };
    native_error(
        "nc-dash native event loop failed",
        native_options,
        session_backend,
        diagnostics,
        &err_text,
    )
    .into()
}

fn dispatch<T: NativeApp>(
    shell: &mut NativeShell<T>,
    window: &winit::window::Window,
    diagnostics: &Rc<RefCell<NativeDiagnostics>>,
    msg: NativeMsg,
    exit_immediately: bool,
) {
    let before_signature = if diagnostics.borrow().diagnostic_mode {
        Some(capture_signature(&shell.app))
    } else {
        None
    };
    let event_seq = {
        let mut diagnostics = diagnostics.borrow_mut();
        let seq = diagnostics.next_event_seq();
        if diagnostics.diagnostic_mode {
            info!(
                target: "nc_dash::native",
                event_seq = seq,
                msg = msg.label(),
                render_count = shell.render_count,
                needs_redraw = shell.needs_redraw,
                redraw_requested = shell.redraw_requested,
                pending_redraw = shell.pending_redraw_cause.map(NativeRedrawCause::label),
                before = before_signature.as_deref().unwrap_or("<no-signature>"),
                "native dispatch begin"
            );
        }
        seq
    };
    let effects = shell.update(msg);
    if diagnostics.borrow().diagnostic_mode {
        let after_signature = capture_signature(&shell.app);
        info!(
            target: "nc_dash::native",
            event_seq,
            msg = msg.label(),
            effects = ?effects,
            should_quit = shell.app.should_quit(),
            needs_redraw = shell.needs_redraw,
            redraw_requested = shell.redraw_requested,
            pending_redraw = shell.pending_redraw_cause.map(NativeRedrawCause::label),
            after = %after_signature,
            "native dispatch end"
        );
    }
    apply_effects(shell, window, diagnostics, effects, exit_immediately);
}

fn apply_effects<T: NativeApp>(
    shell: &mut NativeShell<T>,
    window: &winit::window::Window,
    diagnostics: &Rc<RefCell<NativeDiagnostics>>,
    effects: Vec<NativeEffect>,
    exit_immediately: bool,
) {
    for effect in effects {
        match effect {
            NativeEffect::RequestRedraw(cause) => {
                shell.needs_redraw = true;
                shell.pending_redraw_cause = Some(cause);
            }
            NativeEffect::Exit if exit_immediately => shell.app.set_should_quit(true),
            NativeEffect::Exit => {}
        }
    }
    if shell.needs_redraw && !shell.redraw_requested {
        if let Some(cause) = shell.pending_redraw_cause {
            diagnostics.borrow_mut().set_last_redraw_cause(cause);
        }
        if !shell.serialize_redraws {
            let redraw_seq = {
                let mut diagnostics = diagnostics.borrow_mut();
                let seq = diagnostics.next_redraw_seq();
                if diagnostics.diagnostic_mode {
                    info!(
                        target: "nc_dash::native",
                        redraw_seq = seq,
                        source = "apply_effects",
                        cause = shell.pending_redraw_cause.map(NativeRedrawCause::label),
                        "native redraw requested"
                    );
                }
                seq
            };
            let _ = redraw_seq;
            window.request_redraw();
            shell.redraw_requested = true;
        } else if diagnostics.borrow().diagnostic_mode {
            info!(
                target: "nc_dash::native",
                source = "apply_effects",
                cause = shell.pending_redraw_cause.map(NativeRedrawCause::label),
                "native redraw deferred by serialize_redraws"
            );
        }
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
        DRAG_REDRAW_INTERVAL, HOVER_REDRAW_INTERVAL, NativeApp, NativeDiagnostics, NativeEffect,
        NativeInputCause, NativeMsg, NativeRedrawCause, NativeShell, NativeStage,
        PendingPointer, RedrawSchedule, WinitMouseButton, coalesce_pointer_move,
        map_event_loop_error, native_error, next_pointer_dispatch, pointer_coords,
        pointer_event_kind,
    };
    use crate::RenderedUi;
    use crate::geometry::ScreenGeometry;
    use crate::startup::{NativeBackendPreference, NativeLaunchOptions, NativeWindowMode};
    use crossterm::event::{MouseEvent, MouseEventKind};
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use winit::error::EventLoopError;

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

        assert_eq!(effects, vec![NativeEffect::RequestRedraw(NativeRedrawCause::Mouse)]);
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

        let _ = shell.flush_pointer(false);
        assert_eq!(shell.current_pointer, Some(PendingPointer::Cell(8, 2)));
        assert_eq!(shell.pending_pointer, None);
    }

    #[test]
    fn passive_pointer_move_without_state_change_does_not_request_redraw() {
        let mut shell = test_mouse_shell(false);
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));
        shell.pending_pointer = Some(PendingPointer::Cell(4, 1));
        shell.needs_redraw = false;

        assert!(!shell.flush_pointer(true));
        assert!(!shell.needs_redraw);
        assert_eq!(shell.current_pointer, Some(PendingPointer::Cell(4, 1)));
    }

    #[test]
    fn passive_pointer_move_with_state_change_requests_redraw() {
        let mut shell = test_mouse_shell(true);
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));
        shell.pending_pointer = Some(PendingPointer::Cell(4, 1));

        assert!(shell.flush_pointer(true));
        assert!(shell.needs_redraw);
        assert_eq!(shell.current_pointer, Some(PendingPointer::Cell(4, 1)));
    }

    #[test]
    fn passive_pointer_move_redraw_is_deferred_for_hover_throttle() {
        let mut shell = test_mouse_shell(true);
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));
        shell.pending_pointer = Some(PendingPointer::Cell(4, 1));
        shell.needs_redraw = false;

        assert!(shell.flush_pointer(true));
        assert!(matches!(
            shell.next_redraw_schedule(Instant::now()),
            RedrawSchedule::Deferred(_)
        ));
        let deadline = shell
            .hover_redraw_deadline()
            .expect("hover redraw deadline");
        assert_eq!(
            shell.next_redraw_schedule(deadline + HOVER_REDRAW_INTERVAL),
            RedrawSchedule::Immediate
        );
    }

    #[test]
    fn passive_pointer_move_dispatches_and_defers_redraw() {
        let mut shell = test_mouse_shell(true);
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));
        shell.pending_pointer = Some(PendingPointer::Cell(4, 1));
        shell.needs_redraw = false;

        assert!(shell.flush_pointer(true));
        assert!(shell.needs_redraw);
        assert!(shell.hover_redraw_pending_since.is_some());
        assert_eq!(shell.app.mouse_dispatch_count, 1);
        assert_eq!(shell.current_pointer, Some(PendingPointer::Cell(4, 1)));
    }

    #[test]
    fn queued_passive_pointer_move_waits_for_flush_before_dispatch() {
        let mut shell = test_mouse_shell(true);
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));

        let effects = shell.update(NativeMsg::QueuePointer(PendingPointer::Cell(8, 2)));

        assert!(effects.is_empty());
        assert_eq!(shell.current_pointer, Some(PendingPointer::Cell(3, 1)));
        assert_eq!(shell.pending_pointer, Some(PendingPointer::Cell(8, 2)));
        assert_eq!(shell.app.mouse_dispatch_count, 0);
    }

    #[test]
    fn flushed_passive_pointer_move_keeps_updated_pointer_for_subsequent_click_dispatch() {
        let mut shell = test_mouse_shell(true);
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));
        shell.pending_pointer = Some(PendingPointer::Cell(8, 2));

        assert!(shell.flush_pointer(true));
        assert_eq!(shell.app.mouse_dispatch_count, 1);

        let effects = shell.update(NativeMsg::MouseButton {
            button: WinitMouseButton::Left,
            pressed: true,
        });

        assert_eq!(shell.app.mouse_dispatch_count, 2);
        assert_eq!(shell.app.last_mouse_column, Some(8));
        assert_eq!(shell.app.last_mouse_row, Some(2));
        assert_eq!(effects, vec![NativeEffect::RequestRedraw(NativeRedrawCause::Mouse)]);
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

    #[test]
    fn native_error_includes_backend_and_stage_context() {
        let mut diagnostics = NativeDiagnostics::new(true);
        diagnostics.set_stage(NativeStage::RendererInit);
        diagnostics.set_last_input_cause(NativeInputCause::MouseMove);
        diagnostics.set_last_redraw_cause(NativeRedrawCause::Mouse);
        diagnostics.log_path = Some(PathBuf::from("/tmp/nc-dash.log"));
        let message = native_error(
            "unable to initialize nc-dash renderer",
            NativeLaunchOptions {
                window_mode: NativeWindowMode::MaximizedWindow,
                backend_preference: NativeBackendPreference::Wayland,
                diagnostic_mode: true,
                ..NativeLaunchOptions::default()
            },
            "wayland",
            &diagnostics,
            "boom",
        );

        assert!(message.contains("backend: wayland"));
        assert!(message.contains("stage: renderer_init"));
        assert!(message.contains("renderer: ratatui-wgpu"));
        assert!(message.contains("last_input: mouse_move"));
        assert!(message.contains("last_redraw: mouse"));
        assert!(message.contains("/tmp/nc-dash.log"));
    }

    #[test]
    fn wayland_exit_failure_is_mapped_to_disconnect_message() {
        let diagnostics = NativeDiagnostics::new(false);
        let err = map_event_loop_error(
            EventLoopError::ExitFailure(1),
            NativeLaunchOptions::default(),
            "wayland",
            &diagnostics,
        );

        assert!(
            err.to_string()
                .contains("likely Wayland compositor/protocol disconnect")
        );
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
                mouse_state_changed: false,
                mouse_dispatch_count: 0,
                last_mouse_column: None,
                last_mouse_row: None,
                should_quit: false,
            },
            window_pixel_width,
            window_pixel_height,
        )
    }

    fn test_mouse_shell(mouse_state_changed: bool) -> NativeShell<TestApp> {
        NativeShell::new(
            TestApp {
                geometry: ScreenGeometry::new(10, 3),
                dragging_surface: false,
                mouse_state_changed,
                mouse_dispatch_count: 0,
                last_mouse_column: None,
                last_mouse_row: None,
                should_quit: false,
            },
            100,
            54,
        )
    }

    struct TestApp {
        geometry: ScreenGeometry,
        dragging_surface: bool,
        mouse_state_changed: bool,
        mouse_dispatch_count: usize,
        last_mouse_column: Option<u16>,
        last_mouse_row: Option<u16>,
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

        fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
            self.mouse_dispatch_count += 1;
            self.last_mouse_column = Some(mouse.column);
            self.last_mouse_row = Some(mouse.row);
            self.mouse_state_changed
        }

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
