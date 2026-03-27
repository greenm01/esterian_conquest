//! Nostr subscription loop for `ec-gate serve`.
//!
//! Connects to the configured relay, subscribes to kind-30501 SessionRequest
//! events addressed to this daemon's npub, and dispatches each event through
//! the routing layer.  On a successful route it provisions an SSH key and
//! publishes 30502 SessionReady.  On a routing error it publishes 30503
//! SessionError.  A background reaper task periodically removes expired keys.

pub mod game_def;
pub mod provision;
pub mod request;
pub mod response;
pub mod routing;

use std::path::Path;
use std::sync::{Arc, Mutex};

use nostr_sdk::{Client, Filter, Keys, Kind, PublicKey, RelayPoolNotification, ToBech32};
use tokio::time::{Duration, interval};
use tracing::{debug, error, info, info_span, warn, Instrument};

use crate::config::GateConfig;
use crate::roster::io::load_roster;
use crate::roster::Roster;

/// Run the `ec-gate serve` event loop.
///
/// Connects to `config.relay`, loads all configured game rosters into memory,
/// subscribes to 30501 events tagged to this daemon's npub, routes each event,
/// provisions SSH keys on success, and publishes 30502/30503 back to the player.
/// A background tokio task reaps expired keys on each `key_ttl` interval.
pub async fn run_serve(
    config: &GateConfig,
    keys: &Keys,
) -> Result<(), Box<dyn std::error::Error>> {
    let npub = keys
        .public_key()
        .to_bech32()
        .map_err(|e| format!("npub bech32: {e}"))?;

    info!(relay = %config.relay, "connecting to relay");
    info!(npub = %npub, "daemon pubkey");

    // Load all rosters up front.
    let game_dirs: Vec<_> = config.games.clone();
    let mut rosters: Vec<Roster> = Vec::new();
    for dir in &game_dirs {
        let roster_path = dir.join("roster.kdl");
        match load_roster(&roster_path) {
            Ok(r) => {
                info!(game_id = %r.id, game_name = %r.name, "loaded roster");
                rosters.push(r);
            }
            Err(e) => {
                warn!(path = %roster_path.display(), error = %e, "cannot load roster");
            }
        }
    }

    let shared_rosters = Arc::new(Mutex::new(rosters));
    let shared_dirs: Arc<Vec<_>> = Arc::new(game_dirs);

    let client = Client::new(keys.clone());
    client
        .add_relay(config.relay.as_str())
        .await
        .map_err(|e| format!("add_relay: {e}"))?;
    client.connect().await;

    // Subscribe: kind 30501 events addressed to our pubkey via `p` tag.
    let filter = Filter::new()
        .kind(Kind::Custom(30501))
        .pubkey(keys.public_key());

    client
        .subscribe(filter, None)
        .await
        .map_err(|e| format!("subscribe: {e}"))?;

    info!("subscribed — waiting for session requests");

    // Publish 30500 GameDefinition for each loaded game on startup.
    {
        let rosters = shared_rosters.lock().unwrap();
        for roster in rosters.iter() {
            match game_def::publish_game_definition(&client, keys, roster).await {
                Ok(event_id) => info!(
                    game_id = %roster.id,
                    event_id = %event_id,
                    "published 30500 GameDefinition"
                ),
                Err(e) => warn!(
                    game_id = %roster.id,
                    error = %e,
                    "failed to publish 30500 GameDefinition"
                ),
            }
        }
    }

    // Background reaper: remove expired ephemeral keys every key_ttl seconds.
    let reap_config = config.clone();
    let reap_interval_secs = reap_config.key_ttl.max(1);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(reap_interval_secs));
        loop {
            ticker.tick().await;
            match provision::reap_expired_keys(&reap_config) {
                Ok(0) => {}
                Ok(n) => info!(count = n, "reaped expired SSH key(s)"),
                Err(e) => error!(error = %e, "reap error"),
            }
        }
    });

    let shared_keys = Arc::new(keys.clone());
    let shared_config = Arc::new(config.clone());

    client
        .handle_notifications(|notification| {
            let shared_rosters = Arc::clone(&shared_rosters);
            let shared_dirs = Arc::clone(&shared_dirs);
            let shared_keys = Arc::clone(&shared_keys);
            let shared_config = Arc::clone(&shared_config);
            let client_clone = client.clone();
            async move {
                if let RelayPoolNotification::Event { event, .. } = notification {
                    match request::parse_session_request(&event) {
                        Ok(req) => {
                            // Build the per-request span with fields known at parse time.
                            let span = info_span!(
                                "request",
                                player_npub = %req.player_pubkey,
                                nonce = %req.nonce,
                            );
                            handle_request(
                                req,
                                shared_rosters,
                                shared_dirs,
                                shared_keys,
                                shared_config,
                                client_clone,
                            )
                            .instrument(span)
                            .await;
                        }
                        Err(e) => {
                            debug!(error = %e, "rejected event");
                        }
                    }
                }
                // Return false to keep the loop running.
                Ok(false)
            }
        })
        .await
        .map_err(|e| format!("notification loop: {e}"))?;

    Ok(())
}

/// Process one validated session request inside its tracing span.
async fn handle_request(
    req: request::SessionRequest,
    shared_rosters: Arc<Mutex<Vec<Roster>>>,
    shared_dirs: Arc<Vec<std::path::PathBuf>>,
    shared_keys: Arc<Keys>,
    shared_config: Arc<GateConfig>,
    client: Client,
) {
    // Parse the player's public key for encryption.
    let player_pubkey = match PublicKey::from_hex(&req.player_pubkey) {
        Ok(pk) => pk,
        Err(e) => {
            warn!(error = %e, "cannot parse player pubkey");
            return;
        }
    };

    let dirs_ref: Vec<&Path> = shared_dirs.iter().map(|p| p.as_path()).collect();
    let (decision, game_dir) = {
        let mut rosters = shared_rosters.lock().unwrap();
        let d = routing::route(&req, &mut rosters, &dirs_ref);
        // Capture the game dir that matched, for provisioning.
        let gd = if let routing::RoutingDecision::Provisioned(ref seat) = d {
            rosters
                .iter()
                .position(|r| r.id == seat.game_id)
                .and_then(|i| shared_dirs.get(i))
                .cloned()
        } else {
            None
        };
        (d, gd)
    };

    match decision {
        routing::RoutingDecision::Provisioned(seat) => {
            let game_dir = match game_dir {
                Some(d) => d,
                None => {
                    error!(game_id = %seat.game_id, "no game dir for provisioned seat");
                    return;
                }
            };

            match provision::provision_key(
                &shared_config,
                &seat,
                &req.ssh_pubkey,
                &game_dir,
            ) {
                Ok(provisioned) => {
                    info!(
                        game_id = %seat.game_id,
                        player = seat.player,
                        "provisioned seat"
                    );
                    if let Err(e) = response::publish_session_ready(
                        &client,
                        &shared_keys,
                        &player_pubkey,
                        &req.nonce,
                        &shared_config,
                        &seat,
                        &provisioned,
                    )
                    .await
                    {
                        error!(error = %e, "failed to publish 30502 SessionReady");
                    }
                }
                Err(e) => {
                    error!(error = %e, "provision failed");
                }
            }
        }
        routing::RoutingDecision::Error(route_err) => {
            warn!(
                error_code = route_err.error_code(),
                reason = %route_err,
                "routing error"
            );
            if let Err(e) = response::publish_session_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                &route_err,
            )
            .await
            {
                error!(error = %e, "failed to publish 30503 SessionError");
            }
        }
    }
}
