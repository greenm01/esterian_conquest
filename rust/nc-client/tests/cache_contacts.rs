use nc_client::cache::{ClientCache, ContactEntry, parse_cache_str, render_cache};

#[test]
fn direct_contact_metadata_round_trips_through_kdl_cache() {
    let mut cache = ClientCache::empty();
    cache.upsert_contact(ContactEntry {
        npub: "npub1sysop".to_string(),
        label: "nc_sysop".to_string(),
        nip05: Some("nc_sysop@nostrian-conquest.com".to_string()),
        source: "host".to_string(),
        blocked: true,
        hidden: true,
        unread_count: 3,
        last_activity_at: Some("2026-04-13T22:15:00Z".to_string()),
    });

    let rendered = render_cache(&cache);
    let parsed = parse_cache_str(&rendered).expect("parse cache");

    assert_eq!(parsed.direct_contacts.len(), 1);
    assert_eq!(
        parsed.direct_contacts[0],
        ContactEntry {
            npub: "npub1sysop".to_string(),
            label: "nc_sysop".to_string(),
            nip05: Some("nc_sysop@nostrian-conquest.com".to_string()),
            source: "host".to_string(),
            blocked: true,
            hidden: true,
            unread_count: 3,
            last_activity_at: Some("2026-04-13T22:15:00Z".to_string()),
        }
    );
}
