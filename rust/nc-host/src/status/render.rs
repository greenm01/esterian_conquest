use super::model::{DaemonStatusReport, GameStatusRow};

pub fn render_human(report: &DaemonStatusReport) -> String {
    let mut lines = Vec::new();

    lines.push("nc-host status".to_string());
    lines.push(format!("  Games root: {}", report.games_root));
    if let Some(config_path) = &report.config_path {
        lines.push(format!("  Config: {}", config_path));
    }
    lines.push(format!(
        "  Relay: {} [{}]",
        report.relay.url, report.relay.status
    ));
    if let Some(latency) = report.relay.latency_ms {
        lines.push(format!("  Relay latency: {} ms", latency));
    }
    if let Some(error) = &report.relay.error {
        lines.push(format!("  Relay error: {}", error));
    }
    lines.push(String::new());
    lines.push(format!(
        "Totals: games={} recruiting={} due={} pending-requests={} pending-decisions={} pending-turns={} outbox-pending={} outbox-failed={}",
        report.totals.discovered_games,
        report.totals.public_recruiting_games,
        report.totals.due_maintenance_games,
        report.totals.pending_requests,
        report.totals.pending_decisions,
        report.totals.pending_turns,
        report.totals.outbox_pending,
        report.totals.outbox_failed,
    ));
    lines.push(String::new());

    if report.games.is_empty() {
        lines.push("No hosted games found.".to_string());
    } else {
        lines.push("Games:".to_string());
        for game in &report.games {
            lines.push(render_game_row(game));
        }
    }

    lines.join("\n")
}

fn render_game_row(game: &GameStatusRow) -> String {
    format!(
        "  {}  {}  catalog={}  y{} t{}  seats {}/{}  recruiting={}  maint={}  req={} dec={} turns={} outbox {}/{}",
        game.game_id,
        game.status,
        game.catalog_state,
        game.year,
        game.turn,
        game.claimed_seats,
        game.players,
        game.recruiting,
        if game.maintenance_due_now {
            "due"
        } else if game.maintenance_enabled {
            "on"
        } else {
            "off"
        },
        game.pending_requests,
        game.pending_decisions,
        game.pending_turns,
        game.outbox_pending,
        game.outbox_failed,
    )
}
