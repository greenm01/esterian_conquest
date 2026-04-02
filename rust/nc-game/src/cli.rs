use std::io::IsTerminal;
use std::path::PathBuf;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::app::{App, AppConfig, AppOutcome, apply_action};
use crate::dropfile;
use crate::screen::ScreenGeometry;
use crate::terminal::ColorMode;
use crate::terminal::OutputEncoding;
use crate::terminal::Terminal;
use crate::terminal::door::DoorTerminal;
use crate::terminal::stdout::StdoutTerminal;
use nc_data::{CampaignStore, GameConfig};

struct ParsedLaunchArgs {
    game_dir: PathBuf,
    explicit_player_record_index_1_based: Option<usize>,
    export_root: Option<PathBuf>,
    queue_dir: Option<PathBuf>,
    log_file: Option<PathBuf>,
    log_level: nc_log::LogLevel,
    encoding: OutputEncoding,
    color_mode: ColorMode,
    screen_geometry: ScreenGeometry,
    dropfile_alias: Option<String>,
    session_timeout_secs: Option<u32>,
    session_token: Option<String>,
    hosted_invite_code: Option<String>,
    use_door_terminal: bool,
}

struct SessionLeaseGuard {
    store: CampaignStore,
    player_npub: String,
    session_token: String,
    ttl_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchPlayerBindingSource {
    ExplicitPlayer,
    ReservedAlias,
    StoredHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchPlayerBinding {
    Bound {
        player_record_index_1_based: usize,
        source: LaunchPlayerBindingSource,
    },
    UnboundDropfile,
}

impl LaunchPlayerBinding {
    fn player_record_index_1_based(self) -> Option<usize> {
        match self {
            Self::Bound {
                player_record_index_1_based,
                ..
            } => Some(player_record_index_1_based),
            Self::UnboundDropfile => None,
        }
    }

    fn source(self) -> Option<LaunchPlayerBindingSource> {
        match self {
            Self::Bound { source, .. } => Some(source),
            Self::UnboundDropfile => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostedLaunchContext {
    player_npub: String,
    invite_code: Option<String>,
}

impl SessionLeaseGuard {
    fn activate(
        store: CampaignStore,
        session_token: String,
        now_unix_seconds: u64,
        ttl_seconds: u64,
        player_npub: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        store.activate_session_lease(&session_token, now_unix_seconds, ttl_seconds)?;
        Ok(Self {
            store,
            player_npub,
            session_token,
            ttl_seconds,
        })
    }

    fn heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.store
            .heartbeat_session_lease(&self.session_token, unix_now(), self.ttl_seconds)?;
        Ok(())
    }

    fn release(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.store.release_session_lease(&self.session_token)?;
        Ok(())
    }
}

impl Drop for SessionLeaseGuard {
    fn drop(&mut self) {
        let _ = self.store.release_session_lease(&self.session_token);
    }
}

const LOCAL_EXIT_ATTRIBUTION: &str = "For Griffith and glory.";

pub fn run(args: impl IntoIterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let parsed_args = args.into_iter().collect::<Vec<_>>();
    if matches!(
        parsed_args.get(1).map(String::as_str),
        Some("--help" | "-h")
    ) {
        print_usage();
        return Ok(());
    }
    if matches!(parsed_args.get(1).map(String::as_str), Some("submit-turn")) {
        return crate::submit_turn::run_submit_turn_args(&parsed_args[2..]);
    }
    let parsed = parse_args(&parsed_args)?;
    if let Some(log_file) = &parsed.log_file {
        nc_log::init_file_logging(log_file, parsed.log_level)?;
        tracing::info!(
            log_file = %log_file.display(),
            level = ?parsed.log_level,
            "nc-game logging initialized"
        );
    }

    let campaign_store = CampaignStore::open_default_in_dir(&parsed.game_dir)?;
    let game_config = load_runtime_game_config(&campaign_store)?;
    validate_runtime_game_config(&campaign_store, &game_config)?;
    let launch_binding = resolve_launch_player_binding(&parsed, &game_config, &campaign_store)?;
    let player_record_index_1_based = launch_binding.player_record_index_1_based().unwrap_or(1);
    let session_lease = parsed
        .session_token
        .clone()
        .map(|session_token| {
            let Some(player_record_index_1_based) = launch_binding.player_record_index_1_based()
            else {
                return Err("session token requires a bound player seat".into());
            };
            validate_session_lease(
                campaign_store.clone(),
                session_token,
                player_record_index_1_based,
                parsed.session_timeout_secs,
                &game_config,
            )
        })
        .transpose()?;
    let config = AppConfig {
        game_dir: parsed.game_dir.clone(),
        player_record_index_1_based,
        export_root: parsed.export_root.clone(),
        queue_dir: parsed.queue_dir.clone(),
        session_timeout_secs: parsed.session_timeout_secs,
        game_config: game_config.clone(),
    };

    let mut app = App::load(config)?;
    tracing::info!(
        game_dir = %parsed.game_dir.display(),
        player = player_record_index_1_based,
        door_mode = parsed.use_door_terminal,
        dropfile_alias = parsed.dropfile_alias.as_deref().unwrap_or(""),
        "loaded nc-game app"
    );
    apply_launch_context(
        &mut app,
        &parsed,
        launch_binding,
        session_lease.as_ref().map(|lease| HostedLaunchContext {
            player_npub: lease.player_npub.clone(),
            invite_code: parsed.hosted_invite_code.clone(),
        }),
    );
    if app.door_mode {
        crate::theme::apply_door_theme();
    }
    let mut terminal: Box<dyn Terminal> = if parsed.use_door_terminal {
        Box::new(DoorTerminal::with_encoding_and_color_mode(
            parsed.encoding,
            parsed.color_mode,
            parsed.screen_geometry,
        ))
    } else {
        Box::new(StdoutTerminal::with_encoding_and_color_mode(
            parsed.encoding,
            parsed.color_mode,
        ))
    };

    let interactive_terminal = std::io::stdout().is_terminal();
    let emit_local_exit_attribution =
        should_emit_local_exit_attribution(&parsed, interactive_terminal, session_lease.is_some());
    let result = if interactive_terminal {
        run_interactive(&mut app, terminal.as_mut(), session_lease.as_ref())
    } else {
        if let Some(session_lease) = session_lease.as_ref() {
            session_lease.heartbeat()?;
        }
        render_with_logging(&mut app, terminal.as_mut())
    };
    if let Some(session_lease) = session_lease.as_ref() {
        session_lease.release()?;
    }
    if result.is_ok() && emit_local_exit_attribution {
        emit_local_exit_lines();
    }
    result
}

fn should_emit_local_exit_attribution(
    parsed: &ParsedLaunchArgs,
    interactive_terminal: bool,
    has_session_lease: bool,
) -> bool {
    interactive_terminal
        && !parsed.use_door_terminal
        && parsed.session_token.is_none()
        && !has_session_lease
}

fn emit_local_exit_lines() {
    for line in local_exit_lines() {
        eprintln!("{line}");
    }
}

#[doc(hidden)]
pub fn local_exit_lines() -> Vec<String> {
    vec![LOCAL_EXIT_ATTRIBUTION.to_string()]
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn run_interactive(
    app: &mut App,
    terminal: &mut dyn Terminal,
    session_lease: Option<&SessionLeaseGuard>,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let _guard = RawModeGuard;
    let result = run_interactive_inner(app, terminal, session_lease);
    drop(_guard);
    let cleanup_result = terminal.clear_and_restore();
    result.and(cleanup_result)
}

fn run_interactive_inner(
    app: &mut App,
    terminal: &mut dyn Terminal,
    session_lease: Option<&SessionLeaseGuard>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if let Some(session_lease) = session_lease {
            session_lease.heartbeat()?;
        }
        render_with_logging(app, terminal)?;
        let key = terminal.read_key()?;
        let action = app.handle_key(key);
        let outcome = apply_action(app, action);
        if matches!(outcome, AppOutcome::Quit) {
            return Ok(());
        }
    }
}

fn render_with_logging(
    app: &mut App,
    terminal: &mut dyn Terminal,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(err) = app.render(terminal) {
        tracing::error!(
            player = app.player.record_index_1_based,
            screen = ?app.current_screen(),
            error = %err,
            "nc-game render failed"
        );
        return Err(err);
    }
    Ok(())
}

fn apply_launch_context(
    app: &mut App,
    parsed: &ParsedLaunchArgs,
    launch_binding: LaunchPlayerBinding,
    hosted_context: Option<HostedLaunchContext>,
) {
    app.screen_geometry = parsed.screen_geometry;
    app.door_mode = parsed.use_door_terminal;
    app.startup_state.fixed_player_launch = matches!(
        launch_binding.source(),
        Some(LaunchPlayerBindingSource::ExplicitPlayer)
    );
    if let Some(hosted_context) = hosted_context {
        app.set_hosted_invite_session(hosted_context.player_npub, hosted_context.invite_code);
    }
    app.startup_state.caller_alias = parsed.dropfile_alias.clone();
    if launch_binding == LaunchPlayerBinding::UnboundDropfile {
        app.enter_unbound_bbs_first_time_mode();
    }
}

fn parse_args(args: &[String]) -> Result<ParsedLaunchArgs, Box<dyn std::error::Error>> {
    let mut dir = None;
    let mut player: Option<usize> = None;
    let mut export_root = std::env::var_os("NC_GAME_EXPORT_ROOT")
        .or_else(|| std::env::var_os("EC_CLIENT_EXPORT_ROOT"))
        .map(PathBuf::from);
    let mut queue_dir = std::env::var_os("NC_GAME_QUEUE_DIR")
        .or_else(|| std::env::var_os("EC_CLIENT_QUEUE_DIR"))
        .map(PathBuf::from);
    let mut log_file = None;
    let mut log_level = nc_log::LogLevel::Info;
    let mut encoding = OutputEncoding::Utf8;
    let mut explicit_color_mode: Option<ColorMode> = None;
    let mut dropfile_path: Option<PathBuf> = None;
    let mut explicit_timeout_minutes: Option<u32> = None;
    let mut session_token = None;
    let mut hosted_invite_code = None;
    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--dir" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --dir".into());
                };
                dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--player" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --player".into());
                };
                player = Some(value.parse::<usize>()?);
                idx += 2;
            }
            "--export-root" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --export-root".into());
                };
                export_root = Some(PathBuf::from(value));
                idx += 2;
            }
            "--queue-dir" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --queue-dir".into());
                };
                queue_dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--log-file" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --log-file".into());
                };
                log_file = Some(PathBuf::from(value));
                idx += 2;
            }
            "--log-level" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "missing value for --log-level (error, warn, info, debug, or trace)".into(),
                    );
                };
                log_level = nc_log::LogLevel::parse(value)?;
                idx += 2;
            }
            "--encoding" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --encoding (utf8 or cp437)".into());
                };
                encoding = match value.as_str() {
                    "utf8" | "utf-8" => OutputEncoding::Utf8,
                    "cp437" | "CP437" => OutputEncoding::Cp437,
                    other => {
                        return Err(
                            format!("unknown encoding '{other}'; expected utf8 or cp437").into(),
                        );
                    }
                };
                idx += 2;
            }
            "--color-mode" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(
                        "missing value for --color-mode (ansi16, 256, truecolor, or auto)".into(),
                    );
                };
                explicit_color_mode = Some(match value.as_str() {
                    "ansi16" | "ansi-16" | "16" => ColorMode::Ansi16,
                    "256" | "color256" | "color-256" => ColorMode::Color256,
                    "truecolor" | "true-color" | "24bit" | "24-bit" => ColorMode::TrueColor,
                    "auto" => {
                        // Treat "auto" as an explicit no-op: use detection below.
                        explicit_color_mode = None;
                        idx += 2;
                        continue;
                    }
                    other => {
                        return Err(format!(
                            "unknown color mode '{other}'; expected ansi16, 256, truecolor, or auto"
                        )
                        .into());
                    }
                });
                idx += 2;
            }
            "--dropfile" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --dropfile".into());
                };
                dropfile_path = Some(PathBuf::from(value));
                idx += 2;
            }
            "--timeout" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --timeout (minutes)".into());
                };
                let minutes = value.parse::<u32>().map_err(|_| {
                    format!("--timeout value must be a positive integer, got '{value}'")
                })?;
                explicit_timeout_minutes = Some(minutes);
                idx += 2;
            }
            "--session-token" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --session-token".into());
                };
                session_token = Some(value.to_string());
                idx += 2;
            }
            "--hosted-invite-code" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err("missing value for --hosted-invite-code".into());
                };
                hosted_invite_code = Some(value.trim().to_string());
                idx += 2;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
    }

    // Parse dropfile and apply any fields not already set by explicit flags.
    let mut dropfile_alias: Option<String> = None;
    let mut dropfile_timeout_minutes: Option<u32> = None;
    let mut screen_geometry = ScreenGeometry::local_default();
    let mut use_door_terminal = false;

    if let Some(path) = &dropfile_path {
        let info = dropfile::parse(path).map_err(|e| format!("{e}"))?;
        dropfile_alias = info
            .alias
            .map(|alias| alias.trim().to_string())
            .filter(|alias| !alias.is_empty());
        dropfile_timeout_minutes = info.timeout_minutes;
        screen_geometry = ScreenGeometry::for_door(info.screen_rows);
        use_door_terminal = true;
    }

    // If a dropfile was given without an explicit --encoding, default to cp437.
    if dropfile_path.is_some() && !args.iter().any(|a| a == "--encoding") {
        encoding = OutputEncoding::Cp437;
    }

    let dir = dir.ok_or("usage: nc-game --dir <game_dir> [--player <1-based empire index>]")?;

    if matches!(player, Some(0)) {
        return Err("--player must be >= 1".into());
    }

    // Explicit --timeout overrides the dropfile value.
    let timeout_minutes = explicit_timeout_minutes.or(dropfile_timeout_minutes);
    let session_timeout_secs = timeout_minutes.map(|m| m.saturating_mul(60));

    // Resolve color mode: explicit flag > cp437-default > env-based detection.
    let color_mode = explicit_color_mode.unwrap_or_else(|| {
        if encoding == OutputEncoding::Cp437 {
            // BBS/door clients: assume classic 16-color unless explicitly overridden.
            ColorMode::Ansi16
        } else {
            detect_color_mode()
        }
    });

    Ok(ParsedLaunchArgs {
        game_dir: dir,
        explicit_player_record_index_1_based: player,
        export_root,
        queue_dir,
        log_file,
        log_level,
        encoding,
        color_mode,
        screen_geometry,
        dropfile_alias,
        session_timeout_secs,
        session_token,
        hosted_invite_code,
        use_door_terminal,
    })
}

