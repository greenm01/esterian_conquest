mod app;
mod clipboard;
mod font;
mod input;
mod render;
mod terminal;

use std::env;

use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::WindowBuilder;

use app::{App, LaunchIntent};
use render::WindowRenderer;

pub(crate) const TERM_COLS: u16 = 80;
pub(crate) const TERM_ROWS: u16 = 25;
pub(crate) const CELL_WIDTH: usize = 10;
pub(crate) const CELL_HEIGHT: usize = 18;
const WINDOW_TITLE: &str = "Nostrian Conquest";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let launch_intent = parse_launch_intent()?;
    let event_loop = EventLoop::new()?;
    let logical_width = f64::from(TERM_COLS) * CELL_WIDTH as f64;
    let logical_height = f64::from(TERM_ROWS) * CELL_HEIGHT as f64;
    let window = Box::new(
        WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(logical_width, logical_height))
            .with_resizable(false)
            .build(&event_loop)?,
    );
    let window: &'static winit::window::Window = Box::leak(window);
    let mut renderer = WindowRenderer::new(window)?;
    let mut app = App::new(launch_intent)?;
    let mut modifiers = ModifiersState::empty();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    app.request_close();
                    elwt.exit();
                }
                WindowEvent::ModifiersChanged(new_modifiers) => {
                    modifiers = new_modifiers.state();
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Err(err) = app.handle_mouse_move(position) {
                        show_fatal_error(&err.to_string());
                        elwt.exit();
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    let pressed = state == ElementState::Pressed;
                    if let Err(err) = app.handle_mouse_button(button, pressed) {
                        show_fatal_error(&err.to_string());
                        elwt.exit();
                    } else if app.needs_redraw {
                        window.request_redraw();
                    }
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if let Err(err) = app.handle_key_event(&event, modifiers) {
                        show_fatal_error(&err.to_string());
                        elwt.exit();
                    } else if app.exit_requested {
                        elwt.exit();
                    } else if app.needs_redraw {
                        window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Err(err) = renderer.render(&app.current_buffer()) {
                        show_fatal_error(&format!("unable to render nc-connect window: {err}"));
                        elwt.exit();
                    } else {
                        app.needs_redraw = false;
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                if let Err(err) = app.tick() {
                    show_fatal_error(&err.to_string());
                    elwt.exit();
                    return;
                }
                if app.exit_requested {
                    elwt.exit();
                    return;
                }
                elwt.set_control_flow(app.control_flow());
                if app.needs_redraw {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

fn parse_launch_intent() -> Result<LaunchIntent, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    match args.next() {
        None => Ok(LaunchIntent::Normal),
        Some(flag) if flag == "--join" => {
            let invite = args.next().ok_or("--join requires an invite code")?;
            if let Some(extra) = args.next() {
                return Err(format!(
                    "unexpected extra argument after --join: {extra}"
                )
                .into());
            }
            Ok(LaunchIntent::Join(invite))
        }
        Some(other) => Err(format!(
            "nc-connect supports only no arguments or --join <invite>.\n\nUnrecognized argument: {other}"
        )
        .into()),
    }
}

pub fn show_fatal_error(message: &str) {
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("error: {message}");
    }

    #[cfg(target_os = "windows")]
    use std::ffi::OsStr;
    #[cfg(target_os = "windows")]
    use std::iter;
    #[cfg(target_os = "windows")]
    use std::os::windows::ffi::OsStrExt;

    #[cfg(target_os = "windows")]
    use winapi::um::winuser::{MB_ICONERROR, MB_OK, MessageBoxW};

    #[cfg(target_os = "windows")]
    let title: Vec<u16> = OsStr::new("nc-connect")
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    #[cfg(target_os = "windows")]
    let body: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    #[cfg(target_os = "windows")]
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
