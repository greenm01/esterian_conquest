use nc_nostr::private_payload::{
    decrypt_private_json_from_event, decrypt_private_text_from_event, encrypt_private_json,
    encrypt_private_text,
};
use nostr_sdk::{EventBuilder, Keys, Kind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SamplePayload {
    message: String,
}

#[test]
fn private_text_round_trips_without_compression_for_small_payloads() {
    let sender = Keys::generate();
    let recipient = Keys::generate();
    let plaintext = "small private payload";
    let encrypted =
        encrypt_private_text(&sender, &recipient.public_key(), plaintext).expect("encrypt");

    let event = EventBuilder::new(Kind::Custom(30599), encrypted)
        .sign_with_keys(&sender)
        .expect("sign");

    let decrypted =
        decrypt_private_text_from_event(recipient.secret_key(), &event).expect("decrypt");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn private_json_round_trips_with_compression_for_large_payloads() {
    let sender = Keys::generate();
    let recipient = Keys::generate();
    let payload = SamplePayload {
        message: "x".repeat(4096),
    };
    let encrypted =
        encrypt_private_json(&sender, &recipient.public_key(), &payload).expect("encrypt");

    let event = EventBuilder::new(Kind::Custom(30599), encrypted)
        .sign_with_keys(&sender)
        .expect("sign");

    let decrypted: SamplePayload =
        decrypt_private_json_from_event(recipient.secret_key(), &event).expect("decrypt");
    assert_eq!(decrypted, payload);
}

#[test]
fn private_envelope_keeps_small_payload_uncompressed() {
    let sender = Keys::generate();
    let recipient = Keys::generate();
    let encrypted =
        encrypt_private_text(&sender, &recipient.public_key(), "brief payload").expect("encrypt");
    let event = EventBuilder::new(Kind::Custom(30599), encrypted)
        .sign_with_keys(&sender)
        .expect("sign");

    let wire =
        nostr_sdk::nips::nip44::decrypt(recipient.secret_key(), &event.pubkey, &event.content)
            .expect("decrypt wire envelope");
    assert!(wire.contains(r#""compression":"none""#));
}

#[test]
fn private_envelope_uses_zstd_for_large_compressible_payloads() {
    let sender = Keys::generate();
    let recipient = Keys::generate();
    let encrypted = encrypt_private_text(
        &sender,
        &recipient.public_key(),
        &"compressible".repeat(300),
    )
    .expect("encrypt");
    let event = EventBuilder::new(Kind::Custom(30599), encrypted)
        .sign_with_keys(&sender)
        .expect("sign");

    let wire =
        nostr_sdk::nips::nip44::decrypt(recipient.secret_key(), &event.pubkey, &event.content)
            .expect("decrypt wire envelope");
    assert!(wire.contains(r#""compression":"zstd""#));
}
