//! Bech32-encoded invite code for Esterian Conquest.
//!
//! An `ecinv1...` string is a self-contained invite that embeds the Nostr relay
//! URL and the two-word invite slug so players can join with a single paste.
//!
//! # Wire format (bech32m, HRP = `ecinv`)
//!
//! ```text
//! byte  0:        version — must be 0x01
//! bytes 1..3:     relay URL length as u16 big-endian
//! bytes 3..N:     relay URL (UTF-8)
//! byte  N:        invite words length as u8
//! bytes N+1..M:   invite words (UTF-8, e.g. "velvet-mountain")
//! byte  M:        flags
//!                   bit 0 = game_id present
//!                   bit 1 = gate_npub present
//! [if bit 0] u8 length + UTF-8 game_id
//! [if bit 1] 32 bytes raw gate pubkey
//! ```

use bech32::{Bech32m, Hrp, decode, encode};

const HRP: Hrp = Hrp::parse_unchecked("ecinv");
const VERSION: u8 = 0x01;

/// Decoded invite payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitePayload {
    /// Nostr relay WebSocket URL, e.g. `"wss://play.example.com:7777"`.
    pub relay_url: String,
    /// Two-word invite slug, e.g. `"velvet-mountain"`.
    pub words: String,
    /// Optional game ID hint — skips the discovery query if present.
    pub game_id: Option<String>,
    /// Optional gate public key (raw 32 bytes) — skips the 30500 discovery entirely.
    pub gate_npub: Option<[u8; 32]>,
}

/// Encode an `InvitePayload` as an `ecinv1...` bech32m string.
pub fn encode_invite(payload: &InvitePayload) -> Result<String, String> {
    let relay_bytes = payload.relay_url.as_bytes();
    let words_bytes = payload.words.as_bytes();

    if relay_bytes.len() > u16::MAX as usize {
        return Err("relay URL too long".into());
    }
    if words_bytes.len() > u8::MAX as usize {
        return Err("invite words too long".into());
    }

    let mut flags: u8 = 0;
    if payload.game_id.is_some() {
        flags |= 0x01;
    }
    if payload.gate_npub.is_some() {
        flags |= 0x02;
    }

    let relay_len = relay_bytes.len() as u16;
    let words_len = words_bytes.len() as u8;

    let mut data: Vec<u8> = Vec::new();
    data.push(VERSION);
    data.extend_from_slice(&relay_len.to_be_bytes());
    data.extend_from_slice(relay_bytes);
    data.push(words_len);
    data.extend_from_slice(words_bytes);
    data.push(flags);

    if let Some(game_id) = &payload.game_id {
        let gb = game_id.as_bytes();
        if gb.len() > u8::MAX as usize {
            return Err("game_id too long".into());
        }
        data.push(gb.len() as u8);
        data.extend_from_slice(gb);
    }
    if let Some(npub) = &payload.gate_npub {
        data.extend_from_slice(npub);
    }

    encode::<Bech32m>(HRP, &data).map_err(|e| format!("bech32 encode: {e}"))
}

/// Decode an `ecinv1...` bech32m string into an `InvitePayload`.
pub fn decode_invite(encoded: &str) -> Result<InvitePayload, String> {
    let (hrp, data) = decode(encoded).map_err(|e| format!("bech32 decode: {e}"))?;

    if hrp != HRP {
        return Err(format!("unexpected HRP '{}', expected 'ecinv'", hrp.as_str()));
    }

    let mut pos = 0;

    let version = *data.get(pos).ok_or("truncated: missing version")?;
    if version != VERSION {
        return Err(format!("unsupported version {version}"));
    }
    pos += 1;

    // Relay URL.
    if pos + 2 > data.len() {
        return Err("truncated: relay URL length".into());
    }
    let relay_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    if pos + relay_len > data.len() {
        return Err("truncated: relay URL body".into());
    }
    let relay_url =
        std::str::from_utf8(&data[pos..pos + relay_len]).map_err(|_| "relay URL is not UTF-8")?;
    pos += relay_len;

    // Invite words.
    let words_len = *data.get(pos).ok_or("truncated: words length")? as usize;
    pos += 1;
    if pos + words_len > data.len() {
        return Err("truncated: words body".into());
    }
    let words =
        std::str::from_utf8(&data[pos..pos + words_len]).map_err(|_| "words are not UTF-8")?;
    pos += words_len;

    // Flags.
    let flags = *data.get(pos).ok_or("truncated: flags")? ;
    pos += 1;

    // Optional game_id.
    let game_id = if flags & 0x01 != 0 {
        let len = *data.get(pos).ok_or("truncated: game_id length")? as usize;
        pos += 1;
        if pos + len > data.len() {
            return Err("truncated: game_id body".into());
        }
        let s = std::str::from_utf8(&data[pos..pos + len])
            .map_err(|_| "game_id is not UTF-8")?;
        pos += len;
        Some(s.to_string())
    } else {
        None
    };

    // Optional gate npub.
    let gate_npub = if flags & 0x02 != 0 {
        if pos + 32 > data.len() {
            return Err("truncated: gate npub".into());
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[pos..pos + 32]);
        Some(bytes)
    } else {
        None
    };

    Ok(InvitePayload {
        relay_url: relay_url.to_string(),
        words: words.to_string(),
        game_id,
        gate_npub,
    })
}

/// Return true if `s` looks like an `ecinv1...` bech32 invite code.
pub fn is_bech32_invite(s: &str) -> bool {
    s.starts_with("ecinv1")
}
