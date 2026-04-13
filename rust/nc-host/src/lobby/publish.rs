use nostr_sdk::{Client, EventBuilder, Keys, Kind, PublicKey, Tag};
use std::sync::Arc;

use crate::support::pubkeys::short_pubkey;

#[derive(Clone)]
pub struct EventPublisher {
    client: Arc<Client>,
    keys: Arc<Keys>,
}

impl EventPublisher {
    pub fn new(client: Client, keys: Keys) -> Self {
        Self {
            client: Arc::new(client),
            keys: Arc::new(keys),
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
            .sign_with_keys(self.keys.as_ref())?;

        self.client.send_event(&event).await?;
        tracing::info!("Published event kind {} to relay", kind);
        Ok(())
    }

    pub async fn publish_multi(
        &self,
        kind: u32,
        content: &str,
        tags: Vec<Vec<String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tag_objects = tags
            .into_iter()
            .map(Tag::parse)
            .collect::<Result<Vec<_>, _>>()?;

        let event = EventBuilder::new(Kind::Custom(kind as u16), content)
            .tags(tag_objects)
            .sign_with_keys(self.keys.as_ref())?;

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
            .sign_with_keys(self.keys.as_ref())?;

        self.client.send_event(&event).await?;
        tracing::info!(
            "Published event kind {} to {} on relay",
            kind,
            short_pubkey(recipient_pubkey)
        );
        Ok(())
    }

    pub async fn publish_encrypted(
        &self,
        recipient_pubkey: &str,
        kind: u32,
        content: &str,
        tags: Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let public_key = PublicKey::from_hex(recipient_pubkey)?;
        let encrypted = nc_nostr::private_payload::encrypt_private_text(
            self.keys.as_ref(),
            &public_key,
            content,
        )?;

        let mut tag_objects = Vec::new();
        tag_objects.push(Tag::parse(["p", recipient_pubkey])?);

        for (key, value) in tags {
            tag_objects.push(Tag::parse([key, value])?);
        }

        let event = EventBuilder::new(Kind::Custom(kind as u16), &encrypted)
            .tags(tag_objects)
            .sign_with_keys(self.keys.as_ref())?;

        self.client.send_event(&event).await?;
        tracing::info!(
            "Published encrypted event kind {} to {}",
            kind,
            short_pubkey(recipient_pubkey)
        );
        Ok(())
    }

    pub async fn publish_encrypted_multi(
        &self,
        recipient_pubkey: &str,
        kind: u32,
        content: &str,
        tags: Vec<Vec<String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let public_key = PublicKey::from_hex(recipient_pubkey)?;
        let encrypted = nc_nostr::private_payload::encrypt_private_text(
            self.keys.as_ref(),
            &public_key,
            content,
        )?;

        let mut tag_objects = vec![Tag::parse(["p", recipient_pubkey])?];
        tag_objects.extend(
            tags.into_iter()
                .map(Tag::parse)
                .collect::<Result<Vec<_>, _>>()?,
        );

        let event = EventBuilder::new(Kind::Custom(kind as u16), &encrypted)
            .tags(tag_objects)
            .sign_with_keys(self.keys.as_ref())?;

        self.client.send_event(&event).await?;
        tracing::info!(
            "Published encrypted event kind {} to {}",
            kind,
            short_pubkey(recipient_pubkey)
        );
        Ok(())
    }

    pub async fn publish_private_dm(
        &self,
        recipient_npub: &str,
        body: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let recipient = PublicKey::parse(recipient_npub)?;
        let event = EventBuilder::private_msg(
            self.keys.as_ref(),
            recipient,
            body,
            std::iter::empty::<Tag>(),
        )
        .await?;
        self.client.send_event(&event).await?;
        tracing::info!("Published sysop summary DM to {}", short_pubkey(recipient_npub));
        Ok(())
    }
}
