use std::time::{Duration, Instant};

use nc_nostr::claim::{SeatClaimRequestPayload, SeatClaimResultPayload};
use nc_nostr::contact_message::{ContactMessage, decrypt_contact_message};
use nc_nostr::game_definition::{GameDefinition, parse_game_definition};
use nc_nostr::invite_request::{InviteDecisionPayload, InviteRequestPayload, InviteRequestReceipt};
use nc_nostr::lobby_notice::LobbyNotice;
use nc_nostr::player_message::{
    PlayerMessage, PlayerMessageRequest, decrypt_player_message,
};
use nc_nostr::private_payload::{decrypt_private_json_from_event, encrypt_private_json};
use nc_nostr::state_sync::{GameState, StateDelta, StateRequestPayload};
use nc_nostr::tags::tag_content;
use nc_nostr::thread_message::{SenderRole, SysopThreadMessage, decrypt_thread_message};
use nc_nostr::turn_commands::{TurnCommandsPayload, TurnReceipt};
use nostr_sdk::{
    Alphabet, Client, Event, EventBuilder, Filter, Keys, Kind, PublicKey, SingleLetterTag, Tag,
    Timestamp, ToBech32,
};

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
    pub contact_messages: Vec<ContactMessage>,
    pub player_messages: Vec<PlayerMessage>,
}

#[derive(Debug, Clone)]
pub enum SandboxJoinOutcome {
    Joined(GameState),
    Full(String),
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
                .fetch_events(
                    Filter::new().kinds([Kind::Custom(30500)]),
                    Duration::from_secs(8),
                )
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

    pub fn fetch_lobby_notices(
        &self,
        since_secs: u64,
    ) -> Result<Vec<LobbyNotice>, Box<dyn std::error::Error>> {
        self.with_client(async move |client| {
            let events = client
                .fetch_events(
                    Filter::new()
                        .kinds([Kind::Custom(30516)])
                        .since(Timestamp::now() - Duration::from_secs(since_secs)),
                    Duration::from_secs(8),
                )
                .await?;
            Ok(events
                .iter()
                .filter_map(nc_nostr::lobby_notice::parse_lobby_notice)
                .collect())
        })
    }

    pub fn fetch_thread_messages(
        &self,
        since_secs: u64,
    ) -> Result<Vec<SysopThreadMessage>, Box<dyn std::error::Error>> {
        let player_pubkey_hex = self.keys.public_key().to_hex();
        let secret_key = self.keys.secret_key().clone();
        self.with_client(async move |client| {
            let filter = Filter::new()
                .kinds([Kind::Custom(30517)])
                .custom_tag(
                    SingleLetterTag::lowercase(Alphabet::P),
                    player_pubkey_hex.as_str(),
                )
                .since(Timestamp::now() - Duration::from_secs(since_secs));
            let events = client.fetch_events(filter, Duration::from_secs(8)).await?;
            Ok(events
                .iter()
                .filter_map(|event| decrypt_thread_message(&secret_key, event))
                .collect())
        })
    }

