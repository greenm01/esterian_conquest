use nostr_sdk::{Client, EventBuilder, Keys, Kind, PublicKey, Tag};
use std::sync::Arc;

pub struct EventPublisher {
    client: Arc<Client>,
    keys: Keys,
}

impl EventPublisher {
    pub fn new(client: Client, keys: Keys) -> Self {
        Self {
            client: Arc::new(client),
            keys,
        }
    }

    pub async fn publish(
        &self,
        kind: u32,
        content: &str,
        tags: Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut tag_objects = Vec::new();
        for (key, value) in tags {
            tag_objects.push(Tag::parse([key, value])?);
        }

        let event = EventBuilder::new(Kind::Custom(kind as u16), content)
            .tags(tag_objects)
            .sign_with_keys(&self.keys)?;

        self.client.send_event(&event).await?;
        tracing::info!("Published event kind {} to relay", kind);
        Ok(())
    }

    pub async fn publish_to_pubkey(
        &self,
        recipient_pubkey: &str,
        kind: u32,
        content: &str,
        tags: Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _public_key = PublicKey::from_hex(recipient_pubkey)?;
        let mut tag_objects = Vec::new();

        tag_objects.push(Tag::parse(["p", recipient_pubkey])?);

        for (key, value) in tags {
            tag_objects.push(Tag::parse([key, value])?);
        }

        let event = EventBuilder::new(Kind::Custom(kind as u16), content)
            .tags(tag_objects)
            .sign_with_keys(&self.keys)?;

        self.client.send_event(&event).await?;
        tracing::info!("Published event kind {} to {} on relay", kind, recipient_pubkey);
        Ok(())
    }

    pub async fn publish_encrypted(
        &self,
        recipient_pubkey: &str,
        kind: u32,
        content: &str,
        tags: Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use nostr_sdk::nips::nip44;
        use nostr_sdk::nips::nip44::Version;

        let public_key = PublicKey::from_hex(recipient_pubkey)?;
        let encrypted = nip44::encrypt(
            self.keys.secret_key(),
            &public_key,
            content,
            Version::V2,
        )?;

        let mut tag_objects = Vec::new();
        tag_objects.push(Tag::parse(["p", recipient_pubkey])?);

        for (key, value) in tags {
            tag_objects.push(Tag::parse([key, value])?);
        }

        let event = EventBuilder::new(Kind::Custom(kind as u16), &encrypted)
            .tags(tag_objects)
            .sign_with_keys(&self.keys)?;

        self.client.send_event(&event).await?;
        tracing::info!("Published encrypted event kind {} to {}", kind, recipient_pubkey);
        Ok(())
    }
}
