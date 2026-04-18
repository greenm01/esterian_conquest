mod primitives;
mod renderer;

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::{Ime, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{
    ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy,
};
use winit::keyboard::ModifiersState;
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
use winit::platform::wayland::{
    EventLoopBuilderExtWayland, EventLoopExtWayland, WindowAttributesExtWayland,
};
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{Fullscreen, Window, WindowAttributes};

use crate::Point;
use crate::app::{App, Effect, MATRIX_FRAME_STEP, MIN_SUPPORTED_GEOMETRY, Msg, Route};
use crate::geometry;
use crate::input::{
    MouseButton, MouseEvent, MouseEventKind, key_event_from_winit, key_modifiers_from_winit,
};
use crate::startup::{LaunchTargetOptions, NativeBackendPreference, NativeWindowMode};
use crate::storage::{BootSnapshot, StorageActor, StoredSession};
use crate::transport::{LobbySnapshot, TransportActor};

pub fn run(options: LaunchTargetOptions) -> Result<(), Box<dyn std::error::Error>> {
    let (app, effects) = App::new(options.relay_override.clone());
    let mut builder = EventLoop::<RuntimeEvent>::with_user_event();
    apply_backend_preference(&mut builder, options.native.backend_preference);
    let event_loop = builder.build()?;
    let session_backend = detect_session_backend(&event_loop, options.native.backend_preference);
    let proxy = event_loop.create_proxy();
    let storage = StorageActor::start().map_err(|err| format!("storage init failed: {err}"))?;
    let transport = TransportActor::start();
    let mut runtime = Runtime::new(
        options,
        session_backend,
        proxy,
        app,
        storage,
        transport,
        effects,
    );
    event_loop.run_app(&mut runtime)?;
    Ok(())
}

#[derive(Debug, Clone)]
enum RuntimeEvent {
    BootLoaded(Result<BootSnapshot, String>),
    IdentityCreated(Result<StoredSession, String>),
    Unlocked(Result<StoredSession, String>),
    LobbyUpdated(Result<LobbySnapshot, String>),
    RelaySaved(Result<String, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionBackend {
    Wayland,
    X11,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResizeObservation {
    pixel_width: u32,
    pixel_height: u32,
    scale_factor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResizeShrinkTracker {
    baseline_height: u32,
    consecutive_shrinks: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeVerdict {
    Accept,
    SpuriousShrink { restore_to: (u32, u32) },
}

struct Runtime {
    options: LaunchTargetOptions,
    session_backend: SessionBackend,
    proxy: EventLoopProxy<RuntimeEvent>,
    app: App,
    storage: StorageActor,
    transport: TransportActor,
    pending_effects: Vec<Effect>,
    window: Option<Arc<Window>>,
    renderer: Option<renderer::Renderer>,
    modifiers: ModifiersState,
    pointer_cell: Option<Point>,
    last_resize_observation: Option<ResizeObservation>,
    needs_redraw: bool,
    last_user_input: Option<Instant>,
    mouse_buttons_held: u16,
    shrink_tracker: Option<ResizeShrinkTracker>,
    next_matrix_frame_at: Option<Instant>,
}

impl Runtime {
    fn new(
        options: LaunchTargetOptions,
        session_backend: SessionBackend,
        proxy: EventLoopProxy<RuntimeEvent>,
        app: App,
        storage: StorageActor,
        transport: TransportActor,
        pending_effects: Vec<Effect>,
    ) -> Self {
        Self {
            options,
            session_backend,
            proxy,
            app,
            storage,
            transport,
            pending_effects,
            window: None,
            renderer: None,
            modifiers: ModifiersState::empty(),
            pointer_cell: None,
            last_resize_observation: None,
            needs_redraw: true,
            last_user_input: None,
            mouse_buttons_held: 0,
            shrink_tracker: None,
            next_matrix_frame_at: None,
        }
    }

    fn create_window(
        &self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Arc<Window>, Box<dyn std::error::Error>> {
        let geometry = self.app.model().geometry;
        let size = geometry::logical_window_size_for_grid(geometry.width(), geometry.height());
        let min_size = minimum_window_size();
        let mut attributes = WindowAttributes::default()
            .with_title("Nostrian Conquest - nc-helm")
            .with_resizable(true)
            .with_decorations(window_decorations_for_session(
                self.session_backend,
                std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref(),
            ))
            .with_inner_size(size)
            .with_min_inner_size(min_size);
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            attributes = attributes.with_name("nc-helm", "nc-helm");
        }
        match self.options.native.window_mode {
            NativeWindowMode::Windowed => {}
            NativeWindowMode::BorderlessFullscreen => {
                attributes = attributes.with_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            let token = event_loop.read_token_from_env();
            reset_activation_token_env();
            if let Some(token) = token {
                attributes = attributes.with_activation_token(token);
            }
        }
        Ok(Arc::new(event_loop.create_window(attributes)?))
    }

    fn dispatch(&mut self, msg: Msg, event_loop: &ActiveEventLoop) {
        let msg_label = msg_label(&msg);
        let sync_window_input_state = !matches!(msg, Msg::Resize(_));
        let effects = self.app.dispatch(msg);
        self.diagnostic_log(&format!(
            "state: msg={} route={} focus={} network={:?}",
            msg_label,
            route_label(&self.app.model().route),
            self.app.model().window_focused,
            self.app.model().network
        ));
        self.pending_effects.extend(effects);
        self.process_effects(event_loop);
        if self.app.model().session.is_some() && self.last_user_input.is_none() {
            self.last_user_input = Some(Instant::now());
        }
        if !matches!(self.app.model().route, Route::MatrixLocked) {
            self.next_matrix_frame_at = None;
        }
        if sync_window_input_state {
            self.sync_window_input_state();
        }
        self.needs_redraw = true;
        if self.app.model().should_quit {
            event_loop.exit();
        }
    }

    fn process_effects(&mut self, event_loop: &ActiveEventLoop) {
        while let Some(effect) = self.pending_effects.pop() {
            match effect {
                Effect::LoadBoot => {
                    self.diagnostic_log("dispatch effect: LoadBoot");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.load_boot(tx) {
                        self.dispatch(Msg::BootLoaded(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::BootLoaded(result));
                        }
                    });
                }
                Effect::CreateIdentity {
                    handle,
                    password,
                    relay_url,
                } => {
                    self.diagnostic_log("dispatch effect: CreateIdentity");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self
                        .storage
                        .create_identity(handle, password, relay_url, tx)
                    {
                        self.dispatch(Msg::IdentityCreated(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::IdentityCreated(result));
                        }
                    });
                }
                Effect::SaveRelayUrl { relay_url } => {
                    self.diagnostic_log("dispatch effect: SaveRelayUrl");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.save_relay_url(relay_url, tx) {
                        self.dispatch(Msg::RelaySaved(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::RelaySaved(result));
                        }
                    });
                }
                Effect::Unlock { password } => {
                    self.diagnostic_log("dispatch effect: Unlock");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.unlock(password, tx) {
                        self.dispatch(Msg::Unlocked(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::Unlocked(result));
                        }
                    });
                }
                Effect::ConnectTransport {
                    relay_url,
                    nsec,
                    cache,
                } => {
                    self.diagnostic_log("dispatch effect: ConnectTransport");
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.transport.connect(relay_url, nsec, cache, tx) {
                        self.dispatch(Msg::LobbyUpdated(Err(err)), event_loop);
                        continue;
                    }
                    thread::spawn(move || {
                        while let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::LobbyUpdated(result));
                        }
                    });
                }
                Effect::DisconnectTransport => {
                    self.diagnostic_log("dispatch effect: DisconnectTransport");
                    if let Err(err) = self.transport.disconnect() {
                        self.dispatch(Msg::LobbyUpdated(Err(err)), event_loop);
                    }
                }
                Effect::SaveClientCache { cache, password } => {
                    self.diagnostic_log("dispatch effect: SaveClientCache");
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.save_client_cache(cache, password, tx) {
                        self.diagnostic_log(&format!("save client cache failed: {err}"));
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(Err(err)) = rx.recv() {
                            eprintln!("nc-helm cache save failed: {err}");
                        }
                    });
                }
                Effect::SaveLockTimeout {
                    lock_timeout_minutes,
                } => {
                    self.diagnostic_log("dispatch effect: SaveLockTimeout");
                    let (tx, rx) = std::sync::mpsc::channel();
                    if let Err(err) = self.storage.save_lock_timeout(lock_timeout_minutes, tx) {
                        self.diagnostic_log(&format!("save lock timeout failed: {err}"));
                        continue;
                    }
                    thread::spawn(move || {
                        if let Ok(Err(err)) = rx.recv() {
                            eprintln!("nc-helm lock-timeout save failed: {err}");
                        }
                    });
                }
                Effect::Quit => event_loop.exit(),
            }
        }
    }

    fn diagnostic_log(&self, message: &str) {
        if self.options.native.diagnostic_mode {
            eprintln!("nc-helm diagnostic: {message}");
        }
    }

    fn sync_window_input_state(&self) {
        let Some(window) = &self.window else {
            return;
        };
        window.set_ime_allowed(self.app.model().wants_text_input());
        if backend_supports_programmatic_focus(self.session_backend)
            && self.app.model().wants_window_focus()
            && !self.app.model().window_focused
        {
            window.focus_window();
        }
    }

    fn observe_resize(&mut self, observation: ResizeObservation) -> ResizeVerdict {
        let now = Instant::now();
        let input_recency_ms = self
            .last_user_input
            .map(|t| now.duration_since(t).as_millis() as u32);
        let (verdict, new_tracker) = classify_resize(
            &self.last_resize_observation,
            observation,
            &self.shrink_tracker,
            input_recency_ms,
        );
        self.shrink_tracker = Some(new_tracker);
        self.last_resize_observation = Some(observation);
        verdict
    }

    fn diagnostic_resize_event(
        &self,
        label: &str,
        event_width: u32,
        event_height: u32,
        scale_factor: f64,
    ) {
        if !self.options.native.diagnostic_mode {
            return;
        }
        let backend = session_backend_label(self.session_backend);
        if let Some(window) = self.window.as_ref() {
            let inner = window.inner_size();
            let outer = window.outer_size();
            self.diagnostic_log(&format!(
                "event: {label} backend={backend} event={}x{} inner={}x{} outer={}x{} scale={scale_factor:.3}",
                event_width,
                event_height,
                inner.width,
                inner.height,
                outer.width,
                outer.height
            ));
        } else {
            self.diagnostic_log(&format!(
                "event: {label} backend={backend} event={}x{} scale={scale_factor:.3}",
                event_width, event_height
            ));
        }
    }

    fn sync_geometry_from_size(
        &mut self,
        pixel_width: u32,
        pixel_height: u32,
        scale_factor: f64,
        event_loop: &ActiveEventLoop,
    ) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let geometry = renderer.grid_geometry_for_pixels(pixel_width, pixel_height, scale_factor);
        if self.app.model().geometry == geometry {
            return;
        }
        self.dispatch(Msg::Resize(geometry), event_loop);
    }

    fn scheduled_wakeup(&mut self, now: Instant) -> Option<Instant> {
        let idle_deadline = match (
            self.app.model().session.is_some(),
            matches!(self.app.model().route, Route::Lobby(_)),
            self.app.model().lock_timeout_minutes,
            self.last_user_input,
        ) {
            (true, true, timeout, Some(last_input)) if timeout != 0 => {
                Some(last_input + Duration::from_secs(u64::from(timeout) * 60))
            }
            _ => None,
        };

        let matrix_deadline = if matches!(self.app.model().route, Route::MatrixLocked) {
            Some(
                *self
                    .next_matrix_frame_at
                    .get_or_insert(now + MATRIX_FRAME_STEP),
            )
        } else {
            self.next_matrix_frame_at = None;
            None
        };

        combine_deadlines(idle_deadline, matrix_deadline)
    }
}

