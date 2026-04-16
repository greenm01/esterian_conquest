use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nc_dash::{RenderedUi, blit_rendered_ui};
use ratatui::Terminal;
use ratatui::style::Style;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopBuilder};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

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

const PRIMARY_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Regular.ttf"
));
const PRIMARY_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Bold.ttf"
));
const PRIMARY_ITALIC_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/JetBrainsMono-Italic.ttf"
));
const FALLBACK_REGULAR_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Regular.ttf"
));
const FALLBACK_BOLD_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../nc-connect/assets/fonts/NotoSansMono-Bold.ttf"
));

type NativeTerminal = Terminal<WgpuBackend<'static, 'static>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPreference {
    Auto,
    Wayland,
    X11,
}

impl BackendPreference {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "wayland" => Some(Self::Wayland),
            "x11" => Some(Self::X11),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Wayland => "wayland",
            Self::X11 => "x11",
        }
    }
}

pub struct Options {
    pub backend: BackendPreference,
}

pub fn parse_args() -> Result<Options, Box<dyn std::error::Error>> {
    let mut backend = BackendPreference::Auto;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--backend" => {
                let Some(value) = args.next() else {
                    return Err("--backend requires one of: auto, wayland, x11".into());
                };
                backend = BackendPreference::parse(&value)
                    .ok_or("--backend must be one of: auto, wayland, x11")?;
            }
            "--help" | "-h" => {
                return Err("help requested".into());
            }
            other => {
                return Err(format!("unrecognized argument: {other}").into());
            }
        }
    }
    Ok(Options { backend })
}

pub fn print_usage(example_name: &str) {
    println!(
        "Usage: cargo run -p nc-dash --example {example_name} -- [--backend auto|wayland|x11]"
    );
}

#[allow(dead_code)]
pub fn run_rendered_ui_repro(
    title: &str,
    backend: BackendPreference,
    mut render_ui: impl FnMut() -> Result<RenderedUi, Box<dyn std::error::Error>> + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    run_stateful_rendered_ui_repro(title, backend, (), move |()| render_ui(), |_state, _step| None)
}

