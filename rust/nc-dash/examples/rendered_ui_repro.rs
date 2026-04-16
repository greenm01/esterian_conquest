use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nc_dash::{RenderedUi, blit_rendered_ui};
use ratatui::Terminal;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Widget;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
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
enum BackendPreference {
    Auto,
    Wayland,
    X11,
}

impl BackendPreference {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "wayland" => Some(Self::Wayland),
            "x11" => Some(Self::X11),
            _ => None,
        }
    }
}

struct Options {
    backend: BackendPreference,
}

struct AppState {
    first_frame_at: Option<Instant>,
    second_frame_requested: bool,
    clicked: bool,
    frame_count: u32,
}

impl AppState {
    fn new() -> Self {
        Self {
            first_frame_at: None,
            second_frame_requested: false,
            clicked: false,
            frame_count: 0,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_args()?;
    eprintln!(
        "rendered_ui_repro: backend={}, pid={}",
        backend_label(options.backend),
        std::process::id()
    );

    let mut builder = EventLoopBuilder::new();
    apply_backend_preference(&mut builder, options.backend)?;
    let event_loop = builder.build()?;

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("nc-dash RenderedUi repro")
            .with_inner_size(LogicalSize::new(1200.0, 720.0))
            .with_resizable(true)
            .build(&event_loop)?,
    );

    let mut terminal = build_terminal(window.clone())?;
    let mut app = AppState::new();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    eprintln!("rendered_ui_repro: close requested");
                    elwt.exit();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state == ElementState::Pressed
                        && matches!(event.logical_key, Key::Named(NamedKey::Escape))
                    {
                        eprintln!("rendered_ui_repro: escape pressed");
                        elwt.exit();
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: MouseButton::Left,
                    ..
                } => {
                    app.clicked = true;
                    eprintln!("rendered_ui_repro: left click released, requesting redraw");
                    window.request_redraw();
                }
                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    terminal.backend_mut().resize(size.width.max(1), size.height.max(1));
                    let rendered = build_rendered_ui(size, &app);
                    if let Err(err) = terminal.draw(|frame| {
                        let area = frame.area();
                        blit_rendered_ui(frame.buffer_mut(), area, &rendered, Style::default())
                    }) {
                        eprintln!("rendered_ui_repro: draw failed: {err}");
                        elwt.exit();
                        return;
                    }
                    app.frame_count += 1;
                    if app.first_frame_at.is_none() {
                        app.first_frame_at = Some(Instant::now());
                        eprintln!("rendered_ui_repro: first frame rendered");
                    } else {
                        eprintln!("rendered_ui_repro: frame {} rendered", app.frame_count);
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                if app.first_frame_at.is_none() {
                    window.request_redraw();
                    return;
                }
                if !app.second_frame_requested
                    && app
                        .first_frame_at
                        .is_some_and(|started| started.elapsed() >= Duration::from_millis(750))
                {
                    app.second_frame_requested = true;
                    eprintln!("rendered_ui_repro: timer requesting second redraw");
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

fn build_rendered_ui(size: winit::dpi::PhysicalSize<u32>, app: &AppState) -> RenderedUi {
    let cols = (size.width.max(1) / 10).max(1) as u16;
    let rows = (size.height.max(1) / 18).max(1) as u16;
    let area = Rect::new(0, 0, cols, rows);
    let mut buffer = Buffer::empty(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(1), Constraint::Length(5)])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "RenderedUi + blit_rendered_ui repro",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!("frames rendered: {}", app.frame_count)),
        Line::from(format!(
            "clicked: {}",
            if app.clicked { "yes" } else { "no" }
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title("STATUS")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    let body = Paragraph::new(vec![
        Line::from("This example builds a Buffer, wraps it in RenderedUi,"),
        Line::from("and presents it through blit_rendered_ui."),
        Line::from("It redraws on timer and left-click release."),
        Line::from("Esc exits."),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title("PATH UNDER TEST")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White)),
    );

    let footer = Paragraph::new(vec![
        Line::from("If this crashes but native_wayland_repro does not,"),
        Line::from("the bug is in RenderedUi/blit or their surrounding usage."),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title("EXPECTATION")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    header.render(chunks[0], &mut buffer);
    body.render(chunks[1], &mut buffer);
    footer.render(chunks[2], &mut buffer);

    let cursor_col = (cols / 2).min(cols.saturating_sub(1));
    let cursor_row = (rows / 2).min(rows.saturating_sub(1));
    RenderedUi::new(buffer).with_cursor(
        Some((cursor_col, cursor_row)),
        Style::default().fg(Color::Black).bg(Color::White),
    )
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

fn parse_args() -> Result<Options, Box<dyn std::error::Error>> {
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
                println!("Usage: cargo run -p nc-dash --example rendered_ui_repro -- [--backend auto|wayland|x11]");
                std::process::exit(0);
            }
            other => {
                return Err(format!("unrecognized argument: {other}").into());
            }
        }
    }
    Ok(Options { backend })
}

fn backend_label(backend: BackendPreference) -> &'static str {
    match backend {
        BackendPreference::Auto => "auto",
        BackendPreference::Wayland => "wayland",
        BackendPreference::X11 => "x11",
    }
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
