use std::time::Duration;

use nc_nostr::claim::{
    SeatClaimResultPayload,
};
use nc_nostr::game_definition::{GameDefinition, parse_game_definition};
use nc_nostr::invite_request::{
    InviteDecisionPayload, InviteRequestReceipt,
};
use nc_nostr::state_sync::{GameState, StateDelta};
use nc_nostr::tags::tag_content;
use nc_nostr::turn_commands::TurnReceipt;
use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, Event, EventBuilder, Filter, Kind, Keys, PublicKey, Tag, Timestamp};

#[derive(Debug, Clone)]
pub struct HostedClientSession {
    keys: Keys,
    relay_url: String,
}

#[derive(Debug, Clone)]
pub struct CatalogGame {
    pub daemon_pubkey: String,
    pub definition: GameDefinition,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerEventBatch {
    pub receipts: Vec<InviteRequestReceipt>,
    pub decisions: Vec<InviteDecisionPayload>,
    pub claim_results: Vec<SeatClaimResultPayload>,
    pub states: Vec<GameState>,
    pub deltas: Vec<StateDelta>,
    pub turn_receipts: Vec<TurnReceipt>,
}

impl HostedClientSession {
    pub fn new(keys: Keys, relay_url: impl Into<String>) -> Self {
        Self {
            keys,
            relay_url: relay_url.into(),
        }
    }

    pub fn relay_url(&self) -> &str {
        &self.relay_url
    }

    pub fn public_key_hex(&self) -> String {
        self.keys.public_key().to_hex()
    }

    pub fn fetch_catalog(&self) -> Result<Vec<CatalogGame>, Box<dyn std::error::Error>> {
        self.with_client(async move |client| {
            let events = client
                .fetch_events(Filter::new().kinds([Kind::Custom(30500)]), Duration::from_secs(8))
                .await?;
            Ok(events
                .iter()
                .filter_map(|event| {
                    parse_game_definition(event).map(|definition| CatalogGame {
                        daemon_pubkey: event.pubkey.to_hex(),
                        definition,
                    })
                })
                .collect())
        })
    }

    pub fn refresh_player_events(
        &self,
        since_secs: u64,
    ) -> Result<PlayerEventBatch, Box<dyn std::error::Error>> {
        let player_pubkey = self.keys.public_key();
        let secret_key = self.keys.secret_key().clone();
        self.with_client(async move |client| {
            let filter = Filter::new()
                .kinds([
                    Kind::Custom(30511),
                    Kind::Custom(30514),
                    Kind::Custom(30515),
                    Kind::Custom(30520),
                    Kind::Custom(30521),
                    Kind::Custom(30524),
                ])
                .pubkeys(vec![player_pubkey])
                .since(Timestamp::now() - Duration::from_secs(since_secs));
            let events = client.fetch_events(filter, Duration::from_secs(8)).await?;
            let mut batch = PlayerEventBatch::default();
            for event in events.iter() {
                match event.kind.as_u16() {
                    30511 => {
                        if let Some(payload) =
                            decrypt_json::<SeatClaimResultPayload>(&secret_key, event)
                        {
                            batch.claim_results.push(payload);
                        }
                    }
                    30514 => {
                        if let Some(payload) =
                            decrypt_json::<InviteRequestReceipt>(&secret_key, event)
                        {
                            batch.receipts.push(payload);
                        }
                    }
                    30515 => {
                        if let Some(payload) =
                            decrypt_json::<InviteDecisionPayload>(&secret_key, event)
                        {
                            batch.decisions.push(payload);
                        }
                    }
                    30520 => {
                        if let Some(payload) = decrypt_json::<GameState>(&secret_key, event) {
                            batch.states.push(payload);
                        }
                    }
                    30521 => {
                        if let Some(payload) = decrypt_json::<StateDelta>(&secret_key, event) {
                            batch.deltas.push(payload);
                        }
                    }
                    30524 => {
                        if let Some(payload) = decrypt_json::<TurnReceipt>(&secret_key, event) {
                            batch.turn_receipts.push(payload);
                        }
                    }
                    _ => {}
                }
            }
            Ok(batch)
        })
    }

