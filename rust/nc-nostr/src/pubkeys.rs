use nostr_sdk::{Event, PublicKey, ToBech32};

pub fn event_pubkey_hex(event: &Event) -> String {
    event.pubkey.to_hex()
}

pub fn event_pubkey_npub(event: &Event) -> Option<String> {
    event.pubkey.to_bech32().ok()
}

pub fn hex_to_npub(pubkey_hex: &str) -> Option<String> {
    PublicKey::parse(pubkey_hex).ok()?.to_bech32().ok()
}
