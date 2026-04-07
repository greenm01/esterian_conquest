use crate::json::{escape_json_string, extract_str, extract_u32};
use crate::tags::tag_content;
use crate::timing::is_event_stale;
use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{Event, EventBuilder, Keys, Kind, PublicKey, Tag};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionUiMode {
    ClassicNcGame,
    FullscreenNcDash,
}

impl SessionUiMode {
    pub const CLASSIC_NC_GAME_WIRE: &'static str = "classic_nc_game";
    pub const FULLSCREEN_NC_DASH_WIRE: &'static str = "fullscreen_nc_dash";

    pub fn parse_wire(value: &str) -> Result<Self, String> {
        match value {
            Self::CLASSIC_NC_GAME_WIRE => Ok(Self::ClassicNcGame),
            Self::FULLSCREEN_NC_DASH_WIRE => Ok(Self::FullscreenNcDash),
            other => Err(format!("unknown session_ui {other:?}")),
        }
    }

    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::ClassicNcGame => Self::CLASSIC_NC_GAME_WIRE,
            Self::FullscreenNcDash => Self::FULLSCREEN_NC_DASH_WIRE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRequest {
    pub nonce: String,
    pub player_pubkey: String,
    pub ssh_pubkey: String,
    pub invite_code: Option<String>,
    pub game_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseSessionRequestError {
    WrongKind(u16),
    InvalidSignature,
    Stale,
    MissingNonce,
    MissingSshPubkey,
}

impl std::fmt::Display for ParseSessionRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongKind(kind) => write!(f, "expected kind 30501, got {kind}"),
            Self::InvalidSignature => write!(f, "event signature invalid"),
            Self::Stale => write!(f, "event is too old (replay prevention)"),
            Self::MissingNonce => write!(f, "missing or empty `d` tag (nonce)"),
            Self::MissingSshPubkey => write!(f, "missing or empty `ssh-pubkey` tag"),
        }
    }
}

impl std::error::Error for ParseSessionRequestError {}

pub fn parse_session_request(event: &Event) -> Result<SessionRequest, ParseSessionRequestError> {
    let kind_u16 = event.kind.as_u16();
    if kind_u16 != 30501 {
        return Err(ParseSessionRequestError::WrongKind(kind_u16));
    }
    if !event.verify_signature() {
        return Err(ParseSessionRequestError::InvalidSignature);
    }
    if is_event_stale(event) {
        return Err(ParseSessionRequestError::Stale);
    }

    let nonce = tag_content(&event.tags, "d")
        .filter(|s| !s.is_empty())
        .ok_or(ParseSessionRequestError::MissingNonce)?
        .to_string();
    let ssh_pubkey = tag_content(&event.tags, "ssh-pubkey")
        .filter(|s| !s.is_empty())
        .ok_or(ParseSessionRequestError::MissingSshPubkey)?
        .to_string();
    let invite_code = {
        let content = event.content.trim().to_string();
        if content.is_empty() {
            None
        } else {
            Some(content)
        }
    };
    let game_id = tag_content(&event.tags, "game-id")
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    Ok(SessionRequest {
        nonce,
        player_pubkey: event.pubkey.to_hex(),
        ssh_pubkey,
        invite_code,
        game_id,
    })
}

