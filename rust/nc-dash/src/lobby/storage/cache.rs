use kdl::KdlDocument;

use super::paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedGameCacheEntry {
    pub game_id: String,
    pub game_name: String,
    pub host_alias: String,
    pub seat: Option<u8>,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRequestCacheEntry {
    pub game_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LobbyCacheRecord {
    pub joined_games: Vec<JoinedGameCacheEntry>,
    pub pending_requests: Vec<PendingRequestCacheEntry>,
}

pub fn cache_path() -> std::path::PathBuf {
    paths::cache_path()
}

pub fn parse_cache_kdl(raw: &str) -> Result<LobbyCacheRecord, Box<dyn std::error::Error>> {
    let doc: KdlDocument = raw.parse()?;
    let joined_games = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "joined-game")
        .map(|node| JoinedGameCacheEntry {
            game_id: node
                .get("id")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
            game_name: node
                .get("name")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
            host_alias: node
                .get("host")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
            seat: node
                .get("seat")
                .and_then(|value| value.as_integer())
                .map(|seat| seat as u8),
            state: node
                .get("state")
                .and_then(|value| value.as_string())
                .unwrap_or("joined")
                .to_string(),
        })
        .collect();
    let pending_requests = doc
        .nodes()
        .iter()
        .filter(|node| node.name().value() == "pending-request")
        .map(|node| PendingRequestCacheEntry {
            game_id: node
                .get("game-id")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
            status: node
                .get("status")
                .and_then(|value| value.as_string())
                .unwrap_or_default()
                .to_string(),
        })
        .collect();
    Ok(LobbyCacheRecord {
        joined_games,
        pending_requests,
    })
}

pub fn render_cache_kdl(record: &LobbyCacheRecord) -> String {
    let mut out = String::from("cache\n");
    for game in &record.joined_games {
        let seat = game
            .seat
            .map(|seat| format!(" seat={seat}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "joined-game id=\"{}\" name=\"{}\" host=\"{}\" state=\"{}\"{}\n",
            game.game_id, game.game_name, game.host_alias, game.state, seat
        ));
    }
    for request in &record.pending_requests {
        out.push_str(&format!(
            "pending-request game-id=\"{}\" status=\"{}\"\n",
            request.game_id, request.status
        ));
    }
    out
}
