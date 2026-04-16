use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nc_dash::{RenderedUi, blit_rendered_ui};
use ratatui::Terminal;
use ratatui::style::Style;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

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
    mut state: S,
    mut render_ui: impl FnMut(&mut S) -> Result<RenderedUi, Box<dyn std::error::Error>> + 'static,
    mut run_step: impl FnMut(&mut S, usize) -> Option<&'static str> + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!(
        "{}: backend={}, pid={}",
        title,
        backend.label(),
        std::process::id()
    );

    let mut builder = EventLoopBuilder::new();
    apply_backend_preference(&mut builder, backend)?;
    let event_loop = builder.build()?;

    let window = Arc::new(
        WindowBuilder::new()
            .with_title(title)
            .with_inner_size(LogicalSize::new(1200.0, 720.0))
            .with_resizable(true)
            .build(&event_loop)?,
    );

    let mut terminal = build_terminal(window.clone())?;
    let mut first_frame_at: Option<Instant> = None;
    let mut next_step_at: Option<Instant> = None;
    let mut next_step_index = 0usize;
    let mut frame_count = 0u32;

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    eprintln!("{title}: close requested");
                    elwt.exit();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Pressed
                        && matches!(event.logical_key, Key::Named(NamedKey::Escape))
                    {
                        eprintln!("{title}: escape pressed");
                        elwt.exit();
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: MouseButton::Left,
                    ..
                } => {
                    eprintln!("{title}: left click released, requesting redraw");
                    window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    terminal.backend_mut().resize(size.width.max(1), size.height.max(1));
                    let rendered = match render_ui(&mut state) {
                        Ok(rendered) => rendered,
                        Err(err) => {
                            eprintln!("{title}: render_ui failed: {err}");
                            elwt.exit();
                            return;
                        }
                    };
                    if let Err(err) = terminal.draw(|frame| {
                        let area = frame.area();
                        blit_rendered_ui(frame.buffer_mut(), area, &rendered, Style::default())
                    }) {
                        eprintln!("{title}: draw failed: {err}");
                        elwt.exit();
                        return;
                    }
                    frame_count += 1;
                    if first_frame_at.is_none() {
                        first_frame_at = Some(Instant::now());
                        next_step_at = first_frame_at.map(|started| started + Duration::from_millis(500));
                        eprintln!("{title}: first frame rendered");
                    } else {
                        eprintln!("{title}: frame {frame_count} rendered");
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                if first_frame_at.is_none() {
                    window.request_redraw();
                    return;
                }
                if next_step_at.is_some_and(|deadline| Instant::now() >= deadline) {
                    let current_step = next_step_index;
                    next_step_index += 1;
                    if let Some(label) = run_step(&mut state, current_step) {
                        eprintln!("{title}: step {} -> {}", current_step + 1, label);
                        next_step_at = Some(Instant::now() + Duration::from_millis(500));
                    } else {
                        eprintln!("{title}: scripted steps complete");
                        next_step_at = None;
                    }
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })?;

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
