use std::time::Duration;

use ec_nostr::tags::tag_content;
use nostr_sdk::{Event, Filter, Kind, PublicKey, RelayPoolNotification, SubscriptionId, Timestamp};
use tokio::sync::broadcast;

pub fn build_response_filter<I>(
    gate_pubkey: &PublicKey,
    player_pubkey: &PublicKey,
    kinds: I,
    since: Timestamp,
) -> Filter
where
    I: IntoIterator<Item = Kind>,
{
    Filter::new()
        .kinds(kinds)
        .author(gate_pubkey.clone())
        .pubkeys(vec![player_pubkey.clone()])
        .since(since)
}

pub fn is_matching_response_event(
    event: &Event,
    expected_kinds: &[Kind],
    gate_pubkey: &PublicKey,
    player_pubkey: &PublicKey,
    nonce: &str,
) -> bool {
    expected_kinds.contains(&event.kind)
        && event.pubkey == *gate_pubkey
        && tag_content(&event.tags, "d") == Some(nonce)
        && has_pubkey_tag(event, "p", player_pubkey)
}

pub async fn wait_for_matching_response<F>(
    notifications: &mut broadcast::Receiver<RelayPoolNotification>,
    subscription_id: &SubscriptionId,
    timeout: Duration,
    matches: F,
) -> Option<Event>
where
    F: Fn(&Event) -> bool,
{
    let expected_subscription = subscription_id.clone();
    let waited = tokio::time::timeout(timeout, async {
        loop {
            match notifications.recv().await {
                Ok(RelayPoolNotification::Event {
                    subscription_id,
                    event,
                    ..
                }) => {
                    if subscription_id == expected_subscription && matches(&event) {
                        return Some(*event);
                    }
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    })
    .await;

    match waited {
        Ok(event) => event,
        Err(_) => None,
    }
}

fn has_pubkey_tag(event: &Event, name: &str, expected: &PublicKey) -> bool {
    event.tags.iter().any(|tag| {
        if tag.kind().as_str() != name {
            return false;
        }
        match tag.content() {
            Some(value) => PublicKey::parse(value)
                .map(|pubkey| pubkey == *expected)
                .unwrap_or(false),
            None => false,
        }
    })
}

