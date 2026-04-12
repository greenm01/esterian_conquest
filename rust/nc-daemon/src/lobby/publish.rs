use nostr_sdk::Client;
use std::sync::Arc;

pub struct EventPublisher {
    client: Arc<Client>,
}

impl EventPublisher {
    pub fn new(client: Client) -> Self {
        Self { client: Arc::new(client) }
    }

    pub async fn publish(
        &self,
        kind: u32,
        content: &str,
        tags: Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Would publish kind {} content {} tags {:?}", kind, content, tags);
        tracing::info!("Published event kind {} to relay (stub)", kind);
        Ok(())
    }

    pub async fn publish_to_pubkey(
        &self,
        recipient_pubkey: &str,
        kind: u32,
        content: &str,
        tags: Vec<(&str, &str)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::debug!("Would publish to {} kind {} content {} tags {:?}", recipient_pubkey, kind, content, tags);
        tracing::info!("Published event kind {} to {} (stub)", kind, recipient_pubkey);
        Ok(())
    }
}