    pub fn send_invite_request(
        &self,
        game_id: &str,
        daemon_pubkey: &str,
        message: &str,
        handle: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let request_id = random_nonce_hex();
        let event = build_plain_event(
            &self.keys,
            30513,
            message,
            &request_id,
            Some(game_id),
            daemon_pubkey,
            handle,
        )?;
        self.send_signed_event(event)?;
        Ok(request_id)
    }

    pub fn claim_invite(
        &self,
        game_id: &str,
        daemon_pubkey: &str,
        invite_address: &str,
        handle: Option<&str>,
    ) -> Result<SeatClaimResultPayload, Box<dyn std::error::Error>> {
        let nonce = random_nonce_hex();
        let daemon_pubkey = PublicKey::parse(daemon_pubkey)?;
        let result = self.with_client(async move |client| {
            let mut notifications = client.notifications();
            let subscription = client
                .subscribe(
                    Filter::new()
                        .kinds([Kind::Custom(30511)])
                        .author(daemon_pubkey.clone())
                        .pubkeys(vec![self.keys.public_key()])
                        .since(Timestamp::now() - Duration::from_secs(15)),
                    None,
                )
                .await?
                .val;
            let event = build_plain_event(
                &self.keys,
                30510,
                &nonce,
                invite_address,
                Some(game_id),
                &daemon_pubkey.to_hex(),
                handle,
            )?;
            client.send_event(&event).await?;

            let timeout = tokio::time::Instant::now() + Duration::from_secs(15);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(timeout) => {
                        return Err("seat claim timed out".into());
                    }
                    notification = notifications.recv() => {
                        match notification {
                            Ok(nostr_sdk::RelayPoolNotification::Event { subscription_id, event, .. }) if subscription_id == subscription => {
                                let event = *event;
                                if tag_content(&event.tags, "d") != Some(nonce.as_str()) {
                                    continue;
                                }
                                if let Some(payload) = decrypt_json::<SeatClaimResultPayload>(self.keys.secret_key(), &event) {
                                    client.unsubscribe(&subscription).await;
                                    return Ok(payload);
                                }
                            }
                            Ok(_) => {}
                            Err(_) => return Err("seat claim stream closed".into()),
                        }
                    }
                }
            }
        })?;
        Ok(result)
    }

    pub fn request_state(
        &self,
        game_id: &str,
        daemon_pubkey: &str,
        last_turn: Option<u32>,
        last_hash: Option<&str>,
        handle: Option<&str>,
    ) -> Result<GameState, Box<dyn std::error::Error>> {
        let request_id = random_nonce_hex();
        let daemon_pubkey = PublicKey::parse(daemon_pubkey)?;
        self.with_client(async move |client| {
            let mut notifications = client.notifications();
            let subscription = client
                .subscribe(
                    Filter::new()
                        .kinds([Kind::Custom(30520), Kind::Custom(30521)])
                        .author(daemon_pubkey.clone())
                        .pubkeys(vec![self.keys.public_key()])
                        .since(Timestamp::now() - Duration::from_secs(15)),
                    None,
                )
                .await?
                .val;
            let content = serde_json::json!({
                "last_turn": last_turn,
                "last_hash": last_hash,
            })
            .to_string();
            let event = build_plain_event(
                &self.keys,
                30507,
                &content,
                &request_id,
                Some(game_id),
                &daemon_pubkey.to_hex(),
                handle,
            )?;
            client.send_event(&event).await?;

            let timeout = tokio::time::Instant::now() + Duration::from_secs(15);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(timeout) => return Err("state request timed out".into()),
                    notification = notifications.recv() => {
                        match notification {
                            Ok(nostr_sdk::RelayPoolNotification::Event { subscription_id, event, .. }) if subscription_id == subscription => {
                                let event = *event;
                                if tag_content(&event.tags, "game-id") != Some(game_id) {
                                    continue;
                                }
                                if event.kind.as_u16() == 30520 {
                                    if let Some(state) = decrypt_json::<GameState>(self.keys.secret_key(), &event) {
                                        client.unsubscribe(&subscription).await;
                                        return Ok(state);
                                    }
                                }
                            }
                            Ok(_) => {}
                            Err(_) => return Err("state request stream closed".into()),
                        }
                    }
                }
            }
        })
    }

    pub fn submit_turn(
        &self,
        game_id: &str,
        daemon_pubkey: &str,
        turn: u32,
        commands: &str,
        handle: Option<&str>,
    ) -> Result<TurnReceipt, Box<dyn std::error::Error>> {
        let submit_id = random_nonce_hex();
        let daemon_pubkey = PublicKey::parse(daemon_pubkey)?;
        self.with_client(async move |client| {
            let mut notifications = client.notifications();
            let subscription = client
                .subscribe(
                    Filter::new()
                        .kinds([Kind::Custom(30524)])
                        .author(daemon_pubkey.clone())
                        .pubkeys(vec![self.keys.public_key()])
                        .since(Timestamp::now() - Duration::from_secs(15)),
                    None,
                )
                .await?
                .val;

            let event = build_plain_turn_event(
                &self.keys,
                &submit_id,
                game_id,
                turn,
                commands,
                &daemon_pubkey.to_hex(),
                handle,
            )?;
            client.send_event(&event).await?;

            let timeout = tokio::time::Instant::now() + Duration::from_secs(15);
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(timeout) => return Err("turn submit timed out".into()),
                    notification = notifications.recv() => {
                        match notification {
                            Ok(nostr_sdk::RelayPoolNotification::Event { subscription_id, event, .. }) if subscription_id == subscription => {
                                let event = *event;
                                if tag_content(&event.tags, "d") != Some(submit_id.as_str()) {
                                    continue;
                                }
                                if let Some(receipt) = decrypt_json::<TurnReceipt>(self.keys.secret_key(), &event) {
                                    client.unsubscribe(&subscription).await;
                                    return Ok(receipt);
                                }
                            }
                            Ok(_) => {}
                            Err(_) => return Err("turn receipt stream closed".into()),
                        }
                    }
                }
            }
        })
    }

    fn send_signed_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error>> {
        self.with_client(async move |client| {
            client.send_event(&event).await?;
            Ok(())
        })
    }

    fn with_client<T, F, Fut>(&self, f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnOnce(Client) -> Fut,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            let client = Client::new(self.keys.clone());
            client.add_relay(&self.relay_url).await?;
            client.connect().await;
            let result = f(client.clone()).await;
            client.disconnect().await;
            result
        })
    }
}

