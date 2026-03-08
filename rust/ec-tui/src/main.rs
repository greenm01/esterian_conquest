use std::env;
use std::fs;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ec_data::{ConquestDat, FleetDat, PlanetDat, PlayerDat, SetupDat};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};

const EC_BLUE: Color = Color::Rgb(10, 32, 122);
const EC_BLUE_DARK: Color = Color::Rgb(7, 22, 84);
const EC_GOLD: Color = Color::Rgb(245, 208, 76);
const EC_CREAM: Color = Color::Rgb(243, 234, 206);
const EC_BLACK: Color = Color::Rgb(10, 10, 14);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppMode {
    Player,
    Util,
}

#[derive(Debug, Eq, PartialEq)]
struct CliOptions {
    mode: AppMode,
    dir: PathBuf,
}

#[derive(Debug)]
struct AppData {
    player: PlayerDat,
    planets: PlanetDat,
    setup: SetupDat,
    conquest: ConquestDat,
    fleets: Option<FleetDat>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusPane {
    Overview,
    Players,
    Fleets,
}

struct App {
    options: CliOptions,
    data: AppData,
    focus: FocusPane,
}

impl App {
    fn new(options: CliOptions, data: AppData) -> Self {
        Self {
            options,
            data,
            focus: FocusPane::Overview,
        }
    }

    fn title(&self) -> &'static str {
        match self.options.mode {
            AppMode::Player => "Esterian Conquest Player Client",
            AppMode::Util => "Esterian Conquest Utility Console",
        }
    }

    fn pane_title(&self, pane: FocusPane) -> &'static str {
        match (self.options.mode, pane) {
            (AppMode::Player, FocusPane::Overview) => "Overview",
            (AppMode::Player, FocusPane::Players) => "Players",
            (AppMode::Player, FocusPane::Fleets) => "Fleets",
            (AppMode::Util, FocusPane::Overview) => "Dashboard",
            (AppMode::Util, FocusPane::Players) => "Empire Control",
            (AppMode::Util, FocusPane::Fleets) => "Program & Port Setup",
        }
    }

    fn mode_hint(&self) -> &'static str {
        match self.options.mode {
            AppMode::Player => "Default player shell",
            AppMode::Util => "Modern admin UI over preserved ECUTIL behavior",
        }
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusPane::Overview => FocusPane::Players,
            FocusPane::Players => FocusPane::Fleets,
            FocusPane::Fleets => FocusPane::Overview,
        };
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_args(env::args().skip(1), env::current_dir()?)?;
    let options = resolve_game_dir(options);
    let data = load_app_data(&options.dir)?;

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let terminal_result = run_tui(App::new(options, data));
    disable_raw_mode()?;
    execute!(out, LeaveAlternateScreen)?;
    terminal_result
}

fn run_tui(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let result = tui_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn tui_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| draw(frame, app))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Tab => app.cycle_focus(),
                    KeyCode::Char('1') => app.focus = FocusPane::Overview,
                    KeyCode::Char('2') => app.focus = FocusPane::Players,
                    KeyCode::Char('3') => app.focus = FocusPane::Fleets,
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

fn draw(frame: &mut Frame, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().bg(EC_BLACK)),
        frame.area(),
    );

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    frame.render_widget(header(app), vertical[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(36), Constraint::Min(40)])
        .split(vertical[1]);

    let sidebar = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(8)])
        .split(body[0]);

    frame.render_widget(mode_menu(app), sidebar[0]);
    frame.render_widget(status_panel(app), sidebar[1]);

    match app.focus {
        FocusPane::Overview => frame.render_widget(overview_panel(app), body[1]),
        FocusPane::Players => frame.render_widget(players_panel(app), body[1]),
        FocusPane::Fleets => frame.render_widget(fleets_panel(app), body[1]),
    }

    frame.render_widget(footer(), vertical[2]);
}

fn header(app: &App) -> Paragraph<'static> {
    let mode = match app.options.mode {
        AppMode::Player => "player",
        AppMode::Util => "util",
    };
    Paragraph::new(vec![
        Line::from(Span::styled(
            app.title(),
            Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                format!("dir: {}  ", app.options.dir.display()),
                Style::default().fg(EC_CREAM),
            ),
            Span::styled(format!("mode: {mode}  "), Style::default().fg(EC_GOLD)),
            Span::styled(app.mode_hint(), Style::default().fg(EC_CREAM)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" ec-tui ", Style::default().fg(EC_GOLD)))
            .style(Style::default().bg(EC_BLUE).fg(EC_CREAM)),
    )
}

