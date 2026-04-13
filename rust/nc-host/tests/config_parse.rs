use nc_host::config::host_config::HostConfig;
use std::path::PathBuf;

#[test]
fn parse_host_config_accepts_block_style_kdl() {
    let raw = r#"
host {
    games-root "/tmp/nc-host/games"
    relay-url "ws://127.0.0.1:8080"
    invite-relay-host "localhost:8080"
    identity-path "/tmp/nc-host/host.nsec"
    sysop-contact-npub "npub1example"
}
"#;

    let config = HostConfig::parse(raw).expect("block-style config should parse");

    assert_eq!(config.games_root, PathBuf::from("/tmp/nc-host/games"));
    assert_eq!(config.relay_url, "ws://127.0.0.1:8080");
    assert_eq!(config.invite_relay_host, "localhost:8080");
    assert_eq!(
        config.identity_path,
        PathBuf::from("/tmp/nc-host/host.nsec")
    );
    assert_eq!(config.sysop_contact_npub, "npub1example");
    assert_eq!(config.sysop_contact_label, None);
    assert_eq!(config.sysop_contact_nip05, None);
}

#[test]
fn parse_host_config_accepts_property_style_kdl() {
    let raw = r#"host games-root="/tmp/nc-host/games" relay-url="ws://127.0.0.1:8080" invite-relay-host="localhost:8080" identity-path="/tmp/nc-host/host.nsec" sysop-contact-npub="npub1example""#;

    let config = HostConfig::parse(raw).expect("property-style config should parse");

    assert_eq!(config.games_root, PathBuf::from("/tmp/nc-host/games"));
    assert_eq!(config.relay_url, "ws://127.0.0.1:8080");
    assert_eq!(config.invite_relay_host, "localhost:8080");
    assert_eq!(
        config.identity_path,
        PathBuf::from("/tmp/nc-host/host.nsec")
    );
    assert_eq!(config.sysop_contact_npub, "npub1example");
    assert_eq!(config.sysop_contact_label, None);
    assert_eq!(config.sysop_contact_nip05, None);
}
