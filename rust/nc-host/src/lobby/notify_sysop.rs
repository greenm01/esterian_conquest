use nc_data::hosted::{
    HostedStore, enqueue_sysop_notification, get_game_metadata, get_pending_sysop_notifications,
    mark_sysop_notification_failed, mark_sysop_notification_sent,
};
use nc_nostr::pubkeys::hex_to_npub;

pub fn enqueue_invite_request_summary(
    store: &HostedStore,
    game_id: &str,
    request_id: &str,
    player_pubkey: &str,
    player_handle: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = get_game_metadata(store.connection(), game_id)?;
    let summary = format!(
        "New invite request for {} in {}.",
        player_label(player_pubkey, player_handle),
        metadata.name
    );
    enqueue_sysop_notification(
        store.connection(),
        &format!("invite-request:{request_id}"),
        game_id,
        player_pubkey,
        "invite_request",
        &summary,
    )?;
    Ok(())
}

pub fn enqueue_thread_message_summary(
    store: &HostedStore,
    game_id: &str,
    message_id: &str,
    player_pubkey: &str,
    player_handle: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = get_game_metadata(store.connection(), game_id)?;
    let summary = format!(
        "New player thread message from {} in {}.",
        player_label(player_pubkey, player_handle),
        metadata.name
    );
    enqueue_sysop_notification(
        store.connection(),
        &format!("thread-message:{message_id}"),
        game_id,
        player_pubkey,
        "thread_message",
        &summary,
    )?;
    Ok(())
}

pub async fn publish_pending_notifications(
    publisher: &crate::lobby::publish::EventPublisher,
    games_root: &std::sync::Arc<std::path::PathBuf>,
    sysop_contact_npub: &str,
) {
    if sysop_contact_npub.trim().is_empty() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(games_root.as_path()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let db_path = path.join("hosted.db");
            if !db_path.exists() {
                continue;
            }

            let Some(game_id) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            let store = match HostedStore::open(&db_path) {
                Ok(store) => store,
                Err(err) => {
                    tracing::warn!(
                        "Failed to open {} for sysop notifications: {}",
                        game_id,
                        err
                    );
                    continue;
                }
            };

            let pending = match get_pending_sysop_notifications(store.connection(), game_id, 20) {
                Ok(items) => items,
                Err(err) => {
                    tracing::warn!(
                        "Failed to read pending sysop notifications for {}: {}",
                        game_id,
                        err
                    );
                    continue;
                }
            };

            for notification in pending {
                match publisher
                    .publish_private_dm(sysop_contact_npub, &notification.summary)
                    .await
                {
                    Ok(()) => {
                        let _ = mark_sysop_notification_sent(store.connection(), &notification.id);
                    }
                    Err(err) => {
                        let _ = mark_sysop_notification_failed(
                            store.connection(),
                            &notification.id,
                            &err.to_string(),
                        );
                        tracing::warn!(
                            "Failed to publish sysop notification {} for {}: {}",
                            notification.id,
                            game_id,
                            err
                        );
                    }
                }
            }
        }
    }
}

fn player_label(player_pubkey: &str, player_handle: Option<&str>) -> String {
    if let Some(handle) = player_handle
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return handle.to_string();
    }

    hex_to_npub(player_pubkey)
        .map(|npub| shorten(&npub))
        .unwrap_or_else(|| shorten(player_pubkey))
}

fn shorten(value: &str) -> String {
    const EDGE: usize = 8;
    if value.len() <= EDGE * 2 + 1 {
        return value.to_string();
    }
    format!("{}...{}", &value[..EDGE], &value[value.len() - EDGE..])
}
