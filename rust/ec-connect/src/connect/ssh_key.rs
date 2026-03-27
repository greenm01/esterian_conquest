//! Ephemeral SSH keypair generation.
//!
//! Generates an Ed25519 keypair held only in memory and encodes the public
//! key in the OpenSSH wire format so it can be placed in a kind-30501
//! `ssh-pubkey` tag during the Nostr session handshake.
//!
//! OpenSSH public key wire encoding (RFC 4253):
//!
//! ```text
//! uint32( len("ssh-ed25519") )  ||  "ssh-ed25519"
//! uint32( 32 )                  ||  <32-byte Ed25519 public key>
//! ```
//!
//! The whole blob is base64-encoded and prefixed with `"ssh-ed25519 "`.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

/// An ephemeral Ed25519 keypair held in memory for one session.
pub struct EphemeralKeypair {
    signing_key: SigningKey,
}

impl EphemeralKeypair {
    /// Generate a fresh keypair using the OS CSPRNG.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        EphemeralKeypair { signing_key }
    }

    /// Return the public key in OpenSSH single-line format:
    /// `"ssh-ed25519 <base64>"`.
    pub fn openssh_pubkey_string(&self) -> String {
        let wire = encode_openssh_pubkey(self.signing_key.verifying_key().as_bytes());
        format!("ssh-ed25519 {}", BASE64.encode(&wire))
    }
}

// ── Wire encoding ─────────────────────────────────────────────────────────────

/// Encode an Ed25519 public key (`pubkey_bytes`, exactly 32 bytes) into the
/// OpenSSH public-key wire format (RFC 4253 § 6.6).
///
/// Layout:
/// ```text
/// [u32 big-endian: 11] [b"ssh-ed25519"] [u32 big-endian: 32] [pubkey_bytes]
/// ```
fn encode_openssh_pubkey(pubkey_bytes: &[u8; 32]) -> Vec<u8> {
    const KEY_TYPE: &[u8] = b"ssh-ed25519";
    let key_type_len = KEY_TYPE.len() as u32;
    let pubkey_len = pubkey_bytes.len() as u32;

    let mut wire = Vec::with_capacity(4 + KEY_TYPE.len() + 4 + pubkey_bytes.len());
    wire.extend_from_slice(&key_type_len.to_be_bytes());
    wire.extend_from_slice(KEY_TYPE);
    wire.extend_from_slice(&pubkey_len.to_be_bytes());
    wire.extend_from_slice(pubkey_bytes);
    wire
}

#[cfg(test)]
mod tests_internal {
    use super::*;

    #[test]
    fn wire_format_length() {
        let key_bytes = [0u8; 32];
        let wire = encode_openssh_pubkey(&key_bytes);
        // 4 + 11 + 4 + 32 = 51
        assert_eq!(wire.len(), 51);
    }

    #[test]
    fn wire_format_header() {
        let key_bytes = [0u8; 32];
        let wire = encode_openssh_pubkey(&key_bytes);
        // First 4 bytes: u32 big-endian 11
        assert_eq!(&wire[..4], &[0, 0, 0, 11]);
        // Next 11 bytes: "ssh-ed25519"
        assert_eq!(&wire[4..15], b"ssh-ed25519");
        // Next 4 bytes: u32 big-endian 32
        assert_eq!(&wire[15..19], &[0, 0, 0, 32]);
        // Remaining 32 bytes: the key itself
        assert_eq!(&wire[19..], &[0u8; 32]);
    }
}
