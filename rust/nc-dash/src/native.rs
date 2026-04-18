use crate::geometry::ScreenGeometry;
use crate::input::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind, key_event_from_winit,
    key_modifiers_from_winit,
};
use crate::lobby::storage::settings::PersistedWindowState;
use crate::native_grid::{
    CellGridWindowRenderer, cell_position_at_pixel, logical_window_size_for_grid,
    terminal_grid_for_pixels,
};
use crate::startup::{NativeBackendPreference, NativeLaunchOptions, NativeWindowMode};
use crate::ui::UiScene;
use nc_log::LogLevel;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::dpi::PhysicalPosition;
use winit::error::EventLoopError;
use winit::event::{MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, Icon, Window, WindowAttributes, WindowId};

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::startup_notify::{
    EventLoopExtStartupNotify, WindowAttributesExtStartupNotify, reset_activation_token_env,
};
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::wayland::EventLoopBuilderExtWayland;
#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;
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
    fn native_window_ready(&mut self, _window: &Window) {}
    fn wants_window_focus(&self) -> bool {
        false
    }
    fn wants_text_input(&self) -> bool {
        false
    }
    fn saved_window_state(&self) -> Option<PersistedWindowState> {
        None
    }
    fn persist_window_state(&mut self, _state: PersistedWindowState) -> Result<(), String> {
        Ok(())
    }
    fn dispatch_key_event(&mut self, key: crate::input::KeyEvent);
    fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool;
    fn resize_canvas(&mut self, cols: u16, rows: u16);
    fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>>;
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
const NC_ICON_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/nc.ico"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResolvedWindowPolicy {
    inner_width: u16,
    inner_height: u16,
    maximized: bool,
    fullscreen: bool,
    decorations: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PendingPointer {
    Outside,
    Cell(u16, u16),
}

#[derive(Clone, Copy, Debug)]
enum NativeMsg {
    CloseRequested,
    KeyInput(crate::input::KeyEvent),
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
        scale_factor: f64,
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
    last_signature: Option<String>,
    activation_token_present: Option<bool>,
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
            last_signature: None,
            activation_token_present: None,
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

    fn set_last_signature(&mut self, signature: String) {
        self.last_signature = Some(signature);
    }

    fn set_activation_token_present(&mut self, present: bool) {
        self.activation_token_present = Some(present);
        if self.diagnostic_mode {
            info!(
                target: "nc_dash::native",
                present,
                "native activation token"
            );
        }
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
    window_scale_factor: f64,
    persist_window_state: bool,
    last_persisted_window_state: Option<PersistedWindowState>,
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
    window_has_focus: Option<bool>,
    text_input_enabled: Option<bool>,
    programmatic_focus_supported: bool,
}

impl<T: NativeApp> NativeShell<T> {
    fn new(
        app: T,
        window_pixel_width: u32,
        window_pixel_height: u32,
        window_scale_factor: f64,
        persist_window_state: bool,
        programmatic_focus_supported: bool,
    ) -> Self {
        let last_persisted_window_state = app.saved_window_state();
        Self {
            app,
            window_pixel_width: window_pixel_width.max(1),
            window_pixel_height: window_pixel_height.max(1),
            window_scale_factor,
            persist_window_state,
            last_persisted_window_state,
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
            window_has_focus: Some(false),
            text_input_enabled: None,
            programmatic_focus_supported,
        }
    }

