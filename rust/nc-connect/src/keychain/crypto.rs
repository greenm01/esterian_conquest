//! Keychain encryption: ChaCha20-Poly1305 with PBKDF2-HMAC-SHA256 key derivation.
//!
//! On-disk binary envelope layout:
//!   [16 bytes] random salt
//!   [12 bytes] random nonce
//!   [N bytes]  ChaCha20-Poly1305 ciphertext + 16-byte authentication tag
//!
//! Key derivation: PBKDF2-HMAC-SHA256, 100 000 iterations, 32-byte output key.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use rand::SeedableRng;
use sha2::Sha256;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// PBKDF2 iteration count.  100 000 is the OWASP-recommended minimum for
/// PBKDF2-HMAC-SHA256 as of 2023.
pub const PBKDF2_ITERATIONS: u32 = 100_000;

/// Salt length in bytes.
pub const SALT_LEN: usize = 16;

/// Nonce length for ChaCha20-Poly1305 (96-bit / 12 bytes).
pub const NONCE_LEN: usize = 12;

/// Derived key length for ChaCha20-Poly1305 (256-bit / 32 bytes).
pub const KEY_LEN: usize = 32;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during keychain crypto operations.
#[derive(Debug)]
pub enum CryptoError {
    /// Ciphertext is too short to contain the envelope header.
    TruncatedEnvelope,
    /// AEAD decryption failed (wrong password or corrupted data).
    DecryptionFailed,
    /// Encryption failed (unexpected — should not happen with valid keys).
    EncryptionFailed,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::TruncatedEnvelope => write!(f, "keychain data is too short (truncated)"),
            CryptoError::DecryptionFailed => {
                write!(f, "wrong password or corrupted keychain")
            }
            CryptoError::EncryptionFailed => write!(f, "encryption failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Derive a 32-byte ChaCha20-Poly1305 key from `password` and `salt` using
/// PBKDF2-HMAC-SHA256 with `PBKDF2_ITERATIONS` iterations.
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

/// Encrypt `plaintext` with `password` and return the binary envelope:
/// `[salt (16)][nonce (12)][ciphertext+tag]`.
pub fn encrypt_keychain(plaintext: &str, password: &str) -> Result<Vec<u8>, CryptoError> {
    let mut rng = rand::rngs::StdRng::from_entropy();

    let mut salt = [0u8; SALT_LEN];
    rng.fill_bytes(&mut salt);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut nonce_bytes);

    let key_bytes = derive_key(password, &salt);
    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| CryptoError::EncryptionFailed)?;

    let mut envelope = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&ciphertext);
    Ok(envelope)
}

/// Decrypt the binary `envelope` with `password`.
///
/// Returns the plaintext as a UTF-8 string on success, or `CryptoError` on
/// wrong password, corrupted data, or a truncated envelope.
pub fn decrypt_keychain(envelope: &[u8], password: &str) -> Result<String, CryptoError> {
    if envelope.len() < SALT_LEN + NONCE_LEN {
        return Err(CryptoError::TruncatedEnvelope);
    }

    let salt = &envelope[..SALT_LEN];
    let nonce_bytes = &envelope[SALT_LEN..SALT_LEN + NONCE_LEN];
    let ciphertext = &envelope[SALT_LEN + NONCE_LEN..];

    let key_bytes = derive_key(password, salt);
    let key = Key::from_slice(&key_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    String::from_utf8(plaintext_bytes).map_err(|_| CryptoError::DecryptionFailed)
}
