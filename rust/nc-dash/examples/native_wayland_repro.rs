use std::sync::Arc;
use std::time::{Duration, Instant};

use nc_dash::build_native_terminal_for_repro;
use ratatui::Terminal;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui_wgpu::WgpuBackend;
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

    fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Wayland => "wayland",
            Self::X11 => "x11",
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

struct Handler {
    window_attrs: WindowAttributes,
    window: Option<Arc<Window>>,
    terminal: Option<NativeTerminal>,
    app: AppState,
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(self.window_attrs.clone()) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                eprintln!("native_wayland_repro: failed to create window: {err}");
                event_loop.exit();
                return;
            }
        };
        let terminal = match build_terminal(window.clone()) {
            Ok(t) => t,
            Err(err) => {
                eprintln!("native_wayland_repro: failed to build terminal: {err}");
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
        let (Some(window), Some(terminal)) = (self.window.as_ref(), self.terminal.as_mut()) else {
            return;
        };
        event_loop.set_control_flow(ControlFlow::Wait);
        match event {
            WindowEvent::CloseRequested => {
                eprintln!("native_wayland_repro: close requested");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && matches!(event.logical_key, Key::Named(NamedKey::Escape))
                {
                    eprintln!("native_wayland_repro: escape pressed");
                    event_loop.exit();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.app.clicked = true;
                eprintln!("native_wayland_repro: left click released, requesting redraw");
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let size = window.inner_size();
                terminal
                    .backend_mut()
                    .resize(size.width.max(1), size.height.max(1));
                if let Err(err) = terminal.draw(|frame| render_frame(frame, &self.app)) {
                    eprintln!("native_wayland_repro: draw failed: {err}");
                    event_loop.exit();
                    return;
                }
                self.app.frame_count += 1;
                if self.app.first_frame_at.is_none() {
                    self.app.first_frame_at = Some(Instant::now());
                    eprintln!("native_wayland_repro: first frame rendered");
                } else {
                    eprintln!(
                        "native_wayland_repro: frame {} rendered",
                        self.app.frame_count
                    );
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if self.app.first_frame_at.is_none() {
            window.request_redraw();
            return;
        }
        if !self.app.second_frame_requested
            && self
                .app
                .first_frame_at
                .is_some_and(|started| started.elapsed() >= Duration::from_millis(750))
        {
            self.app.second_frame_requested = true;
            eprintln!("native_wayland_repro: timer requesting second redraw");
            window.request_redraw();
        }
        let _ = event_loop;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_args()?;
    eprintln!(
        "native_wayland_repro: backend={}, pid={}",
        options.backend.label(),
        std::process::id()
    );

    let mut event_loop_builder = EventLoop::builder();
    apply_backend_preference(&mut event_loop_builder, options.backend)?;
    let event_loop = event_loop_builder.build()?;

    let window_attrs = Window::default_attributes()
        .with_title("nc-dash native repro")
        .with_inner_size(LogicalSize::new(1200.0, 720.0))
        .with_resizable(true);

    let mut handler = Handler {
        window_attrs,
        window: None,
        terminal: None,
        app: AppState::new(),
    };
    event_loop.run_app(&mut handler)?;

    Ok(())
}

fn render_frame(frame: &mut ratatui::Frame<'_>, app: &AppState) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(1),
            Constraint::Length(5),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "winit + ratatui-wgpu repro",
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
        Line::from("This example draws once immediately,"),
        Line::from("again after 750ms,"),
        Line::from("and again on left-click release."),
        Line::from("Esc exits."),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title("INSTRUCTIONS")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White)),
    );

    let footer = Paragraph::new(vec![
        Line::from("If this crashes after the second frame,"),
        Line::from("the issue is upstream of nc-dash UI logic."),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .title("EXPECTATION")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(header, chunks[0]);
    frame.render_widget(body, chunks[1]);
    frame.render_widget(footer, chunks[2]);
}

fn build_terminal(
    window: Arc<winit::window::Window>,
) -> Result<NativeTerminal, Box<dyn std::error::Error>> {
    build_native_terminal_for_repro(window)
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
                println!(
                    "Usage: cargo run -p nc-dash --example native_wayland_repro -- [--backend auto|wayland|x11]"
                );
                std::process::exit(0);
            }
            other => {
                return Err(format!("unrecognized argument: {other}").into());
            }
        }
    }
    Ok(Options { backend })
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