fn build_plain_event(
    keys: &Keys,
    kind: u16,
    content: &str,
    nonce: &str,
    game_id: Option<&str>,
    daemon_pubkey: &str,
    handle: Option<&str>,
) -> Result<Event, Box<dyn std::error::Error>> {
    let mut tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", daemon_pubkey])?,
    ];
    if let Some(game_id) = game_id {
        tags.push(Tag::parse(["game-id", game_id])?);
    }
    if let Some(handle) = handle.filter(|handle| !handle.trim().is_empty()) {
        tags.push(Tag::parse(["handle", handle.trim()])?);
    }
    Ok(EventBuilder::new(Kind::Custom(kind), content)
        .tags(tags)
        .sign_with_keys(keys)?)
}

fn build_plain_turn_event(
    keys: &Keys,
    submit_id: &str,
    game_id: &str,
    turn: u32,
    commands: &str,
    daemon_pubkey: &str,
    handle: Option<&str>,
) -> Result<Event, Box<dyn std::error::Error>> {
    let mut tags = vec![
        Tag::parse(["d", submit_id])?,
        Tag::parse(["p", daemon_pubkey])?,
        Tag::parse(["game-id", game_id])?,
        Tag::parse(["turn", &turn.to_string()])?,
    ];
    if let Some(handle) = handle.filter(|handle| !handle.trim().is_empty()) {
        tags.push(Tag::parse(["handle", handle.trim()])?);
    }
    Ok(EventBuilder::new(Kind::Custom(30522), commands)
        .tags(tags)
        .sign_with_keys(keys)?)
}

fn decrypt_json<T: serde::de::DeserializeOwned>(
    secret_key: &nostr_sdk::SecretKey,
    event: &Event,
) -> Option<T> {
    let plaintext = nip44::decrypt(secret_key, &event.pubkey, &event.content).ok()?;
    serde_json::from_str(&plaintext).ok()
}

fn random_nonce_hex() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
