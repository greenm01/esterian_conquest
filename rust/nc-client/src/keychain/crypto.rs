use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use rand::SeedableRng;
use sha2::Sha256;

pub const PBKDF2_ITERATIONS: u32 = 100_000;
pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const KEY_LEN: usize = 32;

#[derive(Debug)]
pub enum CryptoError {
    TruncatedEnvelope,
    DecryptionFailed,
    EncryptionFailed,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedEnvelope => write!(f, "encrypted data is too short"),
            Self::DecryptionFailed => write!(f, "invalid entry, try again."),
            Self::EncryptionFailed => write!(f, "encryption failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

pub fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

pub fn encrypt_blob(plaintext: &str, password: &str) -> Result<Vec<u8>, CryptoError> {
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

pub fn decrypt_blob(envelope: &[u8], password: &str) -> Result<String, CryptoError> {
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
