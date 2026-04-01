use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::Duration;

use nc_nostr::tags::tag_content;
use nostr_sdk::nips::nip44;
use nostr_sdk::{Client, Keys, Kind, PublicKey, RelayPoolNotification, Timestamp};

use crate::connect::live_response::build_response_filter;
use crate::connect::map_fetch::MapBundlePayload;
use crate::connect::resolve::ResolvedTarget;
use crate::map_store::save_map_bundle;

pub const MAP_PUSH_KIND: u16 = 30512;
const MAP_PUSH_MONITOR_POLL_SECS: u64 = 1;
const MAP_PUSH_HISTORY_LOOKBACK_SECS: u64 = 24 * 60 * 60;
const MAP_PUSH_FETCH_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone)]
pub struct FirstJoinMapPushConfig {
    pub player_keys: Keys,
    pub target: ResolvedTarget,
    pub gate_npub: String,
    pub game_id: String,
    pub maps_root: PathBuf,
}

#[derive(Debug, Default)]
pub struct MapPushMonitorResult {
    pub maps_saved_to: Option<PathBuf>,
    pub warning: Option<String>,
}

pub struct MapPushMonitor {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<MapPushMonitorResult>>,
}

impl MapPushMonitor {
    pub fn start(config: FirstJoinMapPushConfig) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(err) => {
                    return MapPushMonitorResult {
                        maps_saved_to: None,
                        warning: Some(format!(
                            "Warning: unable to start proactive starmap listener: {err}"
                        )),
                    };
                }
            };
            rt.block_on(run_map_push_monitor(config, thread_stop))
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }

    pub fn finish(mut self) -> MapPushMonitorResult {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .take()
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default()
    }
}

impl Drop for MapPushMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

async fn run_map_push_monitor(
    config: FirstJoinMapPushConfig,
    stop: Arc<AtomicBool>,
) -> MapPushMonitorResult {
    let gate_pubkey = match PublicKey::parse(&config.gate_npub) {
        Ok(pubkey) => pubkey,
        Err(err) => {
            return MapPushMonitorResult {
                maps_saved_to: None,
                warning: Some(format!(
                    "Warning: proactive starmap listener gate key: {err}"
                )),
            };
        }
    };

    let client = Client::new(config.player_keys.clone());
    if let Err(err) = client.add_relay(&config.target.relay_url).await {
        return MapPushMonitorResult {
            maps_saved_to: None,
            warning: Some(format!(
                "Warning: unable to add proactive starmap relay: {err}"
            )),
        };
    }
    client.connect().await;

    let player_pubkey = config.player_keys.public_key();
    let filter = build_response_filter(
        &gate_pubkey,
        &player_pubkey,
        [Kind::Custom(MAP_PUSH_KIND)],
        Timestamp::now() - Duration::from_secs(60),
    );
    let mut notifications = client.notifications();
    let expected_subscription_id = match client.subscribe(filter, None).await {
        Ok(result) => result.val,
        Err(err) => {
            client.disconnect().await;
            return MapPushMonitorResult {
                maps_saved_to: None,
                warning: Some(format!(
                    "Warning: unable to subscribe for proactive starmaps: {err}"
                )),
            };
        }
    };

    let mut result = MapPushMonitorResult::default();
    while !stop.load(Ordering::Relaxed) {
        match tokio::time::timeout(
            Duration::from_secs(MAP_PUSH_MONITOR_POLL_SECS),
            notifications.recv(),
        )
        .await
        {
            Ok(Ok(RelayPoolNotification::Event {
                subscription_id,
                event,
                ..
            })) if subscription_id == expected_subscription_id => {
                if let Some(bundle) =
                    parse_map_push_event(&event, &config.player_keys, &gate_pubkey, &config.game_id)
                {
                    match bundle.and_then(|bundle| {
                        save_map_bundle(&bundle, &config.target.relay_url, &config.maps_root)
                            .map_err(|err| err.to_string())
                    }) {
                        Ok(path) => {
                            result.maps_saved_to = Some(path);
                            result.warning = None;
                            break;
                        }
                        Err(err) => {
                            result.warning =
                                Some(format!("Warning: unable to save proactive starmaps: {err}"));
                            break;
                        }
                    }
                }
            }
            Ok(Ok(_)) => {}
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {}
            Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => break,
            Err(_) => {}
        }
    }

    let _ = client.unsubscribe(&expected_subscription_id).await;
    client.disconnect().await;
    result
}

pub async fn fetch_recent_map_push(
    player_keys: &Keys,
    target: &ResolvedTarget,
    gate_npub: &str,
    game_id: &str,
) -> Result<MapBundlePayload, String> {
    let gate_pubkey = PublicKey::parse(gate_npub).map_err(|err| format!("gate key: {err}"))?;
    let client = Client::new(player_keys.clone());
    client
        .add_relay(&target.relay_url)
        .await
        .map_err(|err| format!("add relay: {err}"))?;
    client.connect().await;

    let filter = build_response_filter(
        &gate_pubkey,
        &player_keys.public_key(),
        [Kind::Custom(MAP_PUSH_KIND)],
        Timestamp::now() - Duration::from_secs(MAP_PUSH_HISTORY_LOOKBACK_SECS),
    );
    let events = client
        .fetch_events(filter, Duration::from_secs(MAP_PUSH_FETCH_TIMEOUT_SECS))
        .await
        .map_err(|err| format!("fetch proactive starmaps: {err}"))?;
    client.disconnect().await;

    for event in events.iter() {
        if let Some(bundle) = parse_map_push_event(event, player_keys, &gate_pubkey, game_id) {
            return bundle.map_err(|err| format!("parse proactive starmaps: {err}"));
        }
    }

    Err("no recent proactive starmap bundle found".to_string())
}

fn parse_map_push_event(
    event: &nostr_sdk::Event,
    player_keys: &Keys,
    gate_pubkey: &PublicKey,
    game_id: &str,
) -> Option<Result<MapBundlePayload, String>> {
    if event.kind != Kind::Custom(MAP_PUSH_KIND) {
        return None;
    }
    if event.pubkey != *gate_pubkey {
        return None;
    }
    if tag_content(&event.tags, "p") != Some(&player_keys.public_key().to_hex()) {
        return None;
    }
    if tag_content(&event.tags, "game-id") != Some(game_id) {
        return None;
    }
    let plaintext = match nip44::decrypt(player_keys.secret_key(), &event.pubkey, &event.content) {
        Ok(plaintext) => plaintext,
        Err(err) => return Some(Err(format!("decrypt map push: {err}"))),
    };
    let payload = serde_json::from_str::<MapBundlePayload>(&plaintext)
        .map_err(|err| format!("decode map push: {err}"));
    Some(payload)
}
