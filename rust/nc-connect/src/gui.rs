mod app;
mod clipboard;
mod font;
mod input;
mod render;
mod terminal;

use std::env;

use crate::config::load_config;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, WindowBuilder};

use app::App;
use render::WindowRenderer;
use crate::shell::{OUTER_HEIGHT, OUTER_WIDTH};

pub(crate) const TERM_COLS: u16 = 80;
pub(crate) const TERM_ROWS: u16 = 25;
pub(crate) const CELL_WIDTH: usize = 10;
pub(crate) const CELL_HEIGHT: usize = 18;
const WINDOW_TITLE: &str = "Nostrian Conquest";

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    parse_launch_args(env::args().skip(1))?;
    init_gui_logging();
    let event_loop = EventLoop::new()?;
    let logical_width = (OUTER_WIDTH * CELL_WIDTH) as f64;
    let logical_height = (OUTER_HEIGHT * CELL_HEIGHT) as f64;
    let window = Box::new(
        WindowBuilder::new()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(logical_width, logical_height))
            .with_resizable(true)
            .build(&event_loop)?,
    );
    let window: &'static winit::window::Window = Box::leak(window);
    let mut renderer = WindowRenderer::new(window)?;
    let initial_size = window.inner_size();
    let mut app = App::new()?;
    app.update_window_size(initial_size.width, initial_size.height)?;
    let mut modifiers = ModifiersState::empty();
    let mut fullscreen_live = false;
    let mut windowed_size = initial_size;

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
                WindowEvent::Focused(focused) => {
                    tracing::debug!(focused, "nc-connect window focus changed");
                }
                WindowEvent::Occluded(occluded) => {
                    tracing::debug!(occluded, "nc-connect window occlusion changed");
                }
                WindowEvent::Resized(size) => {
                    if !fullscreen_live {
                        windowed_size = size;
                    }
                    if let Err(err) = app.update_window_size(size.width, size.height) {
                        show_fatal_error(&err.to_string());
                        elwt.exit();
                    } else {
                        window.request_redraw();
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let size = window.inner_size();
                    if !fullscreen_live {
                        windowed_size = size;
                    }
                    if let Err(err) = app.update_window_size(size.width, size.height) {
                        show_fatal_error(&err.to_string());
                        elwt.exit();
                    } else {
                        window.request_redraw();
                    }
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
                        sync_window_mode(window, &app, &mut fullscreen_live, &mut windowed_size);
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
                        sync_window_mode(window, &app, &mut fullscreen_live, &mut windowed_size);
                        window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    if let Err(err) = renderer.render(&app.current_buffer(), size.width, size.height)
                    {
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
                sync_window_mode(window, &app, &mut fullscreen_live, &mut windowed_size);
                if app.needs_redraw {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}

pub(crate) fn terminal_grid_for_pixels(pixel_width: u32, pixel_height: u32) -> (u16, u16) {
    let cols = (pixel_width.max(1) as usize / CELL_WIDTH).max(1);
    let rows = (pixel_height.max(1) as usize / CELL_HEIGHT).max(1);
    (
        cols.min(u16::MAX as usize) as u16,
        rows.min(u16::MAX as usize) as u16,
    )
}

fn sync_window_mode(
    window: &winit::window::Window,
    app: &App,
    fullscreen_live: &mut bool,
    windowed_size: &mut winit::dpi::PhysicalSize<u32>,
) {
    if app.in_live_session() {
        if !*fullscreen_live {
            *windowed_size = window.inner_size();
            window.set_fullscreen(Some(Fullscreen::Borderless(window.current_monitor())));
            *fullscreen_live = true;
        }
        return;
    }

    if *fullscreen_live {
        window.set_fullscreen(None);
        let _ = window.request_inner_size(*windowed_size);
        *fullscreen_live = false;
    }
}

fn init_gui_logging() {
    let Ok(config) = load_config() else {
        return;
    };
    let Some(log_file) = config.log_file.as_deref() else {
        return;
    };
    let log_level = config.effective_log_level();
    match nc_log::init_file_logging(log_file, log_level) {
        Ok(()) => tracing::info!(
            log_file = %log_file.display(),
            level = ?log_level,
            "nc-connect GUI logging initialized"
        ),
        Err(err) => eprintln!(
            "warning: unable to initialize nc-connect GUI logging at {}: {}",
            log_file.display(),
            err
        ),
    }
}

fn parse_launch_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    match args.next() {
        None => Ok(()),
        Some(other) => Err(format!(
            "nc-connect does not accept command-line arguments.\n\nOpen the app, press N, and paste your invite code.\n\nUnrecognized argument: {other}"
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_launch_args;

    #[test]
    fn parse_launch_args_accepts_no_arguments() {
        assert!(parse_launch_args(Vec::<String>::new().into_iter()).is_ok());
    }

    #[test]
    fn parse_launch_args_rejects_join_argument() {
        let err = parse_launch_args(
            vec![
                "--join".to_string(),
                "amber-river@relay.example.com".to_string(),
            ]
            .into_iter(),
        )
        .expect_err("desktop nc-connect should reject command-line args");
        assert!(
            err.to_string()
                .contains("nc-connect does not accept command-line arguments")
        );
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
