use nostr_sdk::Event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameDefinition {
    pub game_id: String,
    pub game_name: String,
    pub status: GameStatus,
    pub players: u32,
    pub recruiting: RecruitingMode,
    pub open_seats: u32,
    pub year: u32,
    pub turn: u32,
    pub summary: Option<String>,
    pub host_alias: Option<String>,
    pub slots: Vec<SeatSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameStatus {
    Setup,
    Active,
    Finished,
}

impl GameStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "setup" => Some(GameStatus::Setup),
            "active" => Some(GameStatus::Active),
            "finished" => Some(GameStatus::Finished),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            GameStatus::Setup => "setup",
            GameStatus::Active => "active",
            GameStatus::Finished => "finished",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecruitingMode {
    None,
    NewPlayers,
    ReplacementPlayers,
}

impl RecruitingMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(RecruitingMode::None),
            "new_players" => Some(RecruitingMode::NewPlayers),
            "replacement_players" => Some(RecruitingMode::ReplacementPlayers),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RecruitingMode::None => "none",
            RecruitingMode::NewPlayers => "new_players",
            RecruitingMode::ReplacementPlayers => "replacement_players",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatSlot {
    pub seat: u32,
    pub invite_code_hash: String,
    pub player_npub: Option<String>,
    pub status: SeatStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatStatus {
    Pending,
    Claimed,
}

impl SeatStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(SeatStatus::Pending),
            "claimed" => Some(SeatStatus::Claimed),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SeatStatus::Pending => "pending",
            SeatStatus::Claimed => "claimed",
        }
    }
}

pub fn parse_game_definition(event: &Event) -> Option<GameDefinition> {
    let mut game_id = None;
    let mut game_name = None;
    let mut status = None;
    let mut players = None;
    let mut recruiting = None;
    let mut open_seats = None;
    let mut year = None;
    let mut turn = None;
    let mut summary = None;
    let mut host_alias = None;
    let mut slots = Vec::new();

    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        let Some(kind) = values.first().map(String::as_str) else {
            continue;
        };
        match kind {
            "d" if values.len() >= 2 => game_id = Some(values[1].clone()),
            "name" if values.len() >= 2 => game_name = Some(values[1].clone()),
            "status" if values.len() >= 2 => status = GameStatus::from_str(&values[1]),
            "players" if values.len() >= 2 => players = values[1].parse().ok(),
            "recruiting" if values.len() >= 2 => recruiting = RecruitingMode::from_str(&values[1]),
            "open-seats" if values.len() >= 2 => open_seats = values[1].parse().ok(),
            "year" if values.len() >= 2 => year = values[1].parse().ok(),
            "turn" if values.len() >= 2 => turn = values[1].parse().ok(),
            "summary" if values.len() >= 2 => summary = Some(values[1].clone()),
            "host-alias" if values.len() >= 2 => host_alias = Some(values[1].clone()),
            "slot" if values.len() >= 5 => {
                let seat = values[1].parse().ok()?;
                let invite_code_hash = values[2].clone();
                let player_npub = Some(values[3].clone()).filter(|v| !v.is_empty());
                let status = SeatStatus::from_str(&values[4])?;
                slots.push(SeatSlot {
                    seat,
                    invite_code_hash,
                    player_npub,
                    status,
                });
            }
            _ => {}
        }
    }

    Some(GameDefinition {
        game_id: game_id?,
        game_name: game_name?,
        status: status?,
        players: players?,
        recruiting: recruiting?,
        open_seats: open_seats?,
        year: year?,
        turn: turn?,
        summary,
        host_alias,
        slots,
    })
}

pub fn build_game_definition_tags(def: &GameDefinition) -> Vec<(&'static str, String)> {
    let mut tags = vec![
        ("d", def.game_id.clone()),
        ("name", def.game_name.clone()),
        ("status", def.status.as_str().to_string()),
        ("players", def.players.to_string()),
        ("recruiting", def.recruiting.as_str().to_string()),
        ("open-seats", def.open_seats.to_string()),
        ("year", def.year.to_string()),
        ("turn", def.turn.to_string()),
    ];

    if let Some(ref summary) = def.summary {
        tags.push(("summary", summary.clone()));
    }

    if let Some(ref alias) = def.host_alias {
        tags.push(("host-alias", alias.clone()));
    }

    for slot in &def.slots {
        let tag_values = vec![
            slot.seat.to_string(),
            slot.invite_code_hash.clone(),
            slot.player_npub.clone().unwrap_or_default(),
            slot.status.as_str().to_string(),
        ];
        tags.push(("slot", tag_values.join(" ")));
    }

    tags
}
