use nc_data::hosted::{
    CatalogState as HostedCatalogState, GameTier as HostedTier, HostedStore,
    RecruitingMode as HostedRecruiting, get_game_metadata, get_settings, list_seats,
};
use nc_nostr::game_definition::{
    CatalogState as NostrCatalogState, GameDefinition, GameStatus, GameTier as NostrTier,
    RecruitingMode as NostrRecruiting, SeatSlot, SeatStatus,
};
use std::path::PathBuf;

pub fn publish_game_definition(
    store: &HostedStore,
    game_id: &str,
    host_alias: Option<&str>,
    host_contact_npub: Option<&str>,
    host_contact_label: Option<&str>,
    host_contact_nip05: Option<&str>,
) -> Result<Option<GameDefinition>, Box<dyn std::error::Error>> {
    let settings = match get_settings(store.connection(), game_id) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    if settings.catalog_state != HostedCatalogState::Retired
        && (settings.lobby_visibility != nc_data::hosted::LobbyVisibility::Public
            || settings.recruiting == HostedRecruiting::None)
    {
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

    let game_tier = Some(match settings.game_tier {
        HostedTier::Sandbox => NostrTier::Sandbox,
        HostedTier::League => NostrTier::League,
    });
    let catalog_state = match settings.catalog_state {
        HostedCatalogState::Listed => NostrCatalogState::Listed,
        HostedCatalogState::Retired => NostrCatalogState::Retired,
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
        catalog_state,
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
        host_contact_npub: host_contact_npub
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        host_contact_label: host_contact_label
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        host_contact_nip05: host_contact_nip05
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        slots: slot_tags,
        game_tier,
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
                        if let Ok(Some(def)) =
                            publish_game_definition(&store, game_id, None, None, None, None)
                        {
                            games.push(def);
                        }
                    }
                }
            }
        }
    }

    Ok(games)
}
