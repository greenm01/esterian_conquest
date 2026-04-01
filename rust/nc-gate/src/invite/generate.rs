//! Invite code generation and validation.

use std::collections::HashSet;

use rand::Rng;

use super::wordlist::WORDLIST;

/// Generate a unique invite code not already in `existing_codes`.
///
/// The code is two words from the Monero mnemonic wordlist, joined with a
/// hyphen (e.g. `velvet-mountain`). Retries on collision. With ~2.6 million
/// possible codes and typical game counts in the single digits, collisions
/// are extremely unlikely in practice.
pub fn generate_invite_code(existing_codes: &HashSet<String>) -> String {
    let mut rng = rand::thread_rng();
    loop {
        let code = random_code(&mut rng);
        if !existing_codes.contains(&code) {
            return code;
        }
    }
}

/// Generate a unique invite code using a caller-supplied RNG.
///
/// Useful in tests where deterministic output is required.
pub fn generate_invite_code_with_rng<R: Rng>(
    rng: &mut R,
    existing_codes: &HashSet<String>,
) -> String {
    loop {
        let code = random_code(rng);
        if !existing_codes.contains(&code) {
            return code;
        }
    }
}

/// Check whether `code` (after stripping relay suffix) is a valid two-word
/// Monero mnemonic pair. Both words must be in the wordlist.
pub fn is_valid_invite_code(code: &str) -> bool {
    let normalized = normalize_code(code);
    let parts: Vec<&str> = normalized.splitn(2, '-').collect();
    if parts.len() != 2 {
        return false;
    }
    let word_set: HashSet<&str> = WORDLIST.iter().copied().collect();
    word_set.contains(parts[0]) && word_set.contains(parts[1])
}

/// Strip relay suffix and lowercase. Does not validate wordlist membership.
pub fn normalize_code(code: &str) -> String {
    let stripped = code.trim();
    let without_relay = stripped.split('@').next().unwrap_or(stripped);
    without_relay.to_lowercase()
}

fn random_code<R: Rng>(rng: &mut R) -> String {
    let i = rng.gen_range(0..WORDLIST.len());
    let j = rng.gen_range(0..WORDLIST.len());
    format!("{}-{}", WORDLIST[i], WORDLIST[j])
}