    fn sync_persisted_window_state(
        &mut self,
        window: &winit::window::Window,
    ) -> Result<(), String> {
        if !self.persist_window_state || window.fullscreen().is_some() {
            return Ok(());
        }
        let size = window.inner_size().to_logical::<f64>(window.scale_factor());
        let state = PersistedWindowState {
            width: logical_dimension_to_u16(size.width),
            height: logical_dimension_to_u16(size.height),
            maximized: window.is_maximized(),
        };
        if self.last_persisted_window_state == Some(state) {
            return Ok(());
        }
        self.app.persist_window_state(state)?;
        self.last_persisted_window_state = Some(state);
        Ok(())
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
                scale_factor,
            } => {
                if self.resize_to_window_pixels(pixel_width, pixel_height, scale_factor) {
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
                self.hover_redraw_pending_since
                    .get_or_insert_with(Instant::now);
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

    fn resize_to_window_pixels(
        &mut self,
        pixel_width: u32,
        pixel_height: u32,
        scale_factor: f64,
    ) -> bool {
        let pixel_width = pixel_width.max(1);
        let pixel_height = pixel_height.max(1);
        let (cols, rows) = terminal_grid_for_pixels(pixel_width, pixel_height, scale_factor);
        let geometry = self.app.geometry();
        let geometry_changed =
            geometry.width() != cols as usize || geometry.height() != rows as usize;
        if self.window_pixel_width == pixel_width
            && self.window_pixel_height == pixel_height
            && (self.window_scale_factor - scale_factor).abs() < f64::EPSILON
            && !geometry_changed
        {
            return false;
        }

        self.window_pixel_width = pixel_width;
        self.window_pixel_height = pixel_height;
        self.window_scale_factor = scale_factor;
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

    fn clear_focus_state(&mut self) {
        self.modifiers = ModifiersState::empty();
        self.window_has_focus = Some(false);
        self.text_input_enabled = None;
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
        let mut app = self
            .app_factory
            .take()
            .expect("app_factory consumed in resumed");
        self.diagnostics
            .borrow_mut()
            .set_stage(NativeStage::WindowCreate);
        let window_attrs = apply_startup_activation_token(
            event_loop,
            self.window_attrs.clone(),
            &self.diagnostics,
        );
        let window = match event_loop.create_window(window_attrs) {
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
        app.native_window_ready(window.as_ref());
        self.diagnostics
            .borrow_mut()
            .set_stage(NativeStage::RendererInit);
        let renderer = match CellGridWindowRenderer::new(window.clone(), event_loop) {
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
        let mut shell = NativeShell::new(
            app,
            initial_size.width,
            initial_size.height,
            window.scale_factor(),
            self.native_options.window_mode != NativeWindowMode::BorderlessFullscreen,
            backend_supports_programmatic_focus(self.session_backend),
        );
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
                scale_factor: window.scale_factor(),
            },
            false,
        );
        sync_window_focus_state(&mut shell, window.as_ref());
        sync_window_input_state(&mut shell, window.as_ref());
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
            WindowEvent::Focused(focused) => {
                if self.native_options.diagnostic_mode {
                    info!(
                        target: "nc_dash::native",
                        focused,
                        "window focus event"
                    );
                }
                if focused {
                    shell.window_has_focus = Some(true);
                    sync_window_input_state(shell, window.as_ref());
                } else {
                    shell.clear_focus_state();
                }
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
                        scale_factor: window.scale_factor(),
                    },
                    false,
                );
                sync_window_state(shell, window.as_ref());
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
                        scale_factor: window.scale_factor(),
                    },
                    false,
                );
                sync_window_state(shell, window.as_ref());
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.native_options.diagnostic_mode {
                    info!(
                        target: "nc_dash::native",
                        x = position.x,
                        y = position.y,
                        "cursor moved"
                    );
                }
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
                if self.native_options.diagnostic_mode {
                    info!(target: "nc_dash::native", "cursor left");
                }
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
                if self.native_options.diagnostic_mode {
                    info!(
                        target: "nc_dash::native",
                        button = ?button,
                        pressed = state.is_pressed(),
                        focused = shell.window_has_focus,
                        "mouse input"
                    );
                }
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
                let key = key_event_from_winit(&event, shell.modifiers);
                if self.native_options.diagnostic_mode {
                    info!(
                        target: "nc_dash::native",
                        state = ?event.state,
                        text = event.text.as_deref().unwrap_or_default(),
                        logical_key = ?event.logical_key,
                        converted = key.is_some(),
                        focused = shell.window_has_focus,
                        "keyboard input"
                    );
                }
                let Some(key) = key else {
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
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                if self.native_options.diagnostic_mode {
                    info!(
                        target: "nc_dash::native",
                        text = %text,
                        focused = shell.window_has_focus,
                        "ime commit"
                    );
                }
                for ch in text.chars() {
                    let key =
                        KeyEvent::new(KeyCode::Char(ch), key_modifiers_from_winit(shell.modifiers));
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
            }
            WindowEvent::RedrawRequested => {
                shell.redraw_requested = false;
                sync_window_size(shell, window.as_ref());
                let was_drag_redraw = shell.drag_redraw_pending;
                let pointer_flushed = shell.flush_pointer(false);
                if shell.app.should_quit() {
                    sync_window_state(shell, window.as_ref());
                    event_loop.exit();
                    return;
                }
                let size = window.inner_size();
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };
                match shell.app.render_scene() {
                    Ok(scene) => {
                        shell.render_count += 1;
                        let render_seq = {
                            let mut diagnostics = self.diagnostics.borrow_mut();
                            diagnostics.next_render_seq()
                        };
                        if self.native_options.diagnostic_mode {
                            let signature = capture_signature(&shell.app);
                            self.diagnostics
                                .borrow_mut()
                                .set_last_signature(signature.clone());
                            info!(
                                target: "nc_dash::native",
                                render_seq,
                                render_count = shell.render_count,
                                cause = shell.pending_redraw_cause.map(NativeRedrawCause::label),
                                pointer_flushed,
                                window_width = size.width,
                                window_height = size.height,
                                signature = %signature,
                                "native render begin"
                            );
                        }
                        self.diagnostics
                            .borrow_mut()
                            .set_stage(NativeStage::FirstFrameRender);
                        match renderer.render(
                            &scene,
                            size.width,
                            size.height,
                            self.native_options.diagnostic_mode,
                        ) {
                            Err(err) => {
                                crate::show_fatal_error(&native_error(
                                    "unable to render nc-dash window",
                                    self.native_options,
                                    self.session_backend,
                                    &self.diagnostics.borrow(),
                                    &err.to_string(),
                                ));
                                event_loop.exit();
                            }
                            Ok(frame_stats) => {
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
                                    diagnostics.set_last_signature(signature.clone());
                                    info!(
                                        target: "nc_dash::native",
                                        render_seq,
                                        render_count = shell.render_count,
                                        pointer_flushed,
                                        window_width = frame_stats.window_width,
                                        window_height = frame_stats.window_height,
                                        grid_cols = frame_stats.grid_cols,
                                        grid_rows = frame_stats.grid_rows,
                                        text_areas = frame_stats.text_area_count,
                                        unique_text_buffers = frame_stats.unique_text_buffer_count,
                                        signature = %signature,
                                        "native render end"
                                    );
                                }
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
        sync_window_state(shell, window.as_ref());
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
        let skip_idle = shell.serialize_redraws && (shell.needs_redraw || shell.redraw_requested);
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
                    self.diagnostics
                        .borrow_mut()
                        .set_last_signature(after_signature.clone());
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
                self.diagnostics
                    .borrow_mut()
                    .set_last_signature(after_signature.clone());
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
            sync_window_state(shell, window.as_ref());
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
                    if let Some(deadline) = shell.app.next_wakeup() {
                        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                    }
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
                                let redraw_seq = self.diagnostics.borrow_mut().next_redraw_seq();
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
                        }
                        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                    }
                }
            }
        }
    }
}