pub fn build_session_request_event(
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    nonce: &str,
    ssh_pubkey: &str,
    invite_code: Option<&str>,
    game_id: Option<&str>,
) -> Result<Event, Box<dyn std::error::Error + Send + Sync>> {
    let mut tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", &gate_pubkey.to_hex()])?,
        Tag::parse(["ssh-pubkey", ssh_pubkey])?,
    ];
    if let Some(game_id) = game_id {
        tags.push(Tag::parse(["game-id", game_id])?);
    }
    Ok(
        EventBuilder::new(Kind::Custom(30501), invite_code.unwrap_or(""))
            .tags(tags)
            .sign_with_keys(player_keys)?,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReadyPayload {
    pub game_id: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub host_fingerprint: String,
    pub game_name: String,
    pub seat: u32,
    pub player_name: String,
    pub session_ui: SessionUiMode,
}

impl SessionReadyPayload {
    pub fn to_json(&self) -> String {
        let game_id = escape_json_string(&self.game_id);
        let ssh_host = escape_json_string(&self.ssh_host);
        let ssh_user = escape_json_string(&self.ssh_user);
        let host_fingerprint = escape_json_string(&self.host_fingerprint);
        let game_name = escape_json_string(&self.game_name);
        let player_name = escape_json_string(&self.player_name);
        let session_ui = self.session_ui.as_wire();
        format!(
            r#"{{"game_id":"{game_id}","ssh_host":"{ssh_host}","ssh_port":{ssh_port},"ssh_user":"{ssh_user}","host_fingerprint":"{host_fingerprint}","game_name":"{game_name}","seat":{seat},"player_name":"{player_name}","session_ui":"{session_ui}"}}"#,
            ssh_port = self.ssh_port,
            seat = self.seat,
        )
    }
}

pub fn parse_session_ready(json: &str) -> Result<SessionReadyPayload, String> {
    let game_id = extract_str(json, "game_id")?;
    let ssh_host = extract_str(json, "ssh_host")?;
    let ssh_port = extract_u32(json, "ssh_port")
        .map(|value| value as u16)
        .ok_or("missing or invalid ssh_port")?;
    let ssh_user = extract_str(json, "ssh_user").unwrap_or_default();
    let host_fingerprint = extract_str(json, "host_fingerprint").unwrap_or_default();
    let game_name = extract_str(json, "game_name")?;
    let seat = extract_u32(json, "seat").ok_or("missing or invalid seat")?;
    let player_name = extract_str(json, "player_name").unwrap_or_default();
    let session_ui = SessionUiMode::parse_wire(&extract_str(json, "session_ui")?)?;

    Ok(SessionReadyPayload {
        game_id,
        ssh_host,
        ssh_port,
        ssh_user,
        host_fingerprint,
        game_name,
        seat,
        player_name,
        session_ui,
    })
}

pub fn build_session_ready_event(
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    payload: &SessionReadyPayload,
) -> Result<Event, Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = payload.to_json();
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;
    let tags = vec![
        Tag::parse(["d", session_nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];
    Ok(EventBuilder::new(Kind::Custom(30502), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEntry {
    pub game_id: String,
    pub name: String,
    pub seat: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionErrorPayload {
    pub error: String,
    pub message: String,
    pub games: Vec<GameEntry>,
}

impl SessionErrorPayload {
    pub fn to_json(&self) -> String {
        let error = escape_json_string(&self.error);
        let message = escape_json_string(&self.message);
        if self.error == "multiple_games" {
            let entries = self
                .games
                .iter()
                .map(game_entry_json)
                .collect::<Vec<_>>()
                .join(",");
            format!(r#"{{"error":"{error}","message":"{message}","games":[{entries}]}}"#)
        } else {
            format!(r#"{{"error":"{error}","message":"{message}"}}"#)
        }
    }
}

pub fn parse_session_error(json: &str) -> Result<SessionErrorPayload, String> {
    let error = extract_str(json, "error")?;
    let message = extract_str(json, "message")?;
    let games = if error == "multiple_games" {
        parse_game_entries(json)
    } else {
        Vec::new()
    };
    Ok(SessionErrorPayload {
        error,
        message,
        games,
    })
}

pub fn build_session_error_event(
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    session_nonce: &str,
    payload: &SessionErrorPayload,
) -> Result<Event, Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = payload.to_json();
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;
    let tags = vec![
        Tag::parse(["d", session_nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];
    Ok(EventBuilder::new(Kind::Custom(30503), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?)
}

fn parse_game_entries(json: &str) -> Vec<GameEntry> {
    let mut entries = Vec::new();
    let Some(arr_start) = json.find("\"games\"") else {
        return entries;
    };
    let after = &json[arr_start + 7..];
    let Some(bracket) = after.find('[') else {
        return entries;
    };
    let arr_body = &after[bracket + 1..];
    let mut remaining = arr_body;
    while let Some(obj_start) = remaining.find('{') {
        let body = &remaining[obj_start + 1..];
        let Some(obj_end) = body.find('}') else {
            break;
        };
        let wrapped = format!("{{{}}}", &body[..obj_end]);
        let game_id = extract_str(&wrapped, "game_id").unwrap_or_default();
        let name = extract_str(&wrapped, "name").unwrap_or_default();
        let seat = extract_u32(&wrapped, "seat").unwrap_or(0);
        if !game_id.is_empty() {
            entries.push(GameEntry {
                game_id,
                name,
                seat,
            });
        }
        remaining = &body[obj_end + 1..];
    }
    entries
}

fn game_entry_json(entry: &GameEntry) -> String {
    let game_id = escape_json_string(&entry.game_id);
    let name = escape_json_string(&entry.name);
    format!(
        r#"{{"game_id":"{game_id}","name":"{name}","seat":{seat}}}"#,
        seat = entry.seat
    )
}
