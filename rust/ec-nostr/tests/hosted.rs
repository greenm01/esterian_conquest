use ec_nostr::hosted::{invite_address_from_relay, relay_url_to_invite_host};

#[test]
fn relay_url_to_invite_host_uses_domain_without_port_when_unspecified() {
    assert_eq!(
        relay_url_to_invite_host("wss://relay.example.com").unwrap(),
        "relay.example.com"
    );
}

#[test]
fn relay_url_to_invite_host_preserves_explicit_port() {
    assert_eq!(
        relay_url_to_invite_host("wss://relay.example.com:7447").unwrap(),
        "relay.example.com:7447"
    );
}

#[test]
fn relay_url_to_invite_host_rejects_paths() {
    assert!(relay_url_to_invite_host("wss://relay.example.com/socket").is_err());
}

#[test]
fn invite_address_from_relay_formats_token_with_host() {
    assert_eq!(
        invite_address_from_relay("amber-river", "wss://relay.example.com").unwrap(),
        "amber-river@relay.example.com"
    );
}