pub fn run_stateful_rendered_ui_repro<S: 'static>(
    title: &str,
    backend: BackendPreference,
    state: S,
    render_ui: impl FnMut(&mut S) -> Result<RenderedUi, Box<dyn std::error::Error>> + 'static,
    run_step: impl FnMut(&mut S, usize) -> Option<&'static str> + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!(
        "{}: backend={}, pid={}",
        title,
        backend.label(),
        std::process::id()
    );

    let mut event_loop_builder = EventLoop::builder();
    apply_backend_preference(&mut event_loop_builder, backend)?;
    let event_loop = event_loop_builder.build()?;

    let window_attrs = Window::default_attributes()
        .with_title(title.to_string())
        .with_inner_size(LogicalSize::new(1200.0, 720.0))
        .with_resizable(true);

    struct ReproHandler<S: 'static> {
        title: String,
        window_attrs: WindowAttributes,
        state: S,
        render_ui: Box<dyn FnMut(&mut S) -> Result<RenderedUi, Box<dyn std::error::Error>>>,
        run_step: Box<dyn FnMut(&mut S, usize) -> Option<&'static str>>,
        window: Option<Arc<Window>>,
        terminal: Option<NativeTerminal>,
        first_frame_at: Option<Instant>,
        next_step_at: Option<Instant>,
        next_step_index: usize,
        frame_count: u32,
    }

    impl<S: 'static> ApplicationHandler for ReproHandler<S> {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let window = match event_loop.create_window(self.window_attrs.clone()) {
                Ok(w) => Arc::new(w),
                Err(err) => {
                    eprintln!("{}: failed to create window: {err}", self.title);
                    event_loop.exit();
                    return;
                }
            };
            let terminal = match build_terminal(window.clone()) {
                Ok(t) => t,
                Err(err) => {
                    eprintln!("{}: failed to build terminal: {err}", self.title);
                    event_loop.exit();
                    return;
                }
            };
            self.window = Some(window);
            self.terminal = Some(terminal);
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            let (Some(window), Some(terminal)) =
                (self.window.as_ref(), self.terminal.as_mut())
            else {
                return;
            };
            event_loop.set_control_flow(ControlFlow::Wait);
            match event {
                WindowEvent::CloseRequested => {
                    eprintln!("{}: close requested", self.title);
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Pressed
                        && matches!(event.logical_key, Key::Named(NamedKey::Escape))
                    {
                        eprintln!("{}: escape pressed", self.title);
                        event_loop.exit();
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: MouseButton::Left,
                    ..
                } => {
                    eprintln!("{}: left click released, requesting redraw", self.title);
                    window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    terminal.backend_mut().resize(size.width.max(1), size.height.max(1));
                    let rendered = match (self.render_ui)(&mut self.state) {
                        Ok(rendered) => rendered,
                        Err(err) => {
                            eprintln!("{}: render_ui failed: {err}", self.title);
                            event_loop.exit();
                            return;
                        }
                    };
                    if let Err(err) = terminal.draw(|frame| {
                        let area = frame.area();
                        blit_rendered_ui(frame.buffer_mut(), area, &rendered, Style::default())
                    }) {
                        eprintln!("{}: draw failed: {err}", self.title);
                        event_loop.exit();
                        return;
                    }
                    self.frame_count += 1;
                    if self.first_frame_at.is_none() {
                        self.first_frame_at = Some(Instant::now());
                        self.next_step_at = self
                            .first_frame_at
                            .map(|started| started + Duration::from_millis(500));
                        eprintln!("{}: first frame rendered", self.title);
                    } else {
                        eprintln!("{}: frame {} rendered", self.title, self.frame_count);
                    }
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            let Some(window) = self.window.as_ref() else {
                return;
            };
            if self.first_frame_at.is_none() {
                window.request_redraw();
                return;
            }
            if self
                .next_step_at
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                let current_step = self.next_step_index;
                self.next_step_index += 1;
                if let Some(label) = (self.run_step)(&mut self.state, current_step) {
                    eprintln!(
                        "{}: step {} -> {}",
                        self.title,
                        current_step + 1,
                        label
                    );
                    self.next_step_at = Some(Instant::now() + Duration::from_millis(500));
                } else {
                    eprintln!("{}: scripted steps complete", self.title);
                    self.next_step_at = None;
                }
                window.request_redraw();
            }
            let _ = event_loop;
        }
    }

    let mut handler = ReproHandler {
        title: title.to_string(),
        window_attrs,
        state,
        render_ui: Box::new(render_ui),
        run_step: Box::new(run_step),
        window: None,
        terminal: None,
        first_frame_at: None,
        next_step_at: None,
        next_step_index: 0,
        frame_count: 0,
    };
    event_loop.run_app(&mut handler)?;

    Ok(())
}

fn build_terminal(
    window: Arc<winit::window::Window>,
) -> Result<NativeTerminal, Box<dyn std::error::Error>> {
    let size = window.inner_size();
    let primary_regular =
        Font::new(PRIMARY_REGULAR_FONT).ok_or("unable to load primary regular font")?;
    let primary_bold = Font::new(PRIMARY_BOLD_FONT).ok_or("unable to load primary bold font")?;
    let primary_italic =
        Font::new(PRIMARY_ITALIC_FONT).ok_or("unable to load primary italic font")?;
    let fallback_regular =
        Font::new(FALLBACK_REGULAR_FONT).ok_or("unable to load fallback regular font")?;
    let fallback_bold =
        Font::new(FALLBACK_BOLD_FONT).ok_or("unable to load fallback bold font")?;
    let backend = pollster::block_on(
        Builder::from_font(primary_regular)
            .with_font_size_px(18)
            .with_bold_fonts([primary_bold, fallback_bold])
            .with_italic_fonts([primary_italic])
            .with_regular_fonts([fallback_regular])
            .with_width_and_height(Dimensions {
                width: NonZeroU32::new(size.width.max(1)).ok_or("window width must be non-zero")?,
                height: NonZeroU32::new(size.height.max(1))
                    .ok_or("window height must be non-zero")?,
            })
            .build_with_target(window),
    )?;
    Ok(Terminal::new(backend)?)
}

fn apply_backend_preference(
    builder: &mut EventLoopBuilder<()>,
    backend: BackendPreference,
) -> Result<(), Box<dyn std::error::Error>> {
    match backend {
        BackendPreference::Auto => Ok(()),
        BackendPreference::Wayland => apply_wayland_backend(builder),
        BackendPreference::X11 => apply_x11_backend(builder),
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
