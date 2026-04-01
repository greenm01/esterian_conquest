use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use nc_data::PlayerMapExportData;
use nc_nostr::hash::sha256_hex;
use nc_nostr::tags::tag_content;
use nc_nostr::timing::is_event_stale;
use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{Client, Event, EventBuilder, Keys, Kind, PublicKey, Tag};
use serde::{Deserialize, Serialize};

use crate::serve::routing::ResolvedSeat;

pub const MAX_MAP_PAYLOAD_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct MapRequest {
    pub nonce: String,
    pub player_pubkey: String,
    pub game_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    WrongKind(u16),
    InvalidSignature,
    Stale,
    MissingNonce,
    MissingGameId,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::WrongKind(kind) => write!(f, "expected kind 30504, got {kind}"),
            ParseError::InvalidSignature => write!(f, "event signature invalid"),
            ParseError::Stale => write!(f, "event is too old (replay prevention)"),
            ParseError::MissingNonce => write!(f, "missing or empty `d` tag (nonce)"),
            ParseError::MissingGameId => write!(f, "missing or empty `game-id` tag"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapFilePayload {
    pub name: String,
    pub codec: String,
    pub sha256: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapBundlePayload {
    pub game_id: String,
    pub game_name: String,
    pub seat: u32,
    pub files: Vec<MapFilePayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapErrorPayload {
    pub error: String,
    pub message: String,
}

#[derive(Debug)]
pub enum PublishMapBundleError {
    PayloadTooLarge,
    Other(String),
}

impl std::fmt::Display for PublishMapBundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishMapBundleError::PayloadTooLarge => write!(f, "map payload exceeds size limit"),
            PublishMapBundleError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

pub fn parse_map_request(event: &Event) -> Result<MapRequest, ParseError> {
    let kind_u16 = event.kind.as_u16();
    if kind_u16 != 30504 {
        return Err(ParseError::WrongKind(kind_u16));
    }
    if !event.verify_signature() {
        return Err(ParseError::InvalidSignature);
    }

    if is_event_stale(event) {
        return Err(ParseError::Stale);
    }

    let nonce = tag_content(&event.tags, "d")
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::MissingNonce)?
        .to_string();
    let game_id = tag_content(&event.tags, "game-id")
        .filter(|s| !s.is_empty())
        .ok_or(ParseError::MissingGameId)?
        .to_string();

    Ok(MapRequest {
        nonce,
        player_pubkey: event.pubkey.to_hex(),
        game_id,
    })
}

pub async fn publish_map_bundle(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    request_nonce: &str,
    seat: &ResolvedSeat,
    export: &PlayerMapExportData,
) -> Result<String, PublishMapBundleError> {
    let payload = build_map_bundle_payload(seat, export)?;
    let plaintext = serde_json::to_string(&payload)
        .map_err(|err| PublishMapBundleError::Other(format!("serialize map payload: {err}")))?;
    if plaintext.len() > MAX_MAP_PAYLOAD_BYTES {
        return Err(PublishMapBundleError::PayloadTooLarge);
    }

    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )
    .map_err(|err| PublishMapBundleError::Other(format!("encrypt map payload: {err}")))?;

    let tags = vec![
        Tag::parse(["d", request_nonce])
            .map_err(|err| PublishMapBundleError::Other(format!("tag nonce: {err}")))?,
        Tag::parse(["p", &player_pubkey.to_hex()])
            .map_err(|err| PublishMapBundleError::Other(format!("tag player: {err}")))?,
    ];
    let event = EventBuilder::new(Kind::Custom(30505), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)
        .map_err(|err| PublishMapBundleError::Other(format!("sign map bundle: {err}")))?;

    client
        .send_event(&event)
        .await
        .map_err(|err| PublishMapBundleError::Other(format!("publish map bundle: {err}")))?;
    Ok(event.id.to_hex())
}

pub async fn publish_map_error(
    client: &Client,
    gate_keys: &Keys,
    player_pubkey: &PublicKey,
    request_nonce: &str,
    error: &str,
    message: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let payload = MapErrorPayload {
        error: error.to_string(),
        message: message.to_string(),
    };
    let plaintext = serde_json::to_string(&payload)?;
    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        player_pubkey,
        &plaintext,
        Version::V2,
    )?;
    let tags = vec![
        Tag::parse(["d", request_nonce])?,
        Tag::parse(["p", &player_pubkey.to_hex()])?,
    ];
    let event = EventBuilder::new(Kind::Custom(30506), encrypted)
        .tags(tags)
        .sign_with_keys(gate_keys)?;
    client.send_event(&event).await?;
    Ok(event.id.to_hex())
}

pub fn build_map_bundle_payload(
    seat: &ResolvedSeat,
    export: &PlayerMapExportData,
) -> Result<MapBundlePayload, PublishMapBundleError> {
    let files = export
        .fixed_named_files()
        .into_iter()
        .map(|file| {
            let bytes = file.contents.into_bytes();
            let compressed = zstd::stream::encode_all(std::io::Cursor::new(&bytes), 0)
                .map_err(|err| PublishMapBundleError::Other(format!("compress map file: {err}")))?;
            Ok(MapFilePayload {
                name: file.name.to_string(),
                codec: "zstd+base64".to_string(),
                sha256: sha256_hex(&bytes),
                content: BASE64.encode(compressed),
            })
        })
        .collect::<Result<Vec<_>, PublishMapBundleError>>()?;

    Ok(MapBundlePayload {
        game_id: seat.game_id.clone(),
        game_name: seat.game_name.clone(),
        seat: seat.player as u32,
        files,
    })
}
