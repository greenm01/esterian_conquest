//! KDL serialization and deserialization for `roster.kdl` files.

use std::fs;
use std::io::Write as IoWrite;
use std::path::Path;

use super::{Roster, Seat, SeatStatus};

/// Load a roster from `path` (typically `<game-dir>/roster.kdl`).
pub fn load_roster(path: &Path) -> Result<Roster, Box<dyn std::error::Error>> {
    let text =
        fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    parse_roster_str(&text)
        .map_err(|err| format!("invalid roster at {}: {err}", path.display()).into())
}

/// Save a roster to `path` atomically (write to temp, rename).
pub fn save_roster(path: &Path, roster: &Roster) -> Result<(), Box<dyn std::error::Error>> {
    let content = render_roster(roster);
    // Write to a sibling temp file, then rename for atomicity.
    let tmp_path = path.with_extension("kdl.tmp");
    {
        let mut f = fs::File::create(&tmp_path)
            .map_err(|err| format!("cannot create temp file {}: {err}", tmp_path.display()))?;
        f.write_all(content.as_bytes())
            .map_err(|err| format!("cannot write temp file {}: {err}", tmp_path.display()))?;
        f.flush()
            .map_err(|err| format!("cannot flush temp file {}: {err}", tmp_path.display()))?;
    }
    fs::rename(&tmp_path, path).map_err(|err| {
        format!(
            "cannot rename {} -> {}: {err}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

/// Parse a roster from KDL source text.
///
/// Exposed as a separate function so tests can parse without going through disk.
pub fn parse_roster_str(text: &str) -> Result<Roster, String> {
    let document: kdl::KdlDocument = text
        .parse()
        .map_err(|err| format!("KDL parse error: {err}"))?;

    let game_node = document.get("game").ok_or("missing `game` node")?;

    let id = prop_str(game_node, "id")?;
    let name = prop_str(game_node, "name")?;

    let mut seats = Vec::new();
    if let Some(children) = game_node.children() {
        for node in children.nodes() {
            if node.name().value() != "seat" {
                continue;
            }
            let player = prop_usize(node, "player")?;
            let code = prop_str(node, "code")?;
            let status_str = prop_str(node, "status")?;
            let status = SeatStatus::parse(&status_str)?;
            let npub = opt_prop_str(node, "npub")?;
            seats.push(Seat {
                player,
                code,
                status,
                npub,
            });
        }
    }

    Ok(Roster { id, name, seats })
}

/// Render a roster to a KDL string.
pub fn render_roster(roster: &Roster) -> String {
    let mut out = format!(
        "game id=\"{}\" name=\"{}\" {{\n",
        kdl_escape(&roster.id),
        kdl_escape(&roster.name)
    );
    for seat in &roster.seats {
        match (&seat.status, &seat.npub) {
            (SeatStatus::Claimed, Some(npub)) => {
                out.push_str(&format!(
                    "    seat player={} code=\"{}\" status=\"{}\" npub=\"{}\"\n",
                    seat.player,
                    kdl_escape(&seat.code),
                    seat.status.as_str(),
                    kdl_escape(npub),
                ));
            }
            _ => {
                out.push_str(&format!(
                    "    seat player={} code=\"{}\" status=\"{}\"\n",
                    seat.player,
                    kdl_escape(&seat.code),
                    seat.status.as_str(),
                ));
            }
        }
    }
    out.push_str("}\n");
    out
}

// --- KDL property helpers ---

fn prop_str(node: &kdl::KdlNode, name: &str) -> Result<String, String> {
    node.get(name)
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .ok_or_else(|| format!("missing or non-string property `{name}`"))
}

fn opt_prop_str(node: &kdl::KdlNode, name: &str) -> Result<Option<String>, String> {
    Ok(node
        .get(name)
        .and_then(|v| v.as_string())
        .map(str::to_string))
}

fn prop_usize(node: &kdl::KdlNode, name: &str) -> Result<usize, String> {
    let v = node
        .get(name)
        .and_then(|v| v.as_integer())
        .ok_or_else(|| format!("missing or non-integer property `{name}`"))?;
    usize::try_from(v).map_err(|_| format!("property `{name}` out of range: {v}"))
}

fn kdl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
