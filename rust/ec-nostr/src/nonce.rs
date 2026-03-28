use rand::RngCore;
use rand::rngs::OsRng;

/// Generate a 32-byte random nonce as a lowercase hex string.
pub fn random_nonce_hex() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