fn resolve_window_policy(
    window_mode: NativeWindowMode,
    geometry: ScreenGeometry,
    saved_window_state: Option<PersistedWindowState>,
) -> ResolvedWindowPolicy {
    let default_size = logical_window_size_for_grid(geometry.width(), geometry.height());
    let inner_width = saved_window_state
        .map(|state| state.width)
        .unwrap_or_else(|| logical_dimension_to_u16(default_size.width));
    let inner_height = saved_window_state
        .map(|state| state.height)
        .unwrap_or_else(|| logical_dimension_to_u16(default_size.height));
    match window_mode {
        NativeWindowMode::MaximizedWindow => ResolvedWindowPolicy {
            inner_width,
            inner_height,
            maximized: saved_window_state
                .map(|state| state.maximized)
                .unwrap_or(true),
            fullscreen: false,
            decorations: true,
        },
        NativeWindowMode::BorderlessFullscreen => ResolvedWindowPolicy {
            inner_width,
            inner_height,
            maximized: false,
            fullscreen: true,
            decorations: false,
        },
    }
}

fn logical_dimension_to_u16(value: f64) -> u16 {
    if !value.is_finite() {
        return 1;
    }
    value.round().clamp(1.0, f64::from(u16::MAX)) as u16
}

fn native_window_icon() -> Option<Icon> {
    let icon = image::load_from_memory_with_format(NC_ICON_BYTES, image::ImageFormat::Ico).ok()?;
    let rgba = icon.into_rgba8();
    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).ok()
}

