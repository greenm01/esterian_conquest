use std::time::Duration;

use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, EventBuilder, Filter, Keys, Kind, PublicKey, Tag};
use serde::{Deserialize, Serialize};

use crate::connect::handshake::random_nonce_hex;
use crate::connect::resolve::ResolvedTarget;

pub const MAP_REQUEST_TIMEOUT_SECS: u64 = 15;

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

pub async fn fetch_map_bundle(
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    game_id: &str,
) -> Result<MapBundlePayload, String> {
    let gate_pubkey = PublicKey::parse(gate_npub).map_err(|e| format!("gate key: {e}"))?;
    let nonce = random_nonce_hex();

    let client = Client::new(player_keys.clone());
    client
        .add_relay(&target.relay_url)
        .await
        .map_err(|e| format!("add relay: {e}"))?;
    client.connect().await;

    let response_filter = Filter::new()
        .kinds([Kind::Custom(30505), Kind::Custom(30506)])
        .pubkey(player_keys.public_key());
    client
        .subscribe(response_filter, None)
        .await
        .map_err(|e| format!("subscribe: {e}"))?;

    publish_map_request(&client, player_keys, &gate_pubkey, &nonce, game_id)
        .await
        .map_err(|e| e.to_string())?;

    let timeout = Duration::from_secs(MAP_REQUEST_TIMEOUT_SECS);
    let events = client
        .fetch_events(
            Filter::new()
                .kinds([Kind::Custom(30505), Kind::Custom(30506)])
                .pubkey(player_keys.public_key()),
            timeout,
        )
        .await
        .map_err(|e| format!("fetch events: {e}"))?;

    client.disconnect().await;

    for event in events.iter() {
        let d = tag_value(event.tags.iter(), "d");
        if d.as_deref() != Some(nonce.as_str()) {
            continue;
        }

        let plaintext = nip44::decrypt(player_keys.secret_key(), &event.pubkey, &event.content)
            .map_err(|e| format!("decrypt map payload: {e}"))?;

        return match event.kind.as_u16() {
            30505 => serde_json::from_str::<MapBundlePayload>(&plaintext)
                .map_err(|e| format!("parse map bundle: {e}")),
            30506 => {
                let err = serde_json::from_str::<MapErrorPayload>(&plaintext)
                    .map_err(|e| format!("parse map error: {e}"))?;
                Err(format!("{}: {}", err.error, err.message))
            }
            other => Err(format!("unexpected map response kind: {other}")),
        };
    }

    Err("map download timed out (no response from server)".into())
}

async fn publish_map_request(
    client: &Client,
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    nonce: &str,
    game_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", &gate_pubkey.to_hex()])?,
        Tag::parse(["game-id", game_id])?,
    ];
    let event = EventBuilder::new(Kind::Custom(30504), "")
        .tags(tags)
        .sign_with_keys(player_keys)?;
    client.send_event(&event).await?;
    Ok(())
}

fn tag_value<'a>(mut tags: impl Iterator<Item = &'a nostr_sdk::Tag>, name: &str) -> Option<String> {
    tags.find_map(|tag| {
        if tag.kind().as_str() == name {
            tag.content().map(str::to_string)
        } else {
            None
        }
    })
}
