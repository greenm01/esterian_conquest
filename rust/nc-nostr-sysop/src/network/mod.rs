use anyhow::Result;
use nostr_sdk::prelude::*;
use tokio::sync::mpsc;

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
        Ok(())
    }

    pub async fn send_text(&self, content: &str) -> Result<()> {
        let builder = EventBuilder::text_note(content);
        self.client.send_event_builder(builder).await?;
        Ok(())
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