fn mode_menu(app: &App) -> List<'static> {
    let items = [
        (
            format!("1 {}", app.pane_title(FocusPane::Overview)),
            app.focus == FocusPane::Overview,
        ),
        (
            format!("2 {}", app.pane_title(FocusPane::Players)),
            app.focus == FocusPane::Players,
        ),
        (
            format!("3 {}", app.pane_title(FocusPane::Fleets)),
            app.focus == FocusPane::Fleets,
        ),
        ("Tab Cycle View".to_string(), false),
        ("Q Quit".to_string(), false),
    ]
    .into_iter()
    .map(|(label, selected)| {
        let style = if selected {
            Style::default()
                .fg(EC_BLACK)
                .bg(EC_GOLD)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(EC_CREAM)
        };
        ListItem::new(Line::from(Span::styled(label, style)))
    })
    .collect::<Vec<_>>();

    List::new(items).block(panel_block("Sections", false))
}

fn status_panel(app: &App) -> Paragraph<'static> {
    let setup = &app.data.setup;
    let conquest = &app.data.conquest;
    let lines = vec![
        Line::from(format!("Year: {}", conquest.game_year())),
        Line::from(format!("Players: {}", conquest.player_count())),
        Line::from(format!(
            "Maintenance: {:02x?}",
            conquest.maintenance_schedule_bytes()
        )),
        Line::from(format!(
            "Snoop: {}",
            if setup.snoop_enabled() { "Yes" } else { "No" }
        )),
        Line::from(format!(
            "Flow: {} / {} / {} / {}",
            on_off(setup.com_hardware_flow_control_enabled(0).unwrap_or(false)),
            on_off(setup.com_hardware_flow_control_enabled(1).unwrap_or(false)),
            on_off(setup.com_hardware_flow_control_enabled(2).unwrap_or(false)),
            on_off(setup.com_hardware_flow_control_enabled(3).unwrap_or(false)),
        )),
    ];
    Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(EC_CREAM))
        .block(panel_block("Status", false))
}

fn overview_panel(app: &App) -> Paragraph<'static> {
    if app.options.mode == AppMode::Util {
        return util_dashboard_panel(app);
    }

    let setup = &app.data.setup;
    let conquest = &app.data.conquest;
    let homeworlds = app
        .data
        .planets
        .records
        .iter()
        .filter(|record| record.is_named_homeworld_seed())
        .map(|record| format!("{:?}", record.coords_raw()))
        .collect::<Vec<_>>()
        .join(", ");

    let lines = vec![
        Line::from(Span::styled(
            "Gameplay UI is still a shell, but the preserved data model is live.",
            Style::default().fg(EC_GOLD),
        )),
        Line::from(""),
        Line::from(format!(
            "COM IRQs: [{}, {}, {}, {}]",
            setup.com_irq_raw(0).unwrap_or_default(),
            setup.com_irq_raw(1).unwrap_or_default(),
            setup.com_irq_raw(2).unwrap_or_default(),
            setup.com_irq_raw(3).unwrap_or_default(),
        )),
        Line::from(format!(
            "Hardware flow: [{}, {}, {}, {}]",
            yes_no(setup.com_hardware_flow_control_enabled(0).unwrap_or(false)),
            yes_no(setup.com_hardware_flow_control_enabled(1).unwrap_or(false)),
            yes_no(setup.com_hardware_flow_control_enabled(2).unwrap_or(false)),
            yes_no(setup.com_hardware_flow_control_enabled(3).unwrap_or(false)),
        )),
        Line::from(format!(
            "Timeouts: local={} remote={} max_key_gap={}m min_time={}m",
            yes_no(setup.local_timeout_enabled()),
            yes_no(setup.remote_timeout_enabled()),
            setup.max_time_between_keys_minutes_raw(),
            setup.minimum_time_granted_minutes_raw(),
        )),
        Line::from(format!(
            "Purge after: {} turn(s)  Autopilot after: {} turn(s)",
            setup.purge_after_turns_raw(),
            setup.autopilot_inactive_turns_raw(),
        )),
        Line::from(format!(
            "Control header words: {:04x?}",
            &conquest.header_words()[..8]
        )),
        Line::from(format!("Seed homeworlds: {homeworlds}")),
    ];

    Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(EC_CREAM))
        .block(panel_block(app.pane_title(FocusPane::Overview), app.focus == FocusPane::Overview))
}

