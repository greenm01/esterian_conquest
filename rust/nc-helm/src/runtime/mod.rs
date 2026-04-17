mod renderer;

use std::sync::Arc;
use std::thread;

use winit::application::ApplicationHandler;
use winit::event::{MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopBuilder, EventLoopProxy};
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
use winit::platform::wayland::EventLoopBuilderExtWayland;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{Fullscreen, Window, WindowAttributes};

use crate::app::{App, Effect, Msg};
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
    let proxy = event_loop.create_proxy();
    let storage = StorageActor::start();
    let transport = TransportActor::start();
    let mut runtime = Runtime::new(options, proxy, app, storage, transport, effects);
    event_loop.run_app(&mut runtime)?;
    Ok(())
}

#[derive(Debug, Clone)]
enum RuntimeEvent {
    BootLoaded(Result<BootSnapshot, String>),
    IdentityCreated(Result<StoredSession, String>),
    Unlocked(Result<StoredSession, String>),
    LobbyUpdated(Result<LobbySnapshot, String>),
}

struct Runtime {
    options: LaunchTargetOptions,
    proxy: EventLoopProxy<RuntimeEvent>,
    app: App,
    storage: StorageActor,
    transport: TransportActor,
    pending_effects: Vec<Effect>,
    window: Option<Arc<Window>>,
    renderer: Option<renderer::Renderer>,
    modifiers: ModifiersState,
    pointer_cell: Option<(u16, u16)>,
}

impl Runtime {
    fn new(
        options: LaunchTargetOptions,
        proxy: EventLoopProxy<RuntimeEvent>,
        app: App,
        storage: StorageActor,
        transport: TransportActor,
        pending_effects: Vec<Effect>,
    ) -> Self {
        Self {
            options,
            proxy,
            app,
            storage,
            transport,
            pending_effects,
            window: None,
            renderer: None,
            modifiers: ModifiersState::empty(),
            pointer_cell: None,
        }
    }

    fn create_window(
        &self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Arc<Window>, Box<dyn std::error::Error>> {
        let geometry = self.app.model().geometry;
        let size = renderer::logical_window_size_for_grid(geometry.width(), geometry.height());
        let mut attributes = WindowAttributes::default()
            .with_title("Nostrian Conquest - nc-helm")
            .with_resizable(true)
            .with_inner_size(size);
        match self.options.native.window_mode {
            NativeWindowMode::MaximizedWindow => {
                attributes = attributes.with_maximized(true);
            }
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
        let effects = self.app.dispatch(msg);
        self.pending_effects.extend(effects);
        self.process_effects(event_loop);
        self.sync_window_input_state();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        if self.app.model().should_quit {
            event_loop.exit();
        }
    }

    fn process_effects(&mut self, event_loop: &ActiveEventLoop) {
        while let Some(effect) = self.pending_effects.pop() {
            match effect {
                Effect::LoadBoot => {
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.storage.load_boot(tx);
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
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.storage
                        .create_identity(handle, password, relay_url, tx);
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::IdentityCreated(result));
                        }
                    });
                }
                Effect::Unlock { password } => {
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.storage.unlock(password, tx);
                    thread::spawn(move || {
                        if let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::Unlocked(result));
                        }
                    });
                }
                Effect::ConnectTransport { relay_url, nsec } => {
                    let proxy = self.proxy.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.transport.connect(relay_url, nsec, tx);
                    thread::spawn(move || {
                        while let Ok(result) = rx.recv() {
                            let _ = proxy.send_event(RuntimeEvent::LobbyUpdated(result));
                        }
                    });
                }
                Effect::Quit => event_loop.exit(),
            }
        }
    }

    fn sync_window_input_state(&self) {
        let Some(window) = &self.window else {
            return;
        };
        window.set_ime_allowed(self.app.model().wants_text_input());
        if self.app.model().wants_window_focus() {
            window.focus_window();
        }
    }

    fn sync_geometry_from_window(&mut self, window: Arc<Window>, event_loop: &ActiveEventLoop) {
        let scale = window.scale_factor();
        let size = window.inner_size();
        let (cols, rows) = renderer::terminal_grid_for_pixels(size.width, size.height, scale);
        self.dispatch(
            Msg::Resize(nc_ui::ScreenGeometry::new(
                usize::from(cols),
                usize::from(rows),
            )),
            event_loop,
        );
    }
}

impl ApplicationHandler<RuntimeEvent> for Runtime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
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
                    self.sync_geometry_from_window(window.clone(), event_loop);
                    self.window = Some(window);
                    self.renderer = Some(renderer);
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
            RuntimeEvent::BootLoaded(result) => self.dispatch(Msg::BootLoaded(result), event_loop),
            RuntimeEvent::IdentityCreated(result) => {
                self.dispatch(Msg::IdentityCreated(result), event_loop)
            }
            RuntimeEvent::Unlocked(result) => self.dispatch(Msg::Unlocked(result), event_loop),
            RuntimeEvent::LobbyUpdated(result) => {
                self.dispatch(Msg::LobbyUpdated(result), event_loop)
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
                        crate::input::KeyModifiers::NONE,
                    )),
                    event_loop,
                );
            }
            WindowEvent::Focused(focused) => {
                self.dispatch(Msg::FocusChanged(focused), event_loop);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(key) = key_event_from_winit(&event, self.modifiers) {
                    self.dispatch(Msg::Key(key), event_loop);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let (Some(window), Some(renderer)) = (&self.window, &self.renderer) {
                    self.pointer_cell = renderer.cell_position_at_pixel(window, position);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let button = match button {
                    WinitMouseButton::Left => Some(MouseButton::Left),
                    WinitMouseButton::Right => Some(MouseButton::Right),
                    WinitMouseButton::Middle => Some(MouseButton::Middle),
                    _ => None,
                };
                if let (Some(button), Some((column, row))) = (button, self.pointer_cell) {
                    self.dispatch(
                        Msg::Mouse(MouseEvent {
                            kind: if state.is_pressed() {
                                MouseEventKind::Down(button)
                            } else {
                                MouseEventKind::Up(button)
                            },
                            column,
                            row,
                            modifiers: key_modifiers_from_winit(self.modifiers),
                        }),
                        event_loop,
                    );
                }
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = self.window.clone() {
                    self.sync_geometry_from_window(window, event_loop);
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                    let buffer = self.app.view();
                    if let Err(err) = renderer.render(&buffer) {
                        eprintln!("nc-helm render error: {err}");
                        event_loop.exit();
                    } else {
                        window.pre_present_notify();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
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