fn backend_supports_programmatic_focus(session_backend: &str) -> bool {
    session_backend != "wayland"
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
fn apply_startup_activation_token(
    event_loop: &ActiveEventLoop,
    mut window_attrs: WindowAttributes,
    diagnostics: &Rc<RefCell<NativeDiagnostics>>,
) -> WindowAttributes {
    let token = event_loop.read_token_from_env();
    reset_activation_token_env();
    diagnostics
        .borrow_mut()
        .set_activation_token_present(token.is_some());
    if let Some(token) = token {
        window_attrs = window_attrs.with_activation_token(token);
    }
    window_attrs
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
)))]
fn apply_startup_activation_token(
    _event_loop: &ActiveEventLoop,
    window_attrs: WindowAttributes,
    diagnostics: &Rc<RefCell<NativeDiagnostics>>,
) -> WindowAttributes {
    diagnostics.borrow_mut().set_activation_token_present(false);
    window_attrs
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
    if native_options.diagnostic_mode {
        info!(
            target: "nc_dash::native",
            session_backend,
            backend = native_options.backend_preference.cli_label(),
            "native startup"
        );
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
    let window_policy = resolve_window_policy(
        native_options.window_mode,
        geometry,
        app.saved_window_state(),
    );
    let mut window_attrs = Window::default_attributes()
        .with_title(app.window_title())
        .with_inner_size(LogicalSize::new(
            f64::from(window_policy.inner_width),
            f64::from(window_policy.inner_height),
        ))
        .with_decorations(window_policy.decorations)
        .with_resizable(true)
        .with_window_icon(native_window_icon());
    #[cfg(target_os = "windows")]
    {
        window_attrs = window_attrs.with_taskbar_icon(native_window_icon());
    }
    window_attrs = if window_policy.fullscreen {
        window_attrs.with_fullscreen(Some(Fullscreen::Borderless(None)))
    } else {
        window_attrs.with_maximized(window_policy.maximized)
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
        "{prefix} (mode: {}, backend: {}, session: {}, renderer: glyphon-wgpu, stage: {}, first_frame: {}, serialize_redraws: {}, last_input: {}, last_redraw: {}): {err}",
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
    if let Some(present) = diagnostics.activation_token_present {
        message.push_str(&format!(
            ", activation_token: {}",
            if present { "yes" } else { "no" }
        ));
    }
    if let Some(signature) = diagnostics.last_signature.as_deref() {
        message.push_str(&format!("; signature: {signature}"));
    }
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
    let sync_focus = matches!(msg, NativeMsg::KeyInput(_) | NativeMsg::MouseButton { .. });
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
    if sync_focus {
        sync_window_focus_state(shell, window);
    }
    sync_window_input_state(shell, window);
    if diagnostics.borrow().diagnostic_mode {
        let after_signature = capture_signature(&shell.app);
        diagnostics
            .borrow_mut()
            .set_last_signature(after_signature.clone());
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
    if shell.resize_to_window_pixels(size.width, size.height, window.scale_factor()) {
        shell.needs_redraw = true;
    }
}

fn sync_window_state<T: NativeApp>(shell: &mut NativeShell<T>, window: &winit::window::Window) {
    if let Err(err) = shell.sync_persisted_window_state(window) {
        warn!(target: "nc_dash::native", error = %err, "failed to persist native window state");
    }
}

fn sync_window_focus_requested(current: &mut Option<bool>, wants: bool) -> bool {
    if !wants || *current != Some(false) {
        return false;
    }
    *current = None;
    true
}

fn sync_window_focus_state<T: NativeApp>(
    shell: &mut NativeShell<T>,
    window: &winit::window::Window,
) {
    if !shell.programmatic_focus_supported {
        return;
    }
    if sync_window_focus_requested(&mut shell.window_has_focus, shell.app.wants_window_focus()) {
        window.focus_window();
    }
}

fn sync_text_input_enabled(current: &mut Option<bool>, wants: bool) -> bool {
    if *current == Some(wants) {
        return false;
    }
    *current = Some(wants);
    true
}

fn sync_window_input_state<T: NativeApp>(
    shell: &mut NativeShell<T>,
    window: &winit::window::Window,
) {
    let wants_text_input = shell.app.wants_text_input();
    if sync_text_input_enabled(&mut shell.text_input_enabled, wants_text_input) {
        window.set_ime_allowed(wants_text_input);
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
        shell.window_scale_factor,
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
    key_modifiers_from_winit(modifiers)
}

#[cfg(test)]
mod tests {
    use super::{
        DRAG_REDRAW_INTERVAL, HOVER_REDRAW_INTERVAL, NativeApp, NativeDiagnostics, NativeEffect,
        NativeInputCause, NativeMsg, NativeRedrawCause, NativeShell, NativeStage, PendingPointer,
        RedrawSchedule, WinitMouseButton, backend_supports_programmatic_focus,
        coalesce_pointer_move, map_event_loop_error, native_error, native_window_icon,
        next_pointer_dispatch, pointer_coords, pointer_event_kind, resolve_window_policy,
        sync_text_input_enabled, sync_window_focus_requested,
    };
    use crate::geometry::ScreenGeometry;
    use crate::input::{MouseEvent, MouseEventKind};
    use crate::lobby::storage::settings::PersistedWindowState;
    use crate::startup::{NativeBackendPreference, NativeLaunchOptions, NativeWindowMode};
    use crate::ui::UiScene;
    use nc_data::GameStateBuilder;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use winit::error::EventLoopError;
    use winit::keyboard::ModifiersState;

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
            MouseEventKind::Drag(crate::input::MouseButton::Left)
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
        let (cols, rows) = crate::native_grid::terminal_grid_for_pixels(100, 54, 1.0);
        let mut shell = test_shell(ScreenGeometry::new(1, 1), 100, 54);

        assert!(shell.resize_to_window_pixels(100, 54, 1.0));
        assert_eq!(
            shell.app.geometry,
            ScreenGeometry::new(cols as usize, rows as usize)
        );
    }

    #[test]
    fn resize_noops_when_pixels_and_grid_are_already_current() {
        let (cols, rows) = crate::native_grid::terminal_grid_for_pixels(100, 54, 1.0);
        let mut shell = test_shell(ScreenGeometry::new(cols as usize, rows as usize), 100, 54);

        assert!(!shell.resize_to_window_pixels(100, 54, 1.0));
        assert_eq!(
            shell.app.geometry,
            ScreenGeometry::new(cols as usize, rows as usize)
        );
    }

    #[test]
    fn drag_queue_requests_redraw_without_immediate_pointer_dispatch() {
        let mut shell = test_shell(ScreenGeometry::new(10, 3), 100, 54);
        shell.left_mouse_down = true;
        shell.current_pointer = Some(PendingPointer::Cell(3, 1));

        let effects = shell.update(NativeMsg::QueuePointer(PendingPointer::Cell(7, 2)));

        assert_eq!(
            effects,
            vec![NativeEffect::RequestRedraw(NativeRedrawCause::Mouse)]
        );
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
        assert_eq!(
            effects,
            vec![NativeEffect::RequestRedraw(NativeRedrawCause::Mouse)]
        );
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
    fn focus_loss_clears_modifiers() {
        let mut shell = test_mouse_shell(false);
        shell.window_has_focus = Some(true);
        shell.modifiers = ModifiersState::SHIFT | ModifiersState::CONTROL;
        shell.text_input_enabled = Some(true);

        shell.clear_focus_state();

        assert_eq!(shell.modifiers, ModifiersState::empty());
        assert_eq!(shell.window_has_focus, Some(false));
        assert_eq!(shell.text_input_enabled, None);
    }

    #[test]
    fn text_input_sync_tracks_changes() {
        let mut current = None;

        assert!(sync_text_input_enabled(&mut current, true));
        assert_eq!(current, Some(true));
        assert!(!sync_text_input_enabled(&mut current, true));
        assert!(sync_text_input_enabled(&mut current, false));
        assert_eq!(current, Some(false));
    }

    #[test]
    fn window_focus_sync_requests_once_until_focus_changes() {
        let mut current = Some(false);

        assert!(sync_window_focus_requested(&mut current, true));
        assert_eq!(current, None);
        assert!(!sync_window_focus_requested(&mut current, true));

        current = Some(true);
        assert!(!sync_window_focus_requested(&mut current, true));

        current = Some(false);
        assert!(!sync_window_focus_requested(&mut current, false));
        assert_eq!(current, Some(false));
    }

    #[test]
    fn wayland_backend_does_not_support_programmatic_focus() {
        assert!(!backend_supports_programmatic_focus("wayland"));
        assert!(backend_supports_programmatic_focus("x11"));
    }

    #[test]
    fn native_error_includes_backend_and_stage_context() {
        let mut diagnostics = NativeDiagnostics::new(true);
        diagnostics.set_stage(NativeStage::RendererInit);
        diagnostics.set_last_input_cause(NativeInputCause::MouseMove);
        diagnostics.set_last_redraw_cause(NativeRedrawCause::Mouse);
        diagnostics.set_last_signature("route=HostedGame crosshair=8,10".to_string());
        diagnostics.set_activation_token_present(true);
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
        assert!(message.contains("renderer: glyphon-wgpu"));
        assert!(message.contains("last_input: mouse_move"));
        assert!(message.contains("last_redraw: mouse"));
        assert!(message.contains("activation_token: yes"));
        assert!(message.contains("signature: route=HostedGame crosshair=8,10"));
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

    #[test]
    fn windowed_policy_falls_back_to_maximized_grid_size_without_saved_state() {
        let policy = resolve_window_policy(
            NativeWindowMode::MaximizedWindow,
            ScreenGeometry::new(120, 40),
            None,
        );

        assert!(policy.maximized);
        assert!(!policy.fullscreen);
        assert!(policy.decorations);
        assert!(policy.inner_width > 0);
        assert!(policy.inner_height > 0);
    }

    #[test]
    fn windowed_policy_restores_saved_window_size() {
        let policy = resolve_window_policy(
            NativeWindowMode::MaximizedWindow,
            ScreenGeometry::new(120, 40),
            Some(PersistedWindowState {
                width: 1280,
                height: 720,
                maximized: false,
            }),
        );

        assert_eq!(policy.inner_width, 1280);
        assert_eq!(policy.inner_height, 720);
        assert!(!policy.maximized);
        assert!(!policy.fullscreen);
        assert!(policy.decorations);
    }

    #[test]
    fn windowed_policy_restores_saved_maximized_state() {
        let policy = resolve_window_policy(
            NativeWindowMode::MaximizedWindow,
            ScreenGeometry::new(120, 40),
            Some(PersistedWindowState {
                width: 1366,
                height: 768,
                maximized: true,
            }),
        );

        assert_eq!(policy.inner_width, 1366);
        assert_eq!(policy.inner_height, 768);
        assert!(policy.maximized);
        assert!(!policy.fullscreen);
        assert!(policy.decorations);
    }

    #[test]
    fn fullscreen_policy_overrides_saved_window_state() {
        let policy = resolve_window_policy(
            NativeWindowMode::BorderlessFullscreen,
            ScreenGeometry::new(120, 40),
            Some(PersistedWindowState {
                width: 1280,
                height: 720,
                maximized: true,
            }),
        );

        assert!(policy.fullscreen);
        assert!(!policy.maximized);
        assert!(!policy.decorations);
        assert_eq!(policy.inner_width, 1280);
        assert_eq!(policy.inner_height, 720);
    }

    #[test]
    fn native_window_icon_loads_embedded_asset() {
        assert!(native_window_icon().is_some());
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
        NativeShell::new(
            app,
            window_pixel_width,
            window_pixel_height,
            1.0,
            false,
            true,
        )
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
                wants_window_focus: false,
                wants_text_input: false,
            },
            window_pixel_width,
            window_pixel_height,
            1.0,
            false,
            true,
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
                wants_window_focus: false,
                wants_text_input: false,
            },
            100,
            54,
            1.0,
            false,
            true,
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
        wants_window_focus: bool,
        wants_text_input: bool,
    }

    impl NativeApp for TestApp {
        fn window_title(&self) -> &'static str {
            "test"
        }

        fn geometry(&self) -> ScreenGeometry {
            self.geometry
        }

        fn dispatch_key_event(&mut self, _key: crate::input::KeyEvent) {}

        fn wants_window_focus(&self) -> bool {
            self.wants_window_focus
        }

        fn wants_text_input(&self) -> bool {
            self.wants_text_input
        }

        fn dispatch_mouse_event(&mut self, mouse: MouseEvent) -> bool {
            self.mouse_dispatch_count += 1;
            self.last_mouse_column = Some(mouse.column);
            self.last_mouse_row = Some(mouse.row);
            self.mouse_state_changed
        }

        fn resize_canvas(&mut self, cols: u16, rows: u16) {
            self.geometry = ScreenGeometry::new(cols as usize, rows as usize);
        }

        fn render_scene(&self) -> Result<UiScene, Box<dyn std::error::Error>> {
            Ok(UiScene::from(crate::buffer::PlayfieldBuffer::new(
                self.geometry.width(),
                self.geometry.height(),
                theme::body_style(),
            )))
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
