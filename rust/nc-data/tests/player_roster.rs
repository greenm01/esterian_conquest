use nc_data::hosted::{
    HandleOwnership, RosterStore, resolve_handle_ownership, upsert_player_seen,
};

fn unique_roster_path(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("nc-data-{name}-{nanos}.db"))
}

#[test]
fn handle_ownership_is_case_insensitive_and_trim_aware() {
    let path = unique_roster_path("roster-handle");
    let store = RosterStore::open(&path).expect("open roster");
    upsert_player_seen(store.connection(), "npub1owner", Some(" StarRider "), "friday-night")
        .expect("upsert owner");

    assert_eq!(
        resolve_handle_ownership(store.connection(), "npub1owner", "starrider")
            .expect("resolve self"),
        HandleOwnership::OwnedBySelf
    );
    assert_eq!(
        resolve_handle_ownership(store.connection(), "npub1other", "  STARRIDER  ")
            .expect("resolve taken"),
        HandleOwnership::Taken
    );
    assert_eq!(
        resolve_handle_ownership(store.connection(), "npub1other", "new-handle")
            .expect("resolve available"),
        HandleOwnership::Available
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn changing_handle_frees_the_previous_name() {
    let path = unique_roster_path("roster-rename");
    let store = RosterStore::open(&path).expect("open roster");
    upsert_player_seen(store.connection(), "npub1owner", Some("StarRider"), "friday-night")
        .expect("upsert owner");
    upsert_player_seen(store.connection(), "npub1owner", Some("Nova"), "friday-night")
        .expect("rename owner");

    assert_eq!(
        resolve_handle_ownership(store.connection(), "npub1other", "StarRider")
            .expect("resolve freed"),
        HandleOwnership::Available
    );
    assert_eq!(
        resolve_handle_ownership(store.connection(), "npub1other", "Nova").expect("resolve nova"),
        HandleOwnership::Taken
    );

    let _ = std::fs::remove_file(path);
}
