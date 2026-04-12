use nostr_sdk::{Event, ToBech32};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyNotice {
    pub notice_id: String,
    pub sender_npub: String,
    pub sender_handle: Option<String>,
    pub body: String,
    pub created_at: i64,
}

pub fn parse_lobby_notice(event: &Event) -> Option<LobbyNotice> {
    let mut notice: LobbyNotice = serde_json::from_str(&event.content).ok()?;
    if notice.notice_id.trim().is_empty() {
        notice.notice_id = extract_tag(event, "d")?;
    }
    if notice.sender_npub.trim().is_empty() {
        notice.sender_npub = event.pubkey.to_bech32().ok()?;
    }
    if notice.created_at == 0 {
        notice.created_at = i64::try_from(event.created_at.as_secs()).ok()?;
    }
    Some(notice)
}

pub fn build_lobby_notice_tags(notice: &LobbyNotice) -> Vec<(&'static str, String)> {
    vec![("d", notice.notice_id.clone())]
}

fn extract_tag(event: &Event, key: &str) -> Option<String> {
    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        if values.first().map(String::as_str) == Some(key) && values.len() >= 2 {
            return Some(values[1].clone());
        }
    }
    None
}
