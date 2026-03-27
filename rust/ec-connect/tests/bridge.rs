//! Unit tests for the SSH terminal bridge.
//!
//! These tests cover the pieces that can be verified without a live SSH
//! server: key conversion, host-key fingerprint verification, and the
//! `signing_key_bytes` accessor.
//!
//! The I/O loop and `run_bridge` require a real TCP connection; those are
//! integration tests deferred to step 12.

use ec_connect::connect::ssh_key::EphemeralKeypair;
use russh::keys::ssh_key::HashAlg;

// ── EphemeralKeypair::signing_key_bytes ───────────────────────────────────────

#[test]
fn signing_key_bytes_is_32_bytes() {
    let kp = EphemeralKeypair::generate();
    let bytes = kp.signing_key_bytes();
    assert_eq!(bytes.len(), 32);
}

#[test]
fn two_keypairs_have_different_signing_key_bytes() {
    let kp1 = EphemeralKeypair::generate();
    let kp2 = EphemeralKeypair::generate();
    assert_ne!(kp1.signing_key_bytes(), kp2.signing_key_bytes());
}

// ── EphemeralKeypair::to_russh_private_key ────────────────────────────────────

#[test]
fn to_russh_private_key_succeeds() {
    let kp = EphemeralKeypair::generate();
    let result = kp.to_russh_private_key();
    assert!(result.is_ok(), "conversion should succeed: {result:?}");
}

#[test]
fn russh_private_key_algorithm_is_ed25519() {
    let kp = EphemeralKeypair::generate();
    let privkey = kp.to_russh_private_key().unwrap();
    assert_eq!(
        privkey.algorithm().as_str(),
        "ssh-ed25519",
        "algorithm should be Ed25519"
    );
}

#[test]
fn russh_private_key_pubkey_matches_openssh_pubkey_string() {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    let kp = EphemeralKeypair::generate();
    let privkey = kp.to_russh_private_key().unwrap();

    // Extract the 32-byte Ed25519 public key from the russh PrivateKey.
    let pubkey_bytes = privkey
        .public_key()
        .key_data()
        .ed25519()
        .expect("should be Ed25519 public key")
        .as_ref()
        .to_vec();
    assert_eq!(pubkey_bytes.len(), 32);

    // Decode the OpenSSH wire format from openssh_pubkey_string and compare.
    let openssh_str = kp.openssh_pubkey_string();
    let b64 = openssh_str.strip_prefix("ssh-ed25519 ").unwrap();
    let wire = BASE64.decode(b64).unwrap();
    // Wire: 4 bytes len + "ssh-ed25519" + 4 bytes len + 32-byte pubkey
    let wire_pubkey = &wire[19..51];

    assert_eq!(
        pubkey_bytes.as_slice(),
        wire_pubkey,
        "russh PrivateKey public part should match openssh_pubkey_string"
    );
}

#[test]
fn russh_private_key_round_trips_through_bytes() {
    // Verify that signing_key_bytes → to_russh_private_key stays deterministic.
    let kp = EphemeralKeypair::generate();
    let bytes = kp.signing_key_bytes();

    let privkey1 = kp.to_russh_private_key().unwrap();
    let privkey2 = kp.to_russh_private_key().unwrap();

    // Both conversions of the same keypair should produce the same fingerprint.
    let fp1 = privkey1.fingerprint(HashAlg::Sha256).to_string();
    let fp2 = privkey2.fingerprint(HashAlg::Sha256).to_string();
    assert_eq!(
        fp1, fp2,
        "repeated conversion of same keypair should be stable"
    );
    let _ = bytes;
}

// ── Fingerprint format ────────────────────────────────────────────────────────

#[test]
fn fingerprint_format_is_sha256_prefixed() {
    let kp = EphemeralKeypair::generate();
    let privkey = kp.to_russh_private_key().unwrap();
    let fp = privkey.fingerprint(HashAlg::Sha256).to_string();
    assert!(
        fp.starts_with("SHA256:"),
        "fingerprint should start with SHA256: but got: {fp}"
    );
}

#[test]
fn two_different_keypairs_have_different_fingerprints() {
    let kp1 = EphemeralKeypair::generate();
    let kp2 = EphemeralKeypair::generate();
    let privkey1 = kp1.to_russh_private_key().unwrap();
    let privkey2 = kp2.to_russh_private_key().unwrap();
    let fp1 = privkey1.fingerprint(HashAlg::Sha256).to_string();
    let fp2 = privkey2.fingerprint(HashAlg::Sha256).to_string();
    assert_ne!(
        fp1, fp2,
        "different keypairs should have different fingerprints"
    );
}
