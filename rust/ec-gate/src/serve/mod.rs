//! Nostr subscription loop for `ec-gate serve`.
//!
//! Connects to the configured relay, subscribes to kind-30501 SessionRequest
//! events addressed to this daemon's npub, and dispatches each event through
//! the routing layer.  On a successful route it provisions an SSH key and
//! publishes 30502 SessionReady.  On a routing error it publishes 30503
//! SessionError.  It also serves 30504 starmap requests with 30505 MapBundle
//! or 30506 MapError.  A background reaper task periodically removes expired
//! keys.

pub mod catalog;
pub mod game_def;
pub mod map;
pub mod provision;
pub mod request;
pub mod response;
pub mod routing;

use std::path::Path;
use std::sync::Arc;

use ec_data::{CoreGameData, build_player_map_export_data};
use nostr_sdk::{Client, Filter, Keys, Kind, PublicKey, RelayPoolNotification, ToBech32};
use tokio::time::{Duration, interval};
use tracing::{Instrument, debug, error, info, info_span, warn};

use crate::config::GateConfig;
/// Run the `ec-gate serve` event loop.
///
/// Connects to `config.relay`, loads all configured game rosters into memory,
/// subscribes to 30501 events tagged to this daemon's npub, routes each event,
/// provisions SSH keys on success, and publishes 30502/30503 back to the player.
/// It also serves 30504 map requests with 30505/30506 responses. A background
/// tokio task reaps expired keys on each `key_ttl` interval.
pub async fn run_serve(config: &GateConfig, keys: &Keys) -> Result<(), Box<dyn std::error::Error>> {
    let npub = keys
        .public_key()
        .to_bech32()
        .map_err(|e| format!("npub bech32: {e}"))?;

    info!(relay = %config.relay, "connecting to relay");
    info!(npub = %npub, "daemon pubkey");

    let game_dirs: Vec<_> = config.games.clone();
    let shared_dirs: Arc<Vec<_>> = Arc::new(game_dirs);
    let startup_games = match catalog::load_hosted_games(shared_dirs.as_slice()) {
        Ok(games) => games,
        Err(err) => return Err(err.into()),
    };
    for entry in &startup_games {
        info!(
            game_id = %entry.game.game_id,
            game_name = %entry.game.game_name,
            "loaded hosted game"
        );
    }

    let client = Client::new(keys.clone());
    client
        .add_relay(config.relay.as_str())
        .await
        .map_err(|e| format!("add_relay: {e}"))?;
    client.connect().await;

    // Subscribe: kind 30501 and 30504 events addressed to our pubkey via `p` tag.
    let filter = Filter::new()
        .kinds([Kind::Custom(30501), Kind::Custom(30504)])
        .pubkey(keys.public_key());

    client
        .subscribe(filter, None)
        .await
        .map_err(|e| format!("subscribe: {e}"))?;

    info!("subscribed — waiting for session requests");

    // Publish 30500 GameDefinition for each loaded game on startup.
    {
        for entry in &startup_games {
            match game_def::publish_game_definition(&client, keys, &entry.game).await {
                Ok(event_id) => info!(
                    game_id = %entry.game.game_id,
                    event_id = %event_id,
                    "published 30500 GameDefinition"
                ),
                Err(e) => warn!(
                    game_id = %entry.game.game_id,
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
            let shared_dirs = Arc::clone(&shared_dirs);
            let shared_keys = Arc::clone(&shared_keys);
            let shared_config = Arc::clone(&shared_config);
            let client_clone = client.clone();
            async move {
                if let RelayPoolNotification::Event { event, .. } = notification {
                    match event.kind.as_u16() {
                        30501 => match request::parse_session_request(&event) {
                            Ok(req) => {
                                let span = info_span!(
                                    "request",
                                    player_npub = %req.player_pubkey,
                                    nonce = %req.nonce,
                                );
                                handle_request(
                                    req,
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
                        },
                        30504 => match map::parse_map_request(&event) {
                            Ok(req) => {
                                let span = info_span!(
                                    "map_request",
                                    player_npub = %req.player_pubkey,
                                    nonce = %req.nonce,
                                    game_id = %req.game_id,
                                );
                                handle_map_request(req, shared_dirs, shared_keys, client_clone)
                                    .instrument(span)
                                    .await;
                            }
                            Err(e) => {
                                debug!(error = %e, "rejected map event");
                            }
                        },
                        _ => {}
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

async fn handle_map_request(
    req: map::MapRequest,
    shared_dirs: Arc<Vec<std::path::PathBuf>>,
    shared_keys: Arc<Keys>,
    client: Client,
) {
    let player_pubkey = match PublicKey::from_hex(&req.player_pubkey) {
        Ok(pk) => pk,
        Err(e) => {
            warn!(error = %e, "cannot parse player pubkey for map request");
            return;
        }
    };

    let loaded_games = match catalog::load_hosted_games(shared_dirs.as_slice()) {
        Ok(games) => games,
        Err(err) => {
            warn!(error = %err, "cannot load hosted games for map request");
            if let Err(pub_err) = map::publish_map_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "map_unavailable",
                "The starmap bundle is not available right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30506 MapError");
            }
            return;
        }
    };
    let (seat, game_dir) =
        match routing::resolve_player_in_game(&req.player_pubkey, &req.game_id, &loaded_games) {
            Ok(seat) => {
                let game_dir = loaded_games
                    .iter()
                    .find(|entry| entry.game.game_id == seat.game_id)
                    .map(|entry| entry.dir.clone());
                (Ok(seat), game_dir)
            }
            Err(err) => (Err(err), None),
        };

    let seat = match seat {
        Ok(seat) => seat,
        Err(routing::RouteError::GameNotFound) => {
            if let Err(err) = map::publish_map_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "game_not_found",
                "The requested game was not found on this server.",
            )
            .await
            {
                error!(error = %err, "failed to publish 30506 MapError");
            }
            return;
        }
        Err(_) => {
            if let Err(err) = map::publish_map_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "unknown_player",
                "Your identity is not enrolled in that game.",
            )
            .await
            {
                error!(error = %err, "failed to publish 30506 MapError");
            }
            return;
        }
    };

    let Some(game_dir) = game_dir else {
        if let Err(err) = map::publish_map_error(
            &client,
            &shared_keys,
            &player_pubkey,
            &req.nonce,
            "map_unavailable",
            "The starmap bundle is not available right now.",
        )
        .await
        {
            error!(error = %err, "failed to publish 30506 MapError");
        }
        return;
    };

    let export = match build_player_map_export_data(&game_dir, seat.player) {
        Ok(export) => export,
        Err(err) => {
            warn!(game_id = %seat.game_id, error = %err, "unable to build starmap bundle");
            if let Err(pub_err) = map::publish_map_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "map_unavailable",
                "The starmap bundle is not available right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30506 MapError");
            }
            return;
        }
    };

    match map::publish_map_bundle(
        &client,
        &shared_keys,
        &player_pubkey,
        &req.nonce,
        &seat,
        &export,
    )
    .await
    {
        Ok(event_id) => {
            info!(game_id = %seat.game_id, event_id = %event_id, "published 30505 MapBundle");
        }
        Err(map::PublishMapBundleError::PayloadTooLarge) => {
            if let Err(err) = map::publish_map_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "payload_too_large",
                "The starmap bundle is too large to deliver in one message.",
            )
            .await
            {
                error!(error = %err, "failed to publish 30506 MapError");
            }
        }
        Err(err) => {
            warn!(game_id = %seat.game_id, error = %err, "failed to publish map bundle");
            if let Err(pub_err) = map::publish_map_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "map_unavailable",
                "The starmap bundle is not available right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30506 MapError");
            }
        }
    }
}

/// Process one validated session request inside its tracing span.
async fn handle_request(
    req: request::SessionRequest,
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

    let loaded_games = match catalog::load_hosted_games(shared_dirs.as_slice()) {
        Ok(games) => games,
        Err(err) => {
            error!(error = %err, "cannot load hosted games");
            return;
        }
    };
    let decision = routing::route(&req, &loaded_games);

    match decision {
        routing::RoutingDecision::Provisioned(seat) => {
            let game_dir = match loaded_games
                .iter()
                .find(|entry| entry.game.game_id == seat.game_id)
                .map(|entry| entry.dir.clone())
            {
                Some(d) => d,
                None => {
                    error!(game_id = %seat.game_id, "no game dir for provisioned seat");
                    return;
                }
            };
            if seat.first_claim {
                let Some(invite_code) = req.invite_code.as_deref() else {
                    error!(game_id = %seat.game_id, "first claim missing invite code");
                    return;
                };
                let store = match ec_data::CampaignStore::open_default_in_dir(&game_dir) {
                    Ok(store) => store,
                    Err(err) => {
                        error!(game_id = %seat.game_id, error = %err, "cannot open campaign store for claim");
                        return;
                    }
                };
                match store.claim_hosted_seat(invite_code, &req.player_pubkey) {
                    Ok(_) => match catalog::load_hosted_game(&game_dir) {
                        Ok(entry) => {
                            match game_def::publish_game_definition(
                                &client,
                                &shared_keys,
                                &entry.game,
                            )
                            .await
                            {
                                Ok(event_id) => info!(
                                    game_id = %entry.game.game_id,
                                    event_id = %event_id,
                                    "published updated 30500 GameDefinition"
                                ),
                                Err(err) => warn!(
                                    game_id = %seat.game_id,
                                    error = %err,
                                    "failed to publish updated 30500 GameDefinition"
                                ),
                            }
                        }
                        Err(err) => {
                            warn!(game_id = %seat.game_id, error = %err, "cannot reload hosted game after claim")
                        }
                    },
                    Err(ec_data::ClaimHostedSeatError::InvalidCode) => {
                        if let Err(err) = response::publish_session_error(
                            &client,
                            &shared_keys,
                            &player_pubkey,
                            &req.nonce,
                            &routing::RouteError::InvalidCode,
                        )
                        .await
                        {
                            error!(error = %err, "failed to publish 30503 SessionError");
                        }
                        return;
                    }
                    Err(ec_data::ClaimHostedSeatError::CodeClaimed) => {
                        if let Err(err) = response::publish_session_error(
                            &client,
                            &shared_keys,
                            &player_pubkey,
                            &req.nonce,
                            &routing::RouteError::CodeClaimed,
                        )
                        .await
                        {
                            error!(error = %err, "failed to publish 30503 SessionError");
                        }
                        return;
                    }
                    Err(ec_data::ClaimHostedSeatError::Store(err)) => {
                        error!(game_id = %seat.game_id, error = %err, "claim failed");
                        return;
                    }
                }
            }

            match provision::provision_key(&shared_config, &seat, &req.ssh_pubkey, &game_dir) {
                Ok(provisioned) => {
                    let player_name = player_name_for_seat(&game_dir, seat.player);
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
                        &player_name,
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

fn player_name_for_seat(game_dir: &Path, player_index_1_based: usize) -> String {
    let Ok(game_data) = CoreGameData::load(game_dir) else {
        return String::new();
    };
    game_data
        .player
        .records
        .get(player_index_1_based.saturating_sub(1))
        .map(|record| record.controlled_empire_name_summary())
        .unwrap_or_default()
}
