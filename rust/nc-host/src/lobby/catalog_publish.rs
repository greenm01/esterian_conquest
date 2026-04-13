use nc_data::hosted::{
    HostedStore, RecruitingMode as HostedRecruiting, get_game_metadata, get_settings, list_seats,
};
use nc_nostr::game_definition::{
    GameDefinition, GameStatus, RecruitingMode as NostrRecruiting, SeatSlot, SeatStatus,
};
use std::path::PathBuf;

pub fn publish_game_definition(
    store: &HostedStore,
    game_id: &str,
    host_alias: Option<&str>,
) -> Result<Option<GameDefinition>, Box<dyn std::error::Error>> {
    let settings = match get_settings(store.connection(), game_id) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    if settings.lobby_visibility != nc_data::hosted::LobbyVisibility::Public {
        return Ok(None);
    }
    if settings.recruiting == HostedRecruiting::None {
        return Ok(None);
    }

    let seats = list_seats(store.connection(), game_id)?;
    let open_seats = seats
        .iter()
        .filter(|s| s.status == nc_data::hosted::SeatStatus::Pending)
        .count() as u32;
    let claimed_seats = seats
        .iter()
        .filter(|s| s.status == nc_data::hosted::SeatStatus::Claimed)
        .count();

    let recruiting = match settings.recruiting {
        HostedRecruiting::None => NostrRecruiting::None,
        HostedRecruiting::NewPlayers => NostrRecruiting::NewPlayers,
        HostedRecruiting::ReplacementPlayers => NostrRecruiting::ReplacementPlayers,
    };

    let status = if claimed_seats == 0 {
        GameStatus::Setup
    } else if claimed_seats == seats.len() as usize {
        GameStatus::Active
    } else {
        GameStatus::Active
    };

    let slot_tags: Vec<SeatSlot> = seats
        .iter()
        .map(|s| SeatSlot {
            seat: s.seat_number,
            invite_code_hash: s.invite_code_hash.clone(),
            player_npub: s.player_pubkey.clone(),
            status: match s.status {
                nc_data::hosted::SeatStatus::Pending => SeatStatus::Pending,
                nc_data::hosted::SeatStatus::Claimed => SeatStatus::Claimed,
            },
        })
        .collect();

    let metadata = get_game_metadata(store.connection(), game_id).ok();

    let def = GameDefinition {
        game_id: game_id.to_string(),
        game_name: metadata
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| game_id.to_string()),
        status,
        created_at: metadata.as_ref().map(|m| m.created_at),
        players: metadata
            .as_ref()
            .map(|m| m.players)
            .unwrap_or(seats.len() as u32),
        recruiting,
        open_seats,
        year: metadata.as_ref().map(|m| m.current_year).unwrap_or(3000),
        turn: metadata.as_ref().map(|m| m.current_turn).unwrap_or(0),
        summary: settings.summary,
        host_alias: host_alias
            .map(String::from)
            .or_else(|| settings.host_alias.clone()),
        slots: slot_tags,
    };

    Ok(Some(def))
}

pub fn collect_lobby_games(
    games_root: &PathBuf,
) -> Result<Vec<GameDefinition>, Box<dyn std::error::Error>> {
    let mut games = Vec::new();

    if let Ok(entries) = std::fs::read_dir(games_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let db_path = path.join("hosted.db");
                if db_path.exists() {
                    if let Some(game_id) = path.file_name().and_then(|n| n.to_str()) {
                        let store = HostedStore::open(&db_path)?;
                        if let Ok(Some(def)) = publish_game_definition(&store, game_id, None) {
                            games.push(def);
                        }
                    }
                }
            }
        }
    }

    Ok(games)
}