fn util_dashboard_panel(app: &App) -> Paragraph<'static> {
    let setup = &app.data.setup;
    let conquest = &app.data.conquest;
    let lines = vec![
        Line::from(Span::styled(
            "Campaign Clock",
            Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "  Year {}  |  Players {}  |  Schedule {:02x?}",
            conquest.game_year(),
            conquest.player_count(),
            conquest.maintenance_schedule_bytes()
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Program Rules",
            Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "  Snoop {}  |  Purge {} turn(s)  |  Autopilot {} turn(s)",
            yes_no(setup.snoop_enabled()),
            setup.purge_after_turns_raw(),
            setup.autopilot_inactive_turns_raw()
        )),
        Line::from(format!(
            "  Local timeout {}  |  Remote timeout {}",
            yes_no(setup.local_timeout_enabled()),
            yes_no(setup.remote_timeout_enabled())
        )),
        Line::from(format!(
            "  Max key gap {}m  |  Minimum time {}m",
            setup.max_time_between_keys_minutes_raw(),
            setup.minimum_time_granted_minutes_raw()
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Port Setup",
            Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "  IRQs: COM1={}  COM2={}  COM3={}  COM4={}",
            setup.com_irq_raw(0).unwrap_or_default(),
            setup.com_irq_raw(1).unwrap_or_default(),
            setup.com_irq_raw(2).unwrap_or_default(),
            setup.com_irq_raw(3).unwrap_or_default()
        )),
        Line::from(format!(
            "  Flow: COM1={}  COM2={}  COM3={}  COM4={}",
            yes_no(setup.com_hardware_flow_control_enabled(0).unwrap_or(false)),
            yes_no(setup.com_hardware_flow_control_enabled(1).unwrap_or(false)),
            yes_no(setup.com_hardware_flow_control_enabled(2).unwrap_or(false)),
            yes_no(setup.com_hardware_flow_control_enabled(3).unwrap_or(false))
        )),
    ];

    Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(EC_CREAM))
        .block(panel_block(app.pane_title(FocusPane::Overview), app.focus == FocusPane::Overview))
}

fn players_panel(app: &App) -> List<'static> {
    let items = app
        .data
        .player
        .records
        .iter()
        .enumerate()
        .map(|(idx, record)| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    format!("Empire {:02}", idx + 1),
                    Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("  {}", record.ownership_summary())),
                Line::from(format!("  tax={} owner_mode={}", record.tax_rate(), record.owner_mode_raw())),
            ])
        })
        .collect::<Vec<_>>();

    List::new(items).block(panel_block(app.pane_title(FocusPane::Players), app.focus == FocusPane::Players))
}

fn fleets_panel(app: &App) -> Paragraph<'static> {
    if app.options.mode == AppMode::Util {
        return util_programs_panel(app);
    }

    let lines = match &app.data.fleets {
        Some(fleets) => fleets
            .records
            .chunks_exact(4)
            .enumerate()
            .flat_map(|(group_idx, group)| {
                let mut group_lines = vec![Line::from(Span::styled(
                    format!(
                        "Empire Block {}  home={:?}",
                        group_idx + 1,
                        group[0].home_system_coords_raw()
                    ),
                    Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
                ))];
                for record in group {
                    group_lines.push(Line::from(format!(
                        "  id={} slot={} ships={} order={}",
                        record.fleet_id(),
                        record.local_slot(),
                        record.ship_composition_summary(),
                        record.standing_order_summary()
                    )));
                }
                group_lines.push(Line::from(""));
                group_lines
            })
            .collect::<Vec<_>>(),
        None => vec![Line::from("FLEETS.DAT does not match the initialized 16x54 layout.")],
    };

    Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(EC_CREAM))
        .block(panel_block(app.pane_title(FocusPane::Fleets), app.focus == FocusPane::Fleets))
}

fn footer() -> Paragraph<'static> {
    Paragraph::new("Keys: 1/2/3 switch sections, Tab cycles, q exits. ec-cli remains the scripting interface; ec-tui is the modern interactive shell.")
        .style(Style::default().fg(EC_CREAM))
        .block(panel_block("Help", false))
        .wrap(Wrap { trim: true })
}

