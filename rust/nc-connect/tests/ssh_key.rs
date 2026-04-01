use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use nc_connect::connect::ssh_key::EphemeralKeypair;

#[test]
fn openssh_pubkey_starts_with_prefix() {
    let kp = EphemeralKeypair::generate();
    let s = kp.openssh_pubkey_string();
    assert!(s.starts_with("ssh-ed25519 "), "got: {s}");
}

#[test]
fn openssh_pubkey_base64_decodes_to_51_bytes() {
    let kp = EphemeralKeypair::generate();
    let s = kp.openssh_pubkey_string();
    let b64_part = s.strip_prefix("ssh-ed25519 ").unwrap();
    let wire = BASE64.decode(b64_part).expect("valid base64");
    // 4 + 11 + 4 + 32 = 51
    assert_eq!(wire.len(), 51, "wire length should be 51");
}

#[test]
fn openssh_pubkey_wire_header_correct() {
    let kp = EphemeralKeypair::generate();
    let s = kp.openssh_pubkey_string();
    let b64_part = s.strip_prefix("ssh-ed25519 ").unwrap();
    let wire = BASE64.decode(b64_part).unwrap();

    // u32 big-endian 11
    assert_eq!(&wire[..4], &[0, 0, 0, 11]);
    // "ssh-ed25519"
    assert_eq!(&wire[4..15], b"ssh-ed25519");
    // u32 big-endian 32
    assert_eq!(&wire[15..19], &[0, 0, 0, 32]);
}

#[test]
fn two_generated_keypairs_have_different_pubkeys() {
    let kp1 = EphemeralKeypair::generate();
    let kp2 = EphemeralKeypair::generate();
    // Probability of collision is astronomically low.
    assert_ne!(kp1.openssh_pubkey_string(), kp2.openssh_pubkey_string());
}

#[test]
fn openssh_pubkey_single_space_separated_two_tokens() {
    let kp = EphemeralKeypair::generate();
    let s = kp.openssh_pubkey_string();
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0], "ssh-ed25519");
    // Second token should be non-empty valid base64.
    assert!(!parts[1].is_empty());
    assert!(BASE64.decode(parts[1]).is_ok());
}