/// Detect the terminal's color depth from standard environment variables.
///
/// Detection order:
/// 1. `COLORTERM=truecolor` or `COLORTERM=24bit` → [`ColorMode::TrueColor`]
/// 2. known modern truecolor `TERM` values       → [`ColorMode::TrueColor`]
/// 3. `TERM` containing `256color`               → [`ColorMode::Color256`]
/// 4. any non-empty, non-`dumb` `TERM`           → [`ColorMode::Color256`]
/// 5. fallback                                   → [`ColorMode::Ansi16`]
///
/// The 16-color default is reserved for BBS/door mode or genuinely minimal
/// terminals. For normal local/SSH play, an interactive terminal should get at
/// least 256-color output unless it explicitly identifies as `dumb`.
pub fn detect_color_mode() -> ColorMode {
    if let Ok(colorterm) = std::env::var("COLORTERM") {
        let ct = colorterm.to_ascii_lowercase();
        if ct == "truecolor" || ct == "24bit" {
            return ColorMode::TrueColor;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        let term = term.to_ascii_lowercase();
        if [
            "kitty",
            "wezterm",
            "ghostty",
            "alacritty",
            "foot",
            "contour",
            "rio",
        ]
        .iter()
        .any(|needle| term.contains(needle))
        {
            return ColorMode::TrueColor;
        }
        if term.contains("256color") {
            return ColorMode::Color256;
        }
        if !term.trim().is_empty() && term != "dumb" {
            return ColorMode::Color256;
        }
    }
    ColorMode::Ansi16
}

fn print_usage() {
    println!("Usage:");
    println!(
        "  nc-game --dir <game_dir> [--player <1-based empire index>] \
         [--encoding <utf8|cp437>] [--color-mode <ansi16|256|truecolor|auto>] \
         [--dropfile <path>] [--timeout <minutes>] [--session-token <token>] \
         [--hosted-invite-code <code>] \
         [--export-root <dir>] [--queue-dir <dir>] \
         [--log-file <path>] [--log-level <error|warn|info|debug|trace>]"
    );
    println!(
        "  nc-game submit-turn [--check] --dir <game_dir> --player <record> --file <turn.kdl>"
    );
    println!();
    println!("BBS door flags:");
    println!("  --dropfile <path>   Parse a BBS dropfile (DOOR32.SYS, DOOR.SYS, or CHAIN.TXT).");
    println!("                      Supplies alias and timeout; explicit flags always override.");
    println!("                      Defaults encoding to cp437 when no --encoding is given.");
    println!("                      Reserved aliases in ncgame.db can omit --player.");
    println!("  --timeout <minutes> Session time limit in minutes.");
    println!();
    println!("Logging:");
    println!("  --log-file <path>   Append diagnostic logs to a text file.");
    println!("  --log-level <lvl>   error, warn, info, debug, or trace (default: info).");
    println!();
    println!("Color modes:");
    println!("  ansi16     Classic 16-color ANSI (BBS doors, legacy terminals)");
    println!("  256        256-color xterm palette");
    println!("  truecolor  24-bit RGB (default for modern local/SSH terminals)");
    println!("  auto       Detect from COLORTERM/TERM environment variables (default)");
}

fn load_runtime_game_config(
    campaign_store: &CampaignStore,
) -> Result<GameConfig, Box<dyn std::error::Error>> {
    let settings = campaign_store.load_campaign_settings()?;
    Ok(GameConfig {
        game_name: settings.game_name,
        theme: Some(PathBuf::from(format!(
            "themes/{}.kdl",
            settings.default_theme_key
        ))),
        snoop: settings.snoop_enabled,
        session: nc_data::SessionConfig {
            max_idle_minutes: settings.session_max_idle_minutes,
            minimum_time_minutes: settings.session_minimum_time_minutes,
            local_timeout: settings.session_local_timeout,
            remote_timeout: settings.session_remote_timeout,
        },
        inactivity: nc_data::InactivityConfig {
            purge_after_turns: settings.inactivity_purge_after_turns,
            autopilot_after_turns: settings.inactivity_autopilot_after_turns,
        },
        reservations: settings.reservations,
    })
}

fn resolve_launch_player_binding(
    parsed: &ParsedLaunchArgs,
    game_config: &GameConfig,
    campaign_store: &CampaignStore,
) -> Result<LaunchPlayerBinding, Box<dyn std::error::Error>> {
    let runtime_state = campaign_store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    let player_count = runtime_state.game_data.player.records.len();

    if let Some(explicit_player) = parsed.explicit_player_record_index_1_based {
        if explicit_player > player_count {
            return Err(format!(
                "--player {} exceeds player count {}",
                explicit_player, player_count
            )
            .into());
        }
    }

    let alias_reservation = parsed
        .dropfile_alias
        .as_deref()
        .and_then(|alias| game_config.reservation_for_alias(alias));

    if let Some(reservation) = alias_reservation {
        validate_reserved_seat_runtime(&parsed.game_dir, game_config, reservation)?;
        if let Some(explicit_player) = parsed.explicit_player_record_index_1_based {
            if explicit_player != reservation.player_record_index_1_based {
                return Err(format!(
                    "--player {} does not match reserved seat {} for alias '{}'",
                    explicit_player, reservation.player_record_index_1_based, reservation.alias
                )
                .into());
            }
        }
        return Ok(LaunchPlayerBinding::Bound {
            player_record_index_1_based: reservation.player_record_index_1_based,
            source: LaunchPlayerBindingSource::ReservedAlias,
        });
    }

    if let Some(alias) = parsed.dropfile_alias.as_deref().map(str::trim) {
        if !alias.is_empty() {
            let matching_players = runtime_state
                .game_data
                .player
                .records
                .iter()
                .enumerate()
                .filter_map(|(idx, player)| {
                    let handle = player.assigned_player_handle_summary();
                    (!handle.is_empty() && handle.eq_ignore_ascii_case(alias)).then_some(idx + 1)
                })
                .collect::<Vec<_>>();

            if matching_players.len() > 1 {
                return Err(format!(
                    "caller alias '{}' matches multiple joined empires; reserve the caller explicitly in ncgame.db",
                    alias
                )
                .into());
            }

            if let Some(player_record_index_1_based) = matching_players.first().copied() {
                if let Some(reservation) =
                    game_config.reservation_for_player(player_record_index_1_based)
                {
                    if !reservation.alias.eq_ignore_ascii_case(alias) {
                        return Err(format!(
                            "caller alias '{}' conflicts with reserved alias '{}' for seat {}; reconcile ncgame.db settings or the campaign state",
                            alias, reservation.alias, player_record_index_1_based
                        )
                        .into());
                    }
                }
                if let Some(explicit_player) = parsed.explicit_player_record_index_1_based {
                    if explicit_player != player_record_index_1_based {
                        return Err(format!(
                            "--player {} does not match stored handle seat {} for alias '{}'",
                            explicit_player, player_record_index_1_based, alias
                        )
                        .into());
                    }
                }
                return Ok(LaunchPlayerBinding::Bound {
                    player_record_index_1_based,
                    source: LaunchPlayerBindingSource::StoredHandle,
                });
            }
        }
    }

    if let Some(explicit_player) = parsed.explicit_player_record_index_1_based {
        return Ok(LaunchPlayerBinding::Bound {
            player_record_index_1_based: explicit_player,
            source: LaunchPlayerBindingSource::ExplicitPlayer,
        });
    }

    if parsed.use_door_terminal {
        return Ok(LaunchPlayerBinding::UnboundDropfile);
    }

    Err(
        "usage: nc-game --dir <game_dir> --player <1-based empire index>\n       or use --dropfile for BBS/door mode".into(),
    )
}

fn validate_runtime_game_config(
    campaign_store: &CampaignStore,
    game_config: &GameConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if game_config.reservations.is_empty() {
        return Ok(());
    }
    let runtime_state = campaign_store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    game_config
        .validate_reservations_for_player_count(runtime_state.game_data.player.records.len())?;
    Ok(())
}

fn validate_reserved_seat_runtime(
    game_dir: &std::path::Path,
    game_config: &GameConfig,
    reservation: &nc_data::SeatReservation,
) -> Result<(), Box<dyn std::error::Error>> {
    let campaign_store = CampaignStore::open_default_in_dir(game_dir)?;
    let runtime_state = campaign_store
        .load_latest_runtime_state()?
        .ok_or("campaign store has no snapshots; initialize the campaign with nc-sysop first")?;
    let game_data = runtime_state.game_data;
    game_config.validate_reservations_for_player_count(game_data.player.records.len())?;
    let player = game_data
        .player
        .records
        .get(reservation.player_record_index_1_based - 1)
        .ok_or_else(|| {
            format!(
                "reserved player {} is missing from PLAYER.DAT",
                reservation.player_record_index_1_based
            )
        })?;
    let handle = player.assigned_player_handle_summary();
    if !handle.is_empty() && !handle.eq_ignore_ascii_case(&reservation.alias) {
        return Err(format!(
            "reserved alias '{}' conflicts with stored player handle '{}' for seat {}; reconcile ncgame.db settings or the campaign state",
            reservation.alias, handle, reservation.player_record_index_1_based
        )
        .into());
    }
    Ok(())
}

fn validate_session_lease(
    campaign_store: CampaignStore,
    session_token: String,
    player_record_index_1_based: usize,
    session_timeout_secs: Option<u32>,
    game_config: &GameConfig,
) -> Result<SessionLeaseGuard, Box<dyn std::error::Error>> {
    let lease = campaign_store.load_session_lease(&session_token, unix_now())?;
    if lease.player_record_index_1_based != player_record_index_1_based {
        return Err(format!(
            "session token is for seat {}, not seat {}",
            lease.player_record_index_1_based, player_record_index_1_based
        )
        .into());
    }
    SessionLeaseGuard::activate(
        campaign_store,
        session_token,
        unix_now(),
        session_lease_ttl_seconds(session_timeout_secs, game_config),
        lease.player_npub,
    )
}

fn session_lease_ttl_seconds(session_timeout_secs: Option<u32>, game_config: &GameConfig) -> u64 {
    session_timeout_secs
        .map(u64::from)
        .or_else(|| {
            let idle_timeout_secs =
                u64::from(game_config.session.max_idle_minutes).saturating_mul(60);
            (idle_timeout_secs > 0).then_some(idle_timeout_secs)
        })
        .unwrap_or(120)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::{
        HostedLaunchContext, LaunchPlayerBinding, LaunchPlayerBindingSource, ParsedLaunchArgs,
        apply_launch_context, local_exit_lines, resolve_launch_player_binding,
        session_lease_ttl_seconds, should_emit_local_exit_attribution,
    };
    use crate::app::{App, AppConfig};
    use crate::domains::startup::state::FirstTimeOnboardingMode;
    use crate::error::{
        HOSTED_ONBOARDING_INVARIANT_EXIT_CODE, HostedOnboardingInvariantError, exit_code_for,
    };
    use crate::screen::ScreenGeometry;
    use crate::screen::ScreenId;
    use crate::terminal::{ColorMode, OutputEncoding};
    use nc_compat::import_directory_snapshot;
    use nc_data::{CampaignSettings, CampaignStore, CoreGameData, GameConfig, SeatReservation};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
        fs::create_dir_all(dst).expect("create temp dir");
        for entry in fs::read_dir(src).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            let target = dst.join(entry.file_name());
            if path.is_dir() {
                copy_dir_all(&path, &target);
            } else {
                fs::copy(&path, &target).expect("copy file");
            }
        }
    }

    fn temp_first_time_game_copy() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "nc-game-cli-test-{}-{}-{}",
            std::process::id(),
            TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time ok")
                .as_nanos()
        ));
        copy_dir_all(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
        let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
        import_directory_snapshot(&store, &root).expect("seed sqlite snapshot");
        root
    }

    fn write_reserved_config(root: &std::path::Path, alias: &str, player: usize) {
        let store = CampaignStore::open_default_in_dir(root).expect("open campaign store");
        let mut settings = store
            .load_campaign_settings()
            .unwrap_or_else(|_| CampaignSettings::new("fixture-game", "Esterian Conquest"));
        settings.reservations = vec![SeatReservation {
            player_record_index_1_based: player,
            alias: alias.to_string(),
        }];
        store
            .save_campaign_settings(&settings)
            .expect("write settings");
    }

    fn seed_joined_player_handle(root: &std::path::Path, player: usize, alias: &str) {
        let mut data = CoreGameData::load(root).expect("load fixture");
        data.join_player(player, &format!("Empire {player}"))
            .expect("join player");
        data.player.records[player - 1].set_assigned_player_handle_raw(alias);
        data.save(root).expect("save fixture");
        let store = CampaignStore::open_default_in_dir(root).expect("open campaign store");
        import_directory_snapshot(&store, root).expect("refresh sqlite snapshot");
    }

    fn parsed_args(use_door_terminal: bool) -> ParsedLaunchArgs {
        ParsedLaunchArgs {
            game_dir: PathBuf::from("/tmp/test"),
            explicit_player_record_index_1_based: Some(1),
            export_root: None,
            queue_dir: None,
            log_file: None,
            log_level: nc_log::LogLevel::Info,
            encoding: OutputEncoding::Utf8,
            color_mode: ColorMode::TrueColor,
            screen_geometry: ScreenGeometry::local_default(),
            dropfile_alias: None,
            session_timeout_secs: None,
            session_token: None,
            hosted_invite_code: None,
            use_door_terminal,
        }
    }

    fn hosted_args() -> ParsedLaunchArgs {
        let mut parsed = parsed_args(false);
        parsed.session_token = Some("session-token".to_string());
        parsed
    }

    #[test]
    fn resolve_launch_binding_uses_stored_handle_for_returning_dropfile_caller() {
        let game_dir = temp_first_time_game_copy();
        seed_joined_player_handle(&game_dir, 2, "RIVAL");
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        let mut parsed = parsed_args(true);
        parsed.game_dir = game_dir.clone();
        parsed.explicit_player_record_index_1_based = None;
        parsed.dropfile_alias = Some("rival".to_string());

        let binding =
            resolve_launch_player_binding(&parsed, &GameConfig::default(), &campaign_store)
                .expect("binding should resolve");

        assert_eq!(
            binding,
            LaunchPlayerBinding::Bound {
                player_record_index_1_based: 2,
                source: LaunchPlayerBindingSource::StoredHandle,
            }
        );

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn resolve_launch_binding_uses_unbound_dropfile_for_new_bbs_caller() {
        let game_dir = temp_first_time_game_copy();
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        let mut parsed = parsed_args(true);
        parsed.game_dir = game_dir.clone();
        parsed.explicit_player_record_index_1_based = None;
        parsed.dropfile_alias = Some("RIVAL".to_string());

        let binding =
            resolve_launch_player_binding(&parsed, &GameConfig::default(), &campaign_store)
                .expect("binding should resolve");

        assert_eq!(binding, LaunchPlayerBinding::UnboundDropfile);

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn resolve_launch_binding_rejects_stored_handle_on_other_players_reserved_seat() {
        let game_dir = temp_first_time_game_copy();
        seed_joined_player_handle(&game_dir, 2, "RIVAL");
        write_reserved_config(&game_dir, "SYSOP", 2);
        let campaign_store = CampaignStore::open_default_in_dir(&game_dir).expect("open store");
        let mut parsed = parsed_args(true);
        parsed.game_dir = game_dir.clone();
        parsed.explicit_player_record_index_1_based = None;
        parsed.dropfile_alias = Some("RIVAL".to_string());

        let err = resolve_launch_player_binding(
            &parsed,
            &GameConfig {
                reservations: vec![SeatReservation {
                    player_record_index_1_based: 2,
                    alias: "SYSOP".to_string(),
                }],
                ..GameConfig::default()
            },
            &campaign_store,
        )
        .expect_err("binding should reject reservation conflict");

        assert!(
            err.to_string()
                .contains("conflicts with reserved alias 'SYSOP' for seat 2")
        );

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn session_lease_uses_explicit_timeout_when_present() {
        let mut game_config = GameConfig::default();
        game_config.session.max_idle_minutes = 10;
        assert_eq!(session_lease_ttl_seconds(Some(45), &game_config), 45);
    }

    #[test]
    fn session_lease_uses_campaign_idle_timeout_by_default() {
        let mut game_config = GameConfig::default();
        game_config.session.max_idle_minutes = 10;
        assert_eq!(session_lease_ttl_seconds(None, &game_config), 600);
    }

    #[test]
    fn session_lease_falls_back_when_timeout_is_disabled() {
        let mut game_config = GameConfig::default();
        game_config.session.max_idle_minutes = 0;
        assert_eq!(session_lease_ttl_seconds(None, &game_config), 120);
    }

    #[test]
    fn local_exit_lines_match_nc_connect_attribution() {
        assert_eq!(
            local_exit_lines(),
            vec!["For Griffith and glory.".to_string()]
        );
    }

    #[test]
    fn attribution_only_emits_for_local_interactive_stdout_sessions() {
        assert!(should_emit_local_exit_attribution(
            &parsed_args(false),
            true,
            false
        ));
        assert!(!should_emit_local_exit_attribution(
            &parsed_args(true),
            true,
            false
        ));
        assert!(!should_emit_local_exit_attribution(
            &parsed_args(false),
            false,
            false
        ));
        assert!(!should_emit_local_exit_attribution(
            &parsed_args(false),
            true,
            true
        ));
        assert!(!should_emit_local_exit_attribution(
            &hosted_args(),
            true,
            false
        ));
    }

    #[test]
    fn hosted_onboarding_invariant_maps_to_dedicated_exit_code() {
        let err = HostedOnboardingInvariantError::new("FirstTimeMenu");

        assert_eq!(
            exit_code_for(&err),
            Some(HOSTED_ONBOARDING_INVARIANT_EXIT_CODE)
        );
    }

    #[test]
    fn apply_launch_context_marks_hosted_first_join_state() {
        let game_dir = temp_first_time_game_copy();
        let config = AppConfig {
            game_dir: game_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: GameConfig::default(),
        };
        let mut app = App::load(config).expect("app should load");
        let parsed = parsed_args(false);

        apply_launch_context(
            &mut app,
            &parsed,
            LaunchPlayerBinding::Bound {
                player_record_index_1_based: 1,
                source: LaunchPlayerBindingSource::ExplicitPlayer,
            },
            Some(HostedLaunchContext {
                player_npub: "npub1hostedplayer".to_string(),
                invite_code: Some("velvet-mountain".to_string()),
            }),
        );

        assert_eq!(
            app.startup_state.hosted_player_npub.as_deref(),
            Some("npub1hostedplayer")
        );
        assert_eq!(
            app.startup_state.hosted_invite_code.as_deref(),
            Some("velvet-mountain")
        );
        assert_eq!(
            app.startup_state.first_time_onboarding_mode,
            FirstTimeOnboardingMode::HostedInvite
        );
        assert!(app.startup_state.fixed_player_launch);

        let _ = fs::remove_dir_all(&game_dir);
    }

    #[test]
    fn apply_launch_context_moves_unbound_dropfile_to_first_time_menu() {
        let game_dir = temp_first_time_game_copy();
        let config = AppConfig {
            game_dir: game_dir.clone(),
            player_record_index_1_based: 1,
            export_root: None,
            queue_dir: None,
            session_timeout_secs: None,
            game_config: GameConfig::default(),
        };
        let mut app = App::load(config).expect("app should load");
        let mut parsed = parsed_args(true);
        parsed.dropfile_alias = Some("RIVAL".to_string());

        apply_launch_context(
            &mut app,
            &parsed,
            LaunchPlayerBinding::UnboundDropfile,
            None,
        );

        assert_eq!(app.current_screen(), ScreenId::FirstTimeMenu);
        assert!(app.startup_state.unbound_bbs_caller);
        assert_eq!(app.startup_state.caller_alias.as_deref(), Some("RIVAL"));

        let _ = fs::remove_dir_all(&game_dir);
    }
}
