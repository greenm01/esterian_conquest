use anyhow::Result;
use nostr_sdk::prelude::*;
use tokio::sync::mpsc;
use chrono::Utc;

pub struct SysopClient {
    pub client: Client,
    pub keys: Keys,
}

pub enum NetworkEvent {
    Connected,
    MessageReceived {
        sender: String,
        content: String,
        channel: crate::app::SysopChannel,
        is_direct: bool,
    },
    GameDiscovered {
        id: String,
        name: String,
    },
    Error(String),
}

impl SysopClient {
    pub async fn new(nsec: &str) -> Result<Self> {
        let keys = Keys::parse(nsec)?;
        Self::new_with_keys(keys).await
    }

    pub async fn new_with_keys(keys: Keys) -> Result<Self> {
        let client = Client::new(keys.clone());
        
        Ok(Self {
            client,
            keys,
        })
    }

    pub async fn connect(&self, relays: Vec<String>) -> Result<()> {
        for relay in relays {
            self.client.add_relay(relay).await?;
        }
        self.client.connect().await;

        // Subscribe to relevant sysop events:
        let sysop_pubkey = self.keys.public_key();
        
        let dm_filter = Filter::new()
            .kind(Kind::Custom(30518))
            .pubkey(sysop_pubkey)
            .limit(50); // Get last 50 DMs

        let thread_filter = Filter::new()
            .kind(Kind::Custom(30517))
            .pubkey(sysop_pubkey)
            .limit(50);

        let invite_filter = Filter::new()
            .kind(Kind::Custom(30515))
            .pubkey(sysop_pubkey);

        let game_def_filter = Filter::new()
            .kind(Kind::Custom(30500))
            .limit(20);

        let global_chat_filter = Filter::new()
            .kind(Kind::TextNote)
            .limit(50); // Get last 50 global messages

        // In nostr-sdk 0.44, we can pass multiple filters to subscribe
        let _ = self.client.subscribe(dm_filter, None).await;
        let _ = self.client.subscribe(thread_filter, None).await;
        let _ = self.client.subscribe(invite_filter, None).await;
        let _ = self.client.subscribe(game_def_filter, None).await;
        let _ = self.client.subscribe(global_chat_filter, None).await;
        
        Ok(())
    }

    pub async fn send_text(&self, channel: &crate::app::SysopChannel, content: &str) -> Result<()> {
        match channel {
            crate::app::SysopChannel::Global => {
                let builder = EventBuilder::text_note(content);
                self.client.send_event_builder(builder).await?;
            }
            crate::app::SysopChannel::Game(game_id) => {
                let tag = Tag::custom(TagKind::Custom(std::borrow::Cow::Borrowed("g")), [game_id]);
                let builder = EventBuilder::text_note(content).tag(tag);
                self.client.send_event_builder(builder).await?;
            }
            crate::app::SysopChannel::Direct(npub) => {
                let recipient_pubkey = PublicKey::from_bech32(npub)?;
                let message_id = self.random_nonce_hex();
                let payload = nc_nostr::contact_message::ContactMessage {
                    message_id: message_id.clone(),
                    sender_pubkey: self.keys.public_key().to_hex(),
                    sender_npub: self.keys.public_key().to_bech32()?,
                    sender_label: Some("sysop".to_string()),
                    body: content.to_string(),
                    created_at: Utc::now().timestamp(),
                };
                
                let encrypted = nc_nostr::private_payload::encrypt_private_json(&self.keys, &recipient_pubkey, &payload)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                
                let tags = vec![
                    Tag::custom(TagKind::d(), [message_id]),
                    Tag::public_key(recipient_pubkey),
                ];
                
                let builder = EventBuilder::new(Kind::Custom(30518), encrypted).tags(tags);
                self.client.send_event_builder(builder).await?;
            }
        }
        Ok(())
    }

    fn random_nonce_hex(&self) -> String {
        use rand::RngCore;
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub async fn start_listening(&self, tx: mpsc::UnboundedSender<NetworkEvent>) -> Result<()> {
        let mut notifications = self.client.notifications();
        let secret_key = self.keys.secret_key().clone();
        
        tokio::spawn(async move {
            while let Ok(notification) = notifications.recv().await {
                if let RelayPoolNotification::Event { event, .. } = notification {
                    match event.kind {
                        Kind::TextNote => {
                            let mut channel = crate::app::SysopChannel::Global;
                            for tag in event.tags.iter() {
                                let parts = tag.as_slice();
                                if parts.len() >= 2 && parts[0] == "g" {
                                    channel = crate::app::SysopChannel::Game(parts[1].to_string());
                                    break;
                                }
                            }
                            let _ = tx.send(NetworkEvent::MessageReceived {
                                sender: event.pubkey.to_string()[..12].to_string(),
                                content: event.content.to_string(),
                                channel,
                                is_direct: false,
                            });
                        }
                        Kind::Custom(30518) => {
                            // Decrypt Sysop DM
                            if let Some(msg) = nc_nostr::contact_message::decrypt_contact_message(&secret_key, &event) {
                                let _ = tx.send(NetworkEvent::MessageReceived {
                                    sender: msg.sender_label.unwrap_or_else(|| msg.sender_npub[..12].to_string()),
                                    content: msg.body,
                                    channel: crate::app::SysopChannel::Direct(msg.sender_npub),
                                    is_direct: true,
                                });
                            }
                        }
                        Kind::Custom(30517) => {
                            // Decrypt Sysop Thread Message (Game Inbox)
                            if let Some(msg) = nc_nostr::thread_message::decrypt_thread_message(&secret_key, &event) {
                                let _ = tx.send(NetworkEvent::MessageReceived {
                                    sender: msg.sender_handle.unwrap_or_else(|| msg.sender_npub[..12].to_string()),
                                    content: format!("[INBOX] {}", msg.body),
                                    channel: crate::app::SysopChannel::Game(msg.game_id),
                                    is_direct: true,
                                });
                            }
                        }
                        Kind::Custom(30515) => {
                            // Decrypt Invite Request
                            if let Some(req) = nc_nostr::invite_request::parse_invite_request(&secret_key, &event) {
                                let _ = tx.send(NetworkEvent::MessageReceived {
                                    sender: req.handle.unwrap_or_else(|| req.player_pubkey[..12].to_string()),
                                    content: format!("[REQUEST] {}", req.message),
                                    channel: crate::app::SysopChannel::Game(req.game_id),
                                    is_direct: true,
                                });
                            }
                        }
                        Kind::Custom(30500) => {
                            // Extract game ID and name from tags
                            let mut id = None;
                            let mut name = None;
                            for tag in event.tags.iter() {
                                let parts = tag.as_slice();
                                if parts.len() >= 2 {
                                    if parts[0] == "d" {
                                        id = Some(parts[1].to_string());
                                    } else if parts[0] == "name" {
                                        name = Some(parts[1].to_string());
                                    }
                                }
                            }
                            if let (Some(id), Some(name)) = (id, name) {
                                let _ = tx.send(NetworkEvent::GameDiscovered { id, name });
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
        
        Ok(())
    }
}