    pub fn refresh_player_events(
        &self,
        since_secs: u64,
    ) -> Result<PlayerEventBatch, Box<dyn std::error::Error>> {
        let player_pubkey_hex = self.keys.public_key().to_hex();
        let secret_key = self.keys.secret_key().clone();
        self.with_client(async move |client| {
            let filter = Filter::new()
                .kinds([
                    Kind::Custom(30518),
                    Kind::Custom(30511),
                    Kind::Custom(30514),
                    Kind::Custom(30515),
                    Kind::Custom(30523),
                    Kind::Custom(30520),
                    Kind::Custom(30521),
                    Kind::Custom(30524),
                ])
                .custom_tag(
                    SingleLetterTag::lowercase(Alphabet::P),
                    player_pubkey_hex.as_str(),
                )
                .since(Timestamp::now() - Duration::from_secs(since_secs));
            let events = client.fetch_events(filter, Duration::from_secs(8)).await?;
            let mut batch = PlayerEventBatch::default();
            for event in events.iter() {
                match event.kind.as_u16() {
                    30518 => {
                        if let Some(payload) = decrypt_contact_message(&secret_key, event) {
                            batch.contact_messages.push(payload);
                        }
                    }
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
                    30523 => {
                        if let Some(payload) = decrypt_player_message(&secret_key, event) {
                            batch.player_messages.push(payload);
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
        let event = build_private_json_event(
            &self.keys,
            &PublicKey::parse(daemon_pubkey)?,
            30513,
            &InviteRequestPayload {
                message: message.to_string(),
                handle: normalize_handle(handle),
            },
            &request_id,
            Some(game_id),
        )?;
        self.send_signed_event(event)?;
        Ok(request_id)
    }

    pub fn join_sandbox_game(
        &self,
        game_id: &str,
        daemon_pubkey: &str,
        handle: Option<&str>,
    ) -> Result<SandboxJoinOutcome, Box<dyn std::error::Error>> {
        let request_id = self.send_invite_request(game_id, daemon_pubkey, "", handle)?;
        let deadline = Instant::now() + Duration::from_secs(15);

        loop {
            let batch = self.refresh_player_events(15)?;

            if let Some(receipt) = batch
                .receipts
                .iter()
                .find(|receipt| receipt.request_id == request_id)
            {
                match receipt.status {
                    nc_nostr::invite_request::InviteRequestReceiptStatus::Received => {}
                    nc_nostr::invite_request::InviteRequestReceiptStatus::GameFull => {
                        return Ok(SandboxJoinOutcome::Full(receipt.message.clone()));
                    }
                    _ => return Err(receipt.message.clone().into()),
                }
            }

            if let Some(decision) = batch
                .decisions
                .iter()
                .find(|decision| decision.request_id == request_id)
            {
                match decision.decision {
                    nc_nostr::invite_request::InviteDecision::Approved { .. } => {
                        let state =
                            self.request_state(game_id, daemon_pubkey, None, None, handle)?;
                        return Ok(SandboxJoinOutcome::Joined(state));
                    }
                    nc_nostr::invite_request::InviteDecision::Rejected => {
                        return Err(decision.message.clone().into());
                    }
                }
            }

            if Instant::now() >= deadline {
                return Err("sandbox join timed out".into());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    pub fn send_thread_message(
        &self,
        game_id: &str,
        daemon_pubkey: &str,
        body: &str,
        handle: Option<&str>,
    ) -> Result<SysopThreadMessage, Box<dyn std::error::Error>> {
        let message_id = random_nonce_hex();
        let keys = self.keys.clone();
        let public_key = PublicKey::parse(daemon_pubkey)?;
        let sender_pubkey = keys.public_key().to_hex();
        let sender_npub = keys.public_key().to_bech32()?;
        let payload = SysopThreadMessage {
            message_id: message_id.clone(),
            game_id: game_id.to_string(),
            sender_role: SenderRole::Player,
            sender_pubkey,
            sender_npub,
            sender_handle: handle
                .map(str::trim)
                .map(str::to_string)
                .filter(|value| !value.is_empty()),
            body: body.trim().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0),
        };
        self.with_client(async move |client| {
            let encrypted = encrypt_private_json(&keys, &public_key, &payload)
                .map_err(|e| format!("private thread encryption failed: {e}"))?;
            let tags = vec![
                Tag::parse(["d", &message_id])?,
                Tag::parse(["p", &public_key.to_hex()])?,
                Tag::parse(["game-id", game_id])?,
            ];
            let event = EventBuilder::new(Kind::Custom(30517), &encrypted)
                .tags(tags)
                .sign_with_keys(&keys)?;
            client.send_event(&event).await?;
            Ok(payload)
        })
    }

    pub fn send_contact_message(
        &self,
        contact_npub: &str,
        body: &str,
        label: Option<&str>,
    ) -> Result<ContactMessage, Box<dyn std::error::Error>> {
        let message_id = random_nonce_hex();
        let keys = self.keys.clone();
        let public_key = PublicKey::parse(contact_npub)?;
        let payload = ContactMessage {
            message_id: message_id.clone(),
            sender_pubkey: keys.public_key().to_hex(),
            sender_npub: keys.public_key().to_bech32()?,
            sender_label: normalize_handle(label),
            body: body.trim().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0),
        };
        self.with_client(async move |client| {
            let event = build_private_json_event(&keys, &public_key, 30518, &payload, &message_id, None)?;
            client.send_event(&event).await?;
            Ok(payload)
        })
    }

    pub fn send_player_message(
        &self,
        game_id: &str,
        daemon_pubkey: &str,
        recipient_empire_id: u8,
        body: &str,
    ) -> Result<PlayerMessageRequest, Box<dyn std::error::Error>> {
        let message_id = random_nonce_hex();
        let daemon_pubkey = PublicKey::parse(daemon_pubkey)?;
        let payload = PlayerMessageRequest {
            message_id: message_id.clone(),
            game_id: game_id.to_string(),
            sender_pubkey: self.keys.public_key().to_hex(),
            recipient_empire_id,
            body: body.trim().to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0),
        };
        let event = build_private_json_event(
            &self.keys,
            &daemon_pubkey,
            30523,
            &payload,
            &message_id,
            Some(game_id),
        )?;
        self.send_signed_event(event)?;
        Ok(payload)
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
        let player_pubkey_hex = self.keys.public_key().to_hex();
        let result = self.with_client(async move |client| {
            let mut notifications = client.notifications();
            let subscription = client
                .subscribe(
                    Filter::new()
                        .kinds([Kind::Custom(30511)])
                        .author(daemon_pubkey.clone())
                        .custom_tag(
                            SingleLetterTag::lowercase(Alphabet::P),
                            player_pubkey_hex.as_str(),
                        )
                        .since(Timestamp::now() - Duration::from_secs(15)),
                    None,
                )
                .await?
                .val;
            let event = build_private_json_event(
                &self.keys,
                &daemon_pubkey,
                30510,
                &SeatClaimRequestPayload {
                    invite: invite_address.to_string(),
                    handle: normalize_handle(handle),
                },
                &nonce,
                Some(game_id),
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
        let player_pubkey_hex = self.keys.public_key().to_hex();
        self.with_client(async move |client| {
            let mut notifications = client.notifications();
            let subscription = client
                .subscribe(
                    Filter::new()
                        .kinds([Kind::Custom(30520), Kind::Custom(30521)])
                        .author(daemon_pubkey.clone())
                        .custom_tag(
                            SingleLetterTag::lowercase(Alphabet::P),
                            player_pubkey_hex.as_str(),
                        )
                        .since(Timestamp::now() - Duration::from_secs(15)),
                    None,
                )
                .await?
                .val;
            let event = build_private_json_event(
                &self.keys,
                &daemon_pubkey,
                30507,
                &StateRequestPayload {
                    last_turn,
                    last_hash: last_hash.map(str::to_string),
                    handle: normalize_handle(handle),
                },
                &request_id,
                Some(game_id),
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
        let player_pubkey_hex = self.keys.public_key().to_hex();
        self.with_client(async move |client| {
            let mut notifications = client.notifications();
            let subscription = client
                .subscribe(
                    Filter::new()
                        .kinds([Kind::Custom(30524)])
                        .author(daemon_pubkey.clone())
                        .custom_tag(
                            SingleLetterTag::lowercase(Alphabet::P),
                            player_pubkey_hex.as_str(),
                        )
                        .since(Timestamp::now() - Duration::from_secs(15)),
                    None,
                )
                .await?
                .val;

            let event = build_private_json_turn_event(
                &self.keys,
                &daemon_pubkey,
                &submit_id,
                game_id,
                turn,
                &TurnCommandsPayload {
                    commands: commands.to_string(),
                    handle: normalize_handle(handle),
                },
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

fn build_private_json_event<T: serde::Serialize>(
    keys: &Keys,
    recipient_pubkey: &PublicKey,
    kind: u16,
    payload: &T,
    nonce: &str,
    game_id: Option<&str>,
) -> Result<Event, Box<dyn std::error::Error>> {
    let mut tags = vec![
        Tag::parse(["d", nonce])?,
        Tag::parse(["p", &recipient_pubkey.to_hex()])?,
    ];
    if let Some(game_id) = game_id {
        tags.push(Tag::parse(["game-id", game_id])?);
    }
    let encrypted = encrypt_private_json(keys, recipient_pubkey, payload)?;
    Ok(EventBuilder::new(Kind::Custom(kind), encrypted)
        .tags(tags)
        .sign_with_keys(keys)?)
}

fn build_private_json_turn_event(
    keys: &Keys,
    recipient_pubkey: &PublicKey,
    submit_id: &str,
    game_id: &str,
    turn: u32,
    payload: &TurnCommandsPayload,
) -> Result<Event, Box<dyn std::error::Error>> {
    let tags = vec![
        Tag::parse(["d", submit_id])?,
        Tag::parse(["p", &recipient_pubkey.to_hex()])?,
        Tag::parse(["game-id", game_id])?,
        Tag::parse(["turn", &turn.to_string()])?,
    ];
    let encrypted = encrypt_private_json(keys, recipient_pubkey, payload)?;
    Ok(EventBuilder::new(Kind::Custom(30522), encrypted)
        .tags(tags)
        .sign_with_keys(keys)?)
}

fn decrypt_json<T: serde::de::DeserializeOwned>(
    secret_key: &nostr_sdk::SecretKey,
    event: &Event,
) -> Option<T> {
    decrypt_private_json_from_event(secret_key, event).ok()
}

fn normalize_handle(handle: Option<&str>) -> Option<String> {
    handle
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn random_nonce_hex() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