fn util_programs_panel(app: &App) -> Paragraph<'static> {
    let setup = &app.data.setup;
    let lines = vec![
        Line::from(Span::styled(
            "Setup Programs",
            Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "  Purge messages & reports after: {} turn(s)",
            setup.purge_after_turns_raw()
        )),
        Line::from(format!(
            "  Autopilot inactive empires after: {} turn(s)",
            setup.autopilot_inactive_turns_raw()
        )),
        Line::from(format!(
            "  Snoop Enabled: {}",
            yes_no(setup.snoop_enabled())
        )),
        Line::from(format!(
            "  Local user timeout: {}",
            yes_no(setup.local_timeout_enabled())
        )),
        Line::from(format!(
            "  Remote user timeout: {}",
            yes_no(setup.remote_timeout_enabled())
        )),
        Line::from(format!(
            "  Maximum time between key strokes: {} minute(s)",
            setup.max_time_between_keys_minutes_raw()
        )),
        Line::from(format!(
            "  Minimum time granted: {} minute(s)",
            setup.minimum_time_granted_minutes_raw()
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Modem / COM Ports",
            Style::default().fg(EC_GOLD).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "  COM1 IRQ {}  Flow {}",
            setup.com_irq_raw(0).unwrap_or_default(),
            yes_no(setup.com_hardware_flow_control_enabled(0).unwrap_or(false))
        )),
        Line::from(format!(
            "  COM2 IRQ {}  Flow {}",
            setup.com_irq_raw(1).unwrap_or_default(),
            yes_no(setup.com_hardware_flow_control_enabled(1).unwrap_or(false))
        )),
        Line::from(format!(
            "  COM3 IRQ {}  Flow {}",
            setup.com_irq_raw(2).unwrap_or_default(),
            yes_no(setup.com_hardware_flow_control_enabled(2).unwrap_or(false))
        )),
        Line::from(format!(
            "  COM4 IRQ {}  Flow {}",
            setup.com_irq_raw(3).unwrap_or_default(),
            yes_no(setup.com_hardware_flow_control_enabled(3).unwrap_or(false))
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Next: make these panels editable directly in the TUI.",
            Style::default().fg(EC_CREAM),
        )),
    ];

    Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(EC_CREAM))
        .block(panel_block(app.pane_title(FocusPane::Fleets), app.focus == FocusPane::Fleets))
}

fn panel_block(title: &'static str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(EC_GOLD).bg(EC_BLUE_DARK)
    } else {
        Style::default().fg(EC_CREAM).bg(EC_BLUE_DARK)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .style(Style::default().bg(EC_BLUE_DARK))
        .title(Span::styled(
            format!(" {title} "),
            style.add_modifier(Modifier::BOLD),
        ))
}

fn load_app_data(dir: &Path) -> Result<AppData, Box<dyn std::error::Error>> {
    let player = PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?)?;
    let planets = PlanetDat::parse(&fs::read(dir.join("PLANETS.DAT"))?)?;
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;
    let fleets = fs::read(dir.join("FLEETS.DAT"))
        .ok()
        .and_then(|bytes| FleetDat::parse(&bytes).ok());

    Ok(AppData {
        player,
        planets,
        setup,
        conquest,
        fleets,
    })
}

fn resolve_game_dir(options: CliOptions) -> CliOptions {
    if looks_like_game_dir(&options.dir) {
        return options;
    }

    let repo_default = repo_root().join("original/v1.5");
    if looks_like_game_dir(&repo_default) {
        CliOptions {
            mode: options.mode,
            dir: repo_default,
        }
    } else {
        options
    }
}

fn looks_like_game_dir(dir: &Path) -> bool {
    ["PLAYER.DAT", "PLANETS.DAT", "SETUP.DAT", "CONQUEST.DAT"]
        .into_iter()
        .all(|name| dir.join(name).is_file())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_args(
    mut args: impl Iterator<Item = String>,
    current_dir: PathBuf,
) -> Result<CliOptions, Box<dyn std::error::Error>> {
    match args.next() {
        None => Ok(CliOptions {
            mode: AppMode::Player,
            dir: current_dir,
        }),
        Some(first) if first == "util" => Ok(CliOptions {
            mode: AppMode::Util,
            dir: args.next().map(PathBuf::from).unwrap_or(current_dir),
        }),
        Some(first) => Ok(CliOptions {
            mode: AppMode::Player,
            dir: PathBuf::from(first),
        }),
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_to_player_mode_and_current_dir() {
        let cwd = PathBuf::from("/tmp/ecgame");
        let parsed = parse_args(std::iter::empty(), cwd.clone()).unwrap();
        assert_eq!(
            parsed,
            CliOptions {
                mode: AppMode::Player,
                dir: cwd,
            }
        );
    }

    #[test]
    fn parse_args_supports_util_subcommand_and_optional_dir() {
        let cwd = PathBuf::from("/tmp/ecgame");
        let parsed = parse_args(
            ["util", "/tmp/ecutil"].into_iter().map(String::from),
            cwd,
        )
        .unwrap();
        assert_eq!(
            parsed,
            CliOptions {
                mode: AppMode::Util,
                dir: PathBuf::from("/tmp/ecutil"),
            }
        );
    }

    #[test]
    fn resolve_game_dir_falls_back_to_repo_original_snapshot() {
        let options = CliOptions {
            mode: AppMode::Util,
            dir: PathBuf::from("/tmp/not-a-game-dir"),
        };
        let resolved = resolve_game_dir(options);
        assert!(resolved.dir.ends_with("original/v1.5"));
    }
}
