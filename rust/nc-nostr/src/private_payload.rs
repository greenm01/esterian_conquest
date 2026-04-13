use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use nostr_sdk::nips::nip44;
use nostr_sdk::nips::nip44::Version;
use nostr_sdk::{Event, Keys, PublicKey, SecretKey};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt;
use std::io::Cursor;

const ENVELOPE_VERSION: u8 = 1;
const COMPRESS_THRESHOLD_BYTES: usize = 1024;
const ZSTD_LEVEL: i32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionKind {
    None,
    Zstd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedPrivateEnvelope {
    pub v: u8,
    pub compression: CompressionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_b64: Option<String>,
}

#[derive(Debug)]
pub enum PrivatePayloadError {
    Encrypt(String),
    Decrypt(String),
    Serialize(serde_json::Error),
    Deserialize(serde_json::Error),
    InvalidEnvelopeVersion(u8),
    InvalidEnvelope(String),
    Base64(base64::DecodeError),
    Compress(std::io::Error),
    Decompress(std::io::Error),
}

impl fmt::Display for PrivatePayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encrypt(err) => write!(f, "failed to encrypt private payload: {err}"),
            Self::Decrypt(err) => write!(f, "failed to decrypt private payload: {err}"),
            Self::Serialize(err) => write!(f, "failed to serialize private payload: {err}"),
            Self::Deserialize(err) => write!(f, "failed to deserialize private payload: {err}"),
            Self::InvalidEnvelopeVersion(version) => {
                write!(f, "unsupported private payload envelope version: {version}")
            }
            Self::InvalidEnvelope(err) => write!(f, "invalid private payload envelope: {err}"),
            Self::Base64(err) => write!(f, "invalid private payload base64: {err}"),
            Self::Compress(err) => write!(f, "failed to compress private payload: {err}"),
            Self::Decompress(err) => write!(f, "failed to decompress private payload: {err}"),
        }
    }
}

impl std::error::Error for PrivatePayloadError {}

pub fn encrypt_private_text(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    plaintext: &str,
) -> Result<String, PrivatePayloadError> {
    let envelope = build_envelope(plaintext)?;
    let envelope_json = serde_json::to_string(&envelope).map_err(PrivatePayloadError::Serialize)?;
    nip44::encrypt(
        sender_keys.secret_key(),
        recipient_pubkey,
        &envelope_json,
        Version::V2,
    )
    .map_err(|err| PrivatePayloadError::Encrypt(format!("{err:?}")))
}

pub fn encrypt_private_json<T: Serialize>(
    sender_keys: &Keys,
    recipient_pubkey: &PublicKey,
    value: &T,
) -> Result<String, PrivatePayloadError> {
    let json = serde_json::to_string(value).map_err(PrivatePayloadError::Serialize)?;
    encrypt_private_text(sender_keys, recipient_pubkey, &json)
}

pub fn decrypt_private_text(
    secret_key: &SecretKey,
    sender_pubkey: &PublicKey,
    ciphertext: &str,
) -> Result<String, PrivatePayloadError> {
    let decrypted = nip44::decrypt(secret_key, sender_pubkey, ciphertext)
        .map_err(|err| PrivatePayloadError::Decrypt(format!("{err:?}")))?;
    parse_envelope(&decrypted)
}

pub fn decrypt_private_text_from_event(
    secret_key: &SecretKey,
    event: &Event,
) -> Result<String, PrivatePayloadError> {
    decrypt_private_text(secret_key, &event.pubkey, &event.content)
}

pub fn decrypt_private_json<T: DeserializeOwned>(
    secret_key: &SecretKey,
    sender_pubkey: &PublicKey,
    ciphertext: &str,
) -> Result<T, PrivatePayloadError> {
    let plaintext = decrypt_private_text(secret_key, sender_pubkey, ciphertext)?;
    serde_json::from_str(&plaintext).map_err(PrivatePayloadError::Deserialize)
}

pub fn decrypt_private_json_from_event<T: DeserializeOwned>(
    secret_key: &SecretKey,
    event: &Event,
) -> Result<T, PrivatePayloadError> {
    let plaintext = decrypt_private_text_from_event(secret_key, event)?;
    serde_json::from_str(&plaintext).map_err(PrivatePayloadError::Deserialize)
}

fn build_envelope(plaintext: &str) -> Result<HostedPrivateEnvelope, PrivatePayloadError> {
    if let Some(payload_b64) = compress_if_worth_it(plaintext.as_bytes())? {
        return Ok(HostedPrivateEnvelope {
            v: ENVELOPE_VERSION,
            compression: CompressionKind::Zstd,
            payload: None,
            payload_b64: Some(payload_b64),
        });
    }

    Ok(HostedPrivateEnvelope {
        v: ENVELOPE_VERSION,
        compression: CompressionKind::None,
        payload: Some(plaintext.to_string()),
        payload_b64: None,
    })
}

fn parse_envelope(json: &str) -> Result<String, PrivatePayloadError> {
    let envelope: HostedPrivateEnvelope =
        serde_json::from_str(json).map_err(PrivatePayloadError::Deserialize)?;
    if envelope.v != ENVELOPE_VERSION {
        return Err(PrivatePayloadError::InvalidEnvelopeVersion(envelope.v));
    }

    match envelope.compression {
        CompressionKind::None => envelope.payload.ok_or_else(|| {
            PrivatePayloadError::InvalidEnvelope(
                "compression=none requires `payload`".to_string(),
            )
        }),
        CompressionKind::Zstd => {
            let payload_b64 = envelope.payload_b64.ok_or_else(|| {
                PrivatePayloadError::InvalidEnvelope(
                    "compression=zstd requires `payload_b64`".to_string(),
                )
            })?;
            let compressed = BASE64_STANDARD
                .decode(payload_b64)
                .map_err(PrivatePayloadError::Base64)?;
            let decompressed = zstd::stream::decode_all(Cursor::new(compressed))
                .map_err(PrivatePayloadError::Decompress)?;
            String::from_utf8(decompressed).map_err(|err| {
                PrivatePayloadError::InvalidEnvelope(format!(
                    "decompressed payload is not valid UTF-8: {err}"
                ))
            })
        }
    }
}

fn compress_if_worth_it(bytes: &[u8]) -> Result<Option<String>, PrivatePayloadError> {
    if bytes.len() < COMPRESS_THRESHOLD_BYTES {
        return Ok(None);
    }

    let compressed =
        zstd::stream::encode_all(Cursor::new(bytes), ZSTD_LEVEL).map_err(PrivatePayloadError::Compress)?;
    if compressed.len() >= bytes.len() {
        return Ok(None);
    }

    Ok(Some(BASE64_STANDARD.encode(compressed)))
}