impl ApplicationHandler<RuntimeEvent> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.diagnostic_log(&format!(
                "event: resumed backend={}",
                session_backend_label(self.session_backend)
            ));
            match self.create_window(event_loop) {
                Ok(window) => {
                    let renderer = match renderer::Renderer::new(window.clone(), event_loop) {
                        Ok(renderer) => renderer,
                        Err(err) => {
                            self.dispatch(
                                Msg::BootLoaded(Err(format!("renderer init failed: {err}"))),
                                event_loop,
                            );
                            return;
                        }
                    };
                    self.window = Some(window);
                    self.renderer = Some(renderer);
                    if let Some(window) = self.window.as_ref() {
                        let size = window.inner_size();
                        let scale_factor = window.scale_factor();
                        let _ = self.observe_resize(ResizeObservation {
                            pixel_width: size.width,
                            pixel_height: size.height,
                            scale_factor,
                        });
                        self.sync_geometry_from_size(
                            size.width,
                            size.height,
                            scale_factor,
                            event_loop,
                        );
                    }
                    self.needs_redraw = true;
                    self.process_effects(event_loop);
                    self.sync_window_input_state();
                }
                Err(err) => {
                    self.dispatch(
                        Msg::BootLoaded(Err(format!("window creation failed: {err}"))),
                        event_loop,
                    );
                }
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent) {
        match event {
            RuntimeEvent::BootLoaded(result) => {
                self.diagnostic_log("event: BootLoaded");
                self.dispatch(Msg::BootLoaded(result), event_loop)
            }
            RuntimeEvent::IdentityCreated(result) => {
                self.diagnostic_log("event: IdentityCreated");
                self.dispatch(Msg::IdentityCreated(result), event_loop)
            }
            RuntimeEvent::Unlocked(result) => {
                self.diagnostic_log("event: Unlocked");
                self.dispatch(Msg::Unlocked(result), event_loop)
            }
            RuntimeEvent::LobbyUpdated(result) => {
                self.diagnostic_log("event: LobbyUpdated");
                self.dispatch(Msg::LobbyUpdated(result), event_loop)
            }
            RuntimeEvent::RelaySaved(result) => {
                self.diagnostic_log("event: RelaySaved");
                self.dispatch(Msg::RelaySaved(result), event_loop)
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.dispatch(
                    Msg::Key(crate::input::KeyEvent::new(
                        crate::input::KeyCode::Char('q'),
                        crate::input::KeyModifiers::ALT,
                    )),
                    event_loop,
                );
            }
            WindowEvent::Focused(focused) => {
                if self.app.model().window_focused != focused {
                    self.dispatch(Msg::FocusChanged(focused), event_loop);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.last_user_input = Some(Instant::now());
                if let Some(key) = key_event_from_winit(&event, self.modifiers) {
                    self.dispatch(Msg::Key(key), event_loop);
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if should_dispatch_text_commit(&text) {
                    self.dispatch(Msg::TextInput(text.to_string()), event_loop);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.mouse_buttons_held > 0 {
                    self.last_user_input = Some(Instant::now());
                }
                if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
                    self.pointer_cell = renderer.cell_position_at_pixel(
                        window,
                        self.app.model().geometry,
                        position,
                    );
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if state.is_pressed() {
                    self.mouse_buttons_held += 1;
                    self.last_user_input = Some(Instant::now());
                } else {
                    self.mouse_buttons_held = self.mouse_buttons_held.saturating_sub(1);
                }
                let button = match button {
                    WinitMouseButton::Left => Some(MouseButton::Left),
                    WinitMouseButton::Right => Some(MouseButton::Right),
                    WinitMouseButton::Middle => Some(MouseButton::Middle),
                    _ => None,
                };
                if let (Some(button), Some(position)) = (button, self.pointer_cell) {
                    self.dispatch(
                        Msg::Mouse(MouseEvent {
                            kind: if state.is_pressed() {
                                MouseEventKind::Down(button)
                            } else {
                                MouseEventKind::Up(button)
                            },
                            position,
                            modifiers: key_modifiers_from_winit(self.modifiers),
                        }),
                        event_loop,
                    );
                }
            }
            WindowEvent::Resized(size) => {
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);
                let observation = ResizeObservation {
                    pixel_width: size.width,
                    pixel_height: size.height,
                    scale_factor,
                };
                let verdict = self.observe_resize(observation);
                match verdict {
                    ResizeVerdict::SpuriousShrink { restore_to } => {
                        if self.options.native.diagnostic_mode {
                            self.diagnostic_log(&format!(
                                "event: Resized backend={} spurious=true event={}x{} baseline={}x{}",
                                session_backend_label(self.session_backend),
                                size.width,
                                size.height,
                                restore_to.1,
                                restore_to.0
                            ));
                        }
                        // Counter-configure was tried and cosmic-comp ignored it; min_inner_size
                        // is now the primary defense against sctk-adwaita #67/#68. We still sync
                        // geometry so the app model matches the actual surface.
                        self.sync_geometry_from_size(
                            size.width,
                            size.height,
                            scale_factor,
                            event_loop,
                        );
                    }
                    ResizeVerdict::Accept => {
                        self.diagnostic_resize_event(
                            "Resized",
                            size.width,
                            size.height,
                            scale_factor,
                        );
                        self.sync_geometry_from_size(
                            size.width,
                            size.height,
                            scale_factor,
                            event_loop,
                        );
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(window) = self.window.as_ref() {
                    let size = window.inner_size();
                    let observation = ResizeObservation {
                        pixel_width: size.width,
                        pixel_height: size.height,
                        scale_factor,
                    };
                    let _ = self.observe_resize(observation);
                    self.diagnostic_resize_event(
                        "ScaleFactorChanged",
                        size.width,
                        size.height,
                        scale_factor,
                    );
                    self.sync_geometry_from_size(size.width, size.height, scale_factor, event_loop);
                }
            }
            WindowEvent::RedrawRequested => {
                self.diagnostic_log(&format!(
                    "event: RedrawRequested route={}",
                    route_label(&self.app.model().route)
                ));
                if let Some(renderer) = &mut self.renderer {
                    let buffer = self.app.view();
                    if let Err(err) = renderer.render(&buffer) {
                        eprintln!("nc-helm render error: {err}");
                        event_loop.exit();
                    } else {
                        self.needs_redraw = false;
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        _event_loop.set_control_flow(ControlFlow::Wait);
        let now = Instant::now();
        if matches!(self.app.model().route, Route::MatrixLocked)
            && self
                .next_matrix_frame_at
                .map(|deadline| deadline <= now)
                .unwrap_or(false)
        {
            self.next_matrix_frame_at = Some(now + MATRIX_FRAME_STEP);
            self.dispatch(Msg::MatrixFrame, _event_loop);
        }
        if self.app.model().session.is_some()
            && matches!(self.app.model().route, Route::Lobby(_))
            && self.app.model().lock_timeout_minutes != 0
            && self
                .last_user_input
                .map(|last_input| {
                    now >= last_input
                        + Duration::from_secs(u64::from(self.app.model().lock_timeout_minutes) * 60)
                })
                .unwrap_or(false)
        {
            self.dispatch(Msg::IdleLock, _event_loop);
            self.last_user_input = None;
        }
        if self.needs_redraw {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        if let Some(deadline) = self.scheduled_wakeup(Instant::now()) {
            _event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }
}

fn combine_deadlines(left: Option<Instant>, right: Option<Instant>) -> Option<Instant> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn backend_supports_programmatic_focus(session_backend: SessionBackend) -> bool {
    session_backend != SessionBackend::Wayland
}

fn classify_resize(
    last_observation: &Option<ResizeObservation>,
    observation: ResizeObservation,
    shrink_tracker: &Option<ResizeShrinkTracker>,
    input_recency_ms: Option<u32>,
) -> (ResizeVerdict, ResizeShrinkTracker) {
    let fresh_tracker = ResizeShrinkTracker {
        baseline_height: observation.pixel_height,
        consecutive_shrinks: 0,
    };
    let Some(last) = *last_observation else {
        return (ResizeVerdict::Accept, fresh_tracker);
    };
    let same_width = observation.pixel_width == last.pixel_width;
    let shrink = observation.pixel_height < last.pixel_height;
    let grow = observation.pixel_height > last.pixel_height;
    let no_recent_input = input_recency_ms.map_or(true, |ms| ms > 250);
    let scale_unchanged = (observation.scale_factor - last.scale_factor).abs() < 0.001;

    // User-driven or compositor-intended change: reset tracker baseline.
    if !same_width || !scale_unchanged || !no_recent_input {
        return (ResizeVerdict::Accept, fresh_tracker);
    }

    // Grow without recent input (e.g. compositor restore): new baseline at the
    // larger height.
    if grow {
        return (ResizeVerdict::Accept, fresh_tracker);
    }

    // No height change.
    if !shrink {
        let unchanged_tracker = shrink_tracker.unwrap_or(ResizeShrinkTracker {
            baseline_height: last.pixel_height,
            consecutive_shrinks: 0,
        });
        return (ResizeVerdict::Accept, unchanged_tracker);
    }

    // same_width && shrink && scale_unchanged && no_recent_input
    // Baseline is sticky during a shrink run; only the first shrink establishes it
    // when no tracker exists.
    let baseline = shrink_tracker
        .map(|t| t.baseline_height)
        .unwrap_or(last.pixel_height);
    let new_shrinks = shrink_tracker
        .map(|t| t.consecutive_shrinks)
        .unwrap_or(0)
        .saturating_add(1);
    let new_tracker = ResizeShrinkTracker {
        baseline_height: baseline,
        consecutive_shrinks: new_shrinks,
    };
    if new_shrinks >= 2 && observation.pixel_height <= baseline {
        (
            ResizeVerdict::SpuriousShrink {
                restore_to: (baseline, last.pixel_width),
            },
            new_tracker,
        )
    } else {
        (ResizeVerdict::Accept, new_tracker)
    }
}

fn session_backend_label(session_backend: SessionBackend) -> &'static str {
    match session_backend {
        SessionBackend::Wayland => "wayland",
        SessionBackend::X11 => "x11",
        SessionBackend::Unknown => "unknown",
    }
}

/// Decide whether the window should be created with native decorations.
///
/// Cosmic-comp's server-side decorations path has a bug that shrinks the
/// xdg_toplevel by 36 px on every `wl_surface.leave/enter(output)` pair,
/// which fires whenever the pointer crosses the SSD titlebar edge. Requesting
/// `decorations=false` makes winit ask for client-side decorations instead,
/// which cosmic-comp honors and the shrink loop disappears. On every other
/// platform/compositor we keep native decorations so users get a normal
/// titlebar and standard resize/move behavior.
///
/// TODO: remove this workaround once cosmic-comp stops shrinking SSD
/// windows by 36 px on every drag. Tracked upstream as
/// pop-os/cosmic-comp#2300 (follow-up to #1469, which was closed
/// won't-fix but only covered the client-visible leave/enter events,
/// not the SSD geometry recomputation that causes the shrink).
/// COSMIC is currently in alpha/beta; revisit when the compositor stabilizes.
fn window_decorations_for_session(
    session_backend: SessionBackend,
    xdg_current_desktop: Option<&str>,
) -> bool {
    if session_backend != SessionBackend::Wayland {
        return true;
    }
    let is_cosmic = xdg_current_desktop
        .map(|value| {
            value
                .split(':')
                .any(|entry| entry.eq_ignore_ascii_case("COSMIC"))
        })
        .unwrap_or(false);
    !is_cosmic
}

fn detect_session_backend(
    event_loop: &EventLoop<RuntimeEvent>,
    backend_preference: NativeBackendPreference,
) -> SessionBackend {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        if event_loop.is_wayland() {
            SessionBackend::Wayland
        } else if matches!(backend_preference, NativeBackendPreference::X11)
            || std::env::var_os("DISPLAY").is_some()
            || matches!(
                std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
                Some("x11")
            )
        {
            SessionBackend::X11
        } else {
            SessionBackend::Unknown
        }
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    )))]
    {
        let _ = event_loop;
        let _ = backend_preference;
        SessionBackend::Unknown
    }
}

fn should_dispatch_text_commit(text: &str) -> bool {
    let mut chars = text.chars().filter(|ch| !ch.is_control());
    let Some(first) = chars.next() else {
        return false;
    };
    chars.next().is_some() || !first.is_ascii()
}

fn route_label(route: &Route) -> &'static str {
    match route {
        Route::Boot(_) => "Boot",
        Route::FirstRun(_) => "FirstRun",
        Route::MatrixLocked => "MatrixLocked",
        Route::Locked(_) => "Locked",
        Route::Lobby(_) => "Lobby",
        Route::FatalError(_) => "FatalError",
    }
}

fn msg_label(msg: &Msg) -> &'static str {
    match msg {
        Msg::Resize(_) => "Resize",
        Msg::FocusChanged(_) => "FocusChanged",
        Msg::MatrixFrame => "MatrixFrame",
        Msg::IdleLock => "IdleLock",
        Msg::Key(_) => "Key",
        Msg::TextInput(_) => "TextInput",
        Msg::Mouse(_) => "Mouse",
        Msg::BootLoaded(_) => "BootLoaded",
        Msg::IdentityCreated(_) => "IdentityCreated",
        Msg::Unlocked(_) => "Unlocked",
        Msg::LobbyUpdated(_) => "LobbyUpdated",
        Msg::RelaySaved(_) => "RelaySaved",
    }
}

fn apply_backend_preference(
    builder: &mut EventLoopBuilder<RuntimeEvent>,
    backend_preference: NativeBackendPreference,
) {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    match backend_preference {
        NativeBackendPreference::Auto => {}
        NativeBackendPreference::Wayland => {
            builder.with_wayland();
        }
        NativeBackendPreference::X11 => {
            builder.with_x11();
        }
    }
}

fn minimum_window_size() -> winit::dpi::LogicalSize<f64> {
    geometry::logical_window_size_for_grid(
        MIN_SUPPORTED_GEOMETRY.width(),
        MIN_SUPPORTED_GEOMETRY.height(),
    )
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        ResizeObservation, ResizeShrinkTracker, ResizeVerdict, SessionBackend,
        backend_supports_programmatic_focus, classify_resize, combine_deadlines,
        minimum_window_size, session_backend_label, window_decorations_for_session,
    };
    use crate::app::MIN_SUPPORTED_GEOMETRY;
    use crate::geometry;

    #[test]
    fn wayland_backend_disables_programmatic_focus() {
        assert!(!backend_supports_programmatic_focus(
            SessionBackend::Wayland
        ));
        assert!(backend_supports_programmatic_focus(SessionBackend::X11));
        assert!(backend_supports_programmatic_focus(SessionBackend::Unknown));
    }

    #[test]
    fn backend_labels_match_expected_cli_terms() {
        assert_eq!(session_backend_label(SessionBackend::Wayland), "wayland");
        assert_eq!(session_backend_label(SessionBackend::X11), "x11");
        assert_eq!(session_backend_label(SessionBackend::Unknown), "unknown");
    }

    #[test]
    fn cosmic_wayland_session_disables_decorations() {
        assert!(!window_decorations_for_session(
            SessionBackend::Wayland,
            Some("COSMIC")
        ));
        assert!(!window_decorations_for_session(
            SessionBackend::Wayland,
            Some("cosmic")
        ));
        assert!(!window_decorations_for_session(
            SessionBackend::Wayland,
            Some("pop:COSMIC")
        ));
    }

    #[test]
    fn non_cosmic_wayland_keeps_decorations() {
        assert!(window_decorations_for_session(
            SessionBackend::Wayland,
            Some("GNOME")
        ));
        assert!(window_decorations_for_session(
            SessionBackend::Wayland,
            Some("KDE")
        ));
        assert!(window_decorations_for_session(
            SessionBackend::Wayland,
            None
        ));
    }

    #[test]
    fn non_wayland_always_keeps_decorations() {
        assert!(window_decorations_for_session(
            SessionBackend::X11,
            Some("COSMIC")
        ));
        assert!(window_decorations_for_session(
            SessionBackend::Unknown,
            Some("COSMIC")
        ));
        assert!(window_decorations_for_session(SessionBackend::X11, None));
    }

    #[test]
    fn minimum_window_size_matches_supported_grid_floor() {
        assert_eq!(
            minimum_window_size(),
            geometry::logical_window_size_for_grid(
                MIN_SUPPORTED_GEOMETRY.width(),
                MIN_SUPPORTED_GEOMETRY.height()
            )
        );
    }

    #[test]
    fn combine_deadlines_picks_the_earliest_present_deadline() {
        let now = Instant::now();
        assert_eq!(combine_deadlines(None, None), None);
        assert_eq!(combine_deadlines(Some(now), None), Some(now));
        assert_eq!(combine_deadlines(None, Some(now)), Some(now));
        assert_eq!(
            combine_deadlines(
                Some(now + Duration::from_secs(2)),
                Some(now + Duration::from_secs(1))
            ),
            Some(now + Duration::from_secs(1))
        );
    }

    #[test]
    fn resize_observation_equality_requires_exact_match() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 800,
            scale_factor: 1.25,
        };
        assert_eq!(base, base);
        assert_ne!(
            base,
            ResizeObservation {
                pixel_width: 1201,
                ..base
            }
        );
        assert_ne!(
            base,
            ResizeObservation {
                pixel_height: 801,
                ..base
            }
        );
        assert_ne!(
            base,
            ResizeObservation {
                scale_factor: 1.5,
                ..base
            }
        );
    }

    // Helper: drive classify_resize through a sequence of observations, threading
    // the tracker state between calls just as observe_resize does at runtime.
    fn run_sequence(
        heights: &[u32],
        width: u32,
        scale: f64,
        input_recency_ms: Option<u32>,
    ) -> Vec<ResizeVerdict> {
        let mut last: Option<ResizeObservation> = None;
        let mut tracker: Option<ResizeShrinkTracker> = None;
        let mut verdicts = Vec::new();
        for &h in heights {
            let obs = ResizeObservation {
                pixel_width: width,
                pixel_height: h,
                scale_factor: scale,
            };
            let (verdict, new_tracker) = classify_resize(&last, obs, &tracker, input_recency_ms);
            verdicts.push(verdict);
            last = Some(obs);
            tracker = Some(new_tracker);
        }
        verdicts
    }

    #[test]
    fn classify_resize_rejects_monotonic_shrink_sequence() {
        // Thread tracker state through calls, same as observe_resize does at runtime.
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let shrink1 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let shrink2 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 756,
            scale_factor: 1.0,
        };

        // First call: no prior observation → Accept, tracker initialised.
        let (v0, t0) = classify_resize(&None, base, &None, Some(1000));
        assert_eq!(v0, ResizeVerdict::Accept);
        assert_eq!(t0.consecutive_shrinks, 0);
        assert_eq!(t0.baseline_height, 828);

        // Second call: first shrink → Accept (shrinks=1, baseline stays 828).
        let (v1, t1) = classify_resize(&Some(base), shrink1, &Some(t0), Some(1000));
        assert_eq!(v1, ResizeVerdict::Accept);
        assert_eq!(t1.consecutive_shrinks, 1);
        assert_eq!(t1.baseline_height, 828);

        // Third call: second shrink → SpuriousShrink (shrinks=2 >= threshold).
        let (v2, t2) = classify_resize(&Some(shrink1), shrink2, &Some(t1), Some(1000));
        assert_eq!(
            v2,
            ResizeVerdict::SpuriousShrink {
                restore_to: (828, 1200)
            }
        );
        assert_eq!(t2.consecutive_shrinks, 2);
        assert_eq!(t2.baseline_height, 828);
    }

    #[test]
    fn classify_resize_accepts_when_user_input_is_recent() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let shrink1 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let tracker_with_shrinks = Some(ResizeShrinkTracker {
            baseline_height: 828,
            consecutive_shrinks: 1,
        });

        let (verdict, _) = classify_resize(&Some(base), shrink1, &tracker_with_shrinks, Some(50));
        assert_eq!(verdict, ResizeVerdict::Accept);
    }

    #[test]
    fn classify_resize_accepts_first_shrink() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let shrink1 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };

        let (verdict, tracker) = classify_resize(&Some(base), shrink1, &None, Some(1000));
        assert_eq!(verdict, ResizeVerdict::Accept);
        assert_eq!(tracker.consecutive_shrinks, 1);
        assert_eq!(tracker.baseline_height, 828);
    }

    #[test]
    fn classify_resize_accepts_different_width() {
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let different_width = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 792,
            scale_factor: 1.0,
        };

        let (verdict, tracker) = classify_resize(&Some(base), different_width, &None, Some(1000));
        assert_eq!(verdict, ResizeVerdict::Accept);
        // Width change resets tracker: baseline is the new height, shrinks=0.
        assert_eq!(tracker.consecutive_shrinks, 0);
        assert_eq!(tracker.baseline_height, 792);
    }

    // --- new regression tests ---

    #[test]
    fn classify_resize_cosmic_full_sequence() {
        // COSMIC sctk-adwaita shrink loop: 864 → 828 → 792 → … → 540 (10 steps).
        // No user input throughout.
        // Expected: first observation Accept (no prior), second Accept (shrinks=1),
        // third onward all SpuriousShrink with restore_to=(864, 1200).
        let heights: Vec<u32> = vec![864, 828, 792, 756, 720, 684, 648, 612, 576, 540];
        let verdicts = run_sequence(&heights, 1200, 1.0, None);

        assert_eq!(verdicts[0], ResizeVerdict::Accept); // no prior
        assert_eq!(verdicts[1], ResizeVerdict::Accept); // shrinks=1
        for v in &verdicts[2..] {
            assert_eq!(
                *v,
                ResizeVerdict::SpuriousShrink {
                    restore_to: (864, 1200)
                }
            );
        }
    }

    #[test]
    fn classify_resize_grow_resets_baseline() {
        // After a grow the baseline rises; a subsequent shrink loop restores to the
        // new (larger) baseline, not the old one.
        // Sequence: 864 → 828 (shrinks=1) → 900 (grow, baseline=900, shrinks=0)
        //           → 864 (shrinks=1) → 828 (SpuriousShrink restore_to=900)
        let heights: Vec<u32> = vec![864, 828, 900, 864, 828];
        let verdicts = run_sequence(&heights, 1200, 1.0, None);

        assert_eq!(verdicts[0], ResizeVerdict::Accept); // no prior
        assert_eq!(verdicts[1], ResizeVerdict::Accept); // first shrink
        assert_eq!(verdicts[2], ResizeVerdict::Accept); // grow → baseline=900
        assert_eq!(verdicts[3], ResizeVerdict::Accept); // shrink from 900, shrinks=1
        assert_eq!(
            verdicts[4],
            ResizeVerdict::SpuriousShrink {
                restore_to: (900, 1200)
            }
        );
    }

    #[test]
    fn classify_resize_recent_input_resets_tracker() {
        // A shrink that arrives within 250 ms of user input is treated as
        // user-driven: it resets the baseline to the shrunken height.
        // A later shrink loop is then detected relative to the new baseline.
        let base = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 864,
            scale_factor: 1.0,
        };
        let user_shrink = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        // First: accept with recent input (50 ms) → baseline resets to 828.
        let (v0, t0) = classify_resize(&Some(base), user_shrink, &None, Some(50));
        assert_eq!(v0, ResizeVerdict::Accept);
        assert_eq!(t0.baseline_height, 828);
        assert_eq!(t0.consecutive_shrinks, 0);

        // Now feed two compositor shrinks without input → spurious at the third.
        let shrink2 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let shrink3 = ResizeObservation {
            pixel_width: 1200,
            pixel_height: 756,
            scale_factor: 1.0,
        };
        let (v1, t1) = classify_resize(&Some(user_shrink), shrink2, &Some(t0), Some(1000));
        assert_eq!(v1, ResizeVerdict::Accept);
        assert_eq!(t1.baseline_height, 828); // sticky

        let (v2, _) = classify_resize(&Some(shrink2), shrink3, &Some(t1), Some(1000));
        assert_eq!(
            v2,
            ResizeVerdict::SpuriousShrink {
                restore_to: (828, 1200)
            }
        );
    }

    #[test]
    fn classify_resize_width_change_resets_tracker() {
        // Accumulate two shrinks, then a width change resets the tracker so
        // the following shrink run starts fresh.
        let heights_before: Vec<u32> = vec![864, 828, 792]; // shrinks=0,1,spurious
        let v_before = run_sequence(&heights_before, 1200, 1.0, None);
        assert_eq!(
            v_before[2],
            ResizeVerdict::SpuriousShrink {
                restore_to: (864, 1200)
            }
        );

        // Now: different width resize → tracker reset → next two shrinks at new
        // width are Accept (shrinks=1) then SpuriousShrink on the third.
        let obs_wide = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 864,
            scale_factor: 1.0,
        };
        // Simulate starting fresh from a width-changed state.
        let (v_wide, t_wide) = classify_resize(
            &Some(ResizeObservation {
                pixel_width: 1200,
                pixel_height: 792,
                scale_factor: 1.0,
            }),
            obs_wide,
            &None,
            Some(1000),
        );
        assert_eq!(v_wide, ResizeVerdict::Accept);
        assert_eq!(t_wide.consecutive_shrinks, 0);
        assert_eq!(t_wide.baseline_height, 864);

        // Subsequent shrinks at the new width behave as a fresh sequence.
        let shrink_a = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 828,
            scale_factor: 1.0,
        };
        let (va, ta) = classify_resize(&Some(obs_wide), shrink_a, &Some(t_wide), Some(1000));
        assert_eq!(va, ResizeVerdict::Accept);
        assert_eq!(ta.consecutive_shrinks, 1);
        assert_eq!(ta.baseline_height, 864);

        let shrink_b = ResizeObservation {
            pixel_width: 1000,
            pixel_height: 792,
            scale_factor: 1.0,
        };
        let (vb, _) = classify_resize(&Some(shrink_a), shrink_b, &Some(ta), Some(1000));
        assert_eq!(
            vb,
            ResizeVerdict::SpuriousShrink {
                restore_to: (864, 1000)
            }
        );
    }
}
