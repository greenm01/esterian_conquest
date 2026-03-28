use sha2::{Digest, Sha256};

/// Compute SHA-256 of arbitrary bytes and return lowercase hex.
pub fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}
