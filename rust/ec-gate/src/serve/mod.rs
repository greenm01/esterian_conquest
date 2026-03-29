//! Nostr subscription loop for `ec-gate serve`.
//!
//! Connects to the configured relay, subscribes to kind-30501 SessionRequest
//! events addressed to this daemon's npub, and dispatches each event through
//! the routing layer.  On a successful route it provisions an SSH key and
//! publishes 30502 SessionReady.  On a routing error it publishes 30503
//! SessionError.  It also serves 30504 starmap requests with 30505 MapBundle,
//! 30506 MapError, and 30507 state refresh requests with 30508/30509
//! responses. A background reaper task periodically removes expired keys.

pub mod catalog;
pub mod claim;
pub mod game_def;
pub mod map;
pub mod provision;
pub mod request;
pub mod response;
pub mod routing;
pub mod state;

use std::path::Path;
use std::sync::Arc;

use ec_data::{CampaignStore, build_player_map_export_data};
use nostr_sdk::{
    Client, Filter, Keys, Kind, PublicKey, RelayPoolNotification, Timestamp, ToBech32,
};
use tokio::time::{Duration, interval};
use tracing::{Instrument, debug, error, info, info_span, warn};

use crate::config::GateConfig;
/// Run the `ec-gate serve` event loop.
///
/// Connects to `config.relay`, loads all configured game rosters into memory,
/// subscribes to 30501 events tagged to this daemon's npub, routes each event,
/// provisions SSH keys on success, and publishes 30502/30503 back to the player.
/// It also serves 30504 map requests with 30505/30506 responses and 30507
/// state refresh requests with 30508/30509 responses. A background tokio task
/// reaps expired keys on each `key_ttl` interval.
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

    // Subscribe broadly to the request kinds, then enforce the `p` tag
    // locally. Some relays are inconsistent about live `#p` delivery.
    // Use a 1-minute buffer to avoid missing events due to clock drift.
    let filter = request_subscription_filter(Timestamp::now() - Duration::from_secs(60));

    client
        .subscribe(filter, None)
        .await
        .map_err(|e| format!("subscribe: {e}"))?;

    info!("subscribed — waiting for session requests");

    // Publish 30500 GameDefinition for each loaded game on startup.
    {
        for entry in &startup_games {
            match game_def::publish_game_definition(
                &client,
                keys,
                &entry.game,
                &config.ssh_host,
                config.ssh_port,
                &config.relay,
            )
            .await
            {
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
                    if !is_targeted_to_gate(&event, &shared_keys.public_key()) {
                        return Ok(false);
                    }
                    match event.kind.as_u16() {
                        30510 => match claim::parse_seat_claim_request(&event) {
                            Ok(req) => {
                                let span = info_span!(
                                    "claim_request",
                                    player_npub = %req.player_pubkey,
                                    nonce = %req.nonce,
                                );
                                handle_claim_request(
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
                                debug!(error = %e, "rejected seat-claim event");
                            }
                        },
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
                        30507 => match state::parse_session_state_request(&event) {
                            Ok(req) => {
                                let span = info_span!(
                                    "state_request",
                                    player_npub = %req.player_pubkey,
                                    nonce = %req.nonce,
                                    game_id = %req.game_id,
                                );
                                handle_session_state_request(
                                    req,
                                    shared_dirs,
                                    shared_keys,
                                    client_clone,
                                )
                                .instrument(span)
                                .await;
                            }
                            Err(e) => {
                                debug!(error = %e, "rejected state event");
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

pub fn request_subscription_filter(since: Timestamp) -> Filter {
    Filter::new()
        .kinds([
            Kind::Custom(30501),
            Kind::Custom(30504),
            Kind::Custom(30507),
            Kind::Custom(30510),
        ])
        .since(since)
}

fn is_targeted_to_gate(event: &nostr_sdk::Event, gate_pubkey: &PublicKey) -> bool {
    event.tags.iter().any(|tag| {
        if tag.kind().as_str() != "p" {
            return false;
        }
        // Standard NIP-01 'p' tag: ["p", <pubkey>, <relay-url>, <alias>]
        // nostr-sdk 0.44 Tag::content() returns the first value (the pubkey).
        tag.content()
            .and_then(|value| PublicKey::parse(value).ok())
            .map(|pubkey| pubkey == *gate_pubkey)
            .unwrap_or(false)
    })
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

async fn handle_session_state_request(
    req: state::SessionStateRequest,
    shared_dirs: Arc<Vec<std::path::PathBuf>>,
    shared_keys: Arc<Keys>,
    client: Client,
) {
    let player_pubkey = match PublicKey::from_hex(&req.player_pubkey) {
        Ok(pk) => pk,
        Err(err) => {
            warn!(error = %err, "cannot parse player pubkey for state request");
            return;
        }
    };

    let loaded_games = match catalog::load_hosted_games(shared_dirs.as_slice()) {
        Ok(games) => games,
        Err(err) => {
            warn!(error = %err, "cannot load hosted games for state request");
            if let Err(pub_err) = state::publish_session_state_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "internal_error",
                "Unable to refresh game metadata right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30509 SessionStateError");
            }
            return;
        }
    };

    let seat =
        match routing::resolve_player_in_game(&req.player_pubkey, &req.game_id, &loaded_games) {
            Ok(seat) => seat,
            Err(routing::RouteError::GameNotFound) => {
                if let Err(err) = state::publish_session_state_error(
                    &client,
                    &shared_keys,
                    &player_pubkey,
                    &req.nonce,
                    "game_not_found",
                    "The requested game was not found on this server.",
                )
                .await
                {
                    error!(error = %err, "failed to publish 30509 SessionStateError");
                }
                return;
            }
            Err(_) => {
                if let Err(err) = state::publish_session_state_error(
                    &client,
                    &shared_keys,
                    &player_pubkey,
                    &req.nonce,
                    "unknown_player",
                    "Your identity is not enrolled in that game.",
                )
                .await
                {
                    error!(error = %err, "failed to publish 30509 SessionStateError");
                }
                return;
            }
        };

    let Some(game_entry) = loaded_games
        .iter()
        .find(|entry| entry.game.game_id == seat.game_id)
    else {
        if let Err(err) = state::publish_session_state_error(
            &client,
            &shared_keys,
            &player_pubkey,
            &req.nonce,
            "internal_error",
            "Unable to refresh game metadata right now.",
        )
        .await
        {
            error!(error = %err, "failed to publish 30509 SessionStateError");
        }
        return;
    };

    let player_name = match player_name_for_seat(&game_entry.dir, seat.player) {
        Ok(player_name) => player_name,
        Err(err) => {
            error!(game_id = %seat.game_id, error = %err, "cannot load runtime player state");
            if let Err(pub_err) = state::publish_session_state_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "internal_error",
                "Unable to refresh game metadata right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30509 SessionStateError");
            }
            return;
        }
    };

    let payload = state::SessionStatePayload {
        game_id: seat.game_id.clone(),
        game_name: seat.game_name.clone(),
        seat: seat.player as u32,
        player_name,
    };

    match state::publish_session_state(&client, &shared_keys, &player_pubkey, &req.nonce, &payload)
        .await
    {
        Ok(event_id) => {
            info!(
                game_id = %payload.game_id,
                seat = payload.seat,
                event_id = %event_id,
                "published 30508 SessionStateReady"
            );
        }
        Err(err) => {
            error!(error = %err, "failed to publish 30508 SessionStateReady");
        }
    }
}

async fn handle_claim_request(
    req: claim::SeatClaimRequest,
    shared_dirs: Arc<Vec<std::path::PathBuf>>,
    shared_keys: Arc<Keys>,
    shared_config: Arc<GateConfig>,
    client: Client,
) {
    let player_pubkey = match PublicKey::from_hex(&req.player_pubkey) {
        Ok(pk) => pk,
        Err(err) => {
            warn!(error = %err, "cannot parse player pubkey for seat claim");
            return;
        }
    };

    let loaded_games = match catalog::load_hosted_games(shared_dirs.as_slice()) {
        Ok(games) => games,
        Err(err) => {
            error!(error = %err, "cannot load hosted games for seat claim");
            if let Err(pub_err) = claim::publish_seat_claim_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "internal_error",
                "Unable to process this invite right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30511 SeatClaimError");
            }
            return;
        }
    };

    let Some(game_entry) =
        resolve_claim_target(&loaded_games, req.game_id.as_deref(), &req.invite_code)
    else {
        let (code, message) = if req.game_id.is_some() {
            (
                "invalid_code",
                "The invite code does not match this hosted game.",
            )
        } else {
            (
                "game_not_found",
                "The invite code is not valid for any hosted game on this server.",
            )
        };
        if let Err(err) = claim::publish_seat_claim_error(
            &client,
            &shared_keys,
            &player_pubkey,
            &req.nonce,
            code,
            message,
        )
        .await
        {
            error!(error = %err, "failed to publish 30511 SeatClaimError");
        }
        return;
    };

    let store = match ec_data::CampaignStore::open_default_in_dir(&game_entry.dir) {
        Ok(store) => store,
        Err(err) => {
            error!(game_id = %game_entry.game.game_id, error = %err, "cannot open campaign store for claim");
            if let Err(pub_err) = claim::publish_seat_claim_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "internal_error",
                "Unable to process this invite right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30511 SeatClaimError");
            }
            return;
        }
    };

    match store.claim_hosted_seat(&req.invite_code, &req.player_pubkey) {
        Ok(_) => {
            info!(
                invite_code = %req.invite_code,
                player_npub = %req.player_pubkey,
                game_id = %game_entry.game.game_id,
                "seat claimed"
            );
            match catalog::load_hosted_game(&game_entry.dir) {
                Ok(entry) => match game_def::publish_game_definition(
                    &client,
                    &shared_keys,
                    &entry.game,
                    &shared_config.ssh_host,
                    shared_config.ssh_port,
                    &shared_config.relay,
                )
                .await
                {
                    Ok(event_id) => info!(
                        game_id = %entry.game.game_id,
                        event_id = %event_id,
                        "published updated 30500 GameDefinition after claim"
                    ),
                    Err(err) => warn!(
                        game_id = %entry.game.game_id,
                        error = %err,
                        "failed to publish updated 30500 GameDefinition after claim"
                    ),
                },
                Err(err) => warn!(
                    game_id = %game_entry.game.game_id,
                    error = %err,
                    "cannot reload hosted game after claim"
                ),
            }
        }
        Err(ec_data::ClaimHostedSeatError::InvalidCode) => {
            if let Err(err) = claim::publish_seat_claim_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "invalid_code",
                "The invite code is not valid.",
            )
            .await
            {
                error!(error = %err, "failed to publish 30511 SeatClaimError");
            }
        }
        Err(ec_data::ClaimHostedSeatError::CodeClaimed) => {
            if let Err(err) = claim::publish_seat_claim_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "code_claimed",
                "The invite code has already been claimed.",
            )
            .await
            {
                error!(error = %err, "failed to publish 30511 SeatClaimError");
            }
        }
        Err(ec_data::ClaimHostedSeatError::Store(err)) => {
            error!(game_id = %game_entry.game.game_id, error = %err, "claim failed");
            if let Err(pub_err) = claim::publish_seat_claim_error(
                &client,
                &shared_keys,
                &player_pubkey,
                &req.nonce,
                "internal_error",
                "Unable to process this invite right now.",
            )
            .await
            {
                error!(error = %pub_err, "failed to publish 30511 SeatClaimError");
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
                                &shared_config.ssh_host,
                                shared_config.ssh_port,
                                &shared_config.relay,
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

            let store = match ec_data::CampaignStore::open_default_in_dir(&game_dir) {
                Ok(store) => store,
                Err(err) => {
                    error!(game_id = %seat.game_id, error = %err, "cannot open campaign store for session lease");
                    return;
                }
            };
            let session_token = provision::new_session_token();
            match store.create_pending_session_lease(
                &session_token,
                seat.player,
                &req.player_pubkey,
                unix_now(),
                shared_config.key_ttl,
            ) {
                Ok(_) => {}
                Err(ec_data::SessionLeaseError::SeatBusy { .. }) => {
                    if let Err(err) = response::publish_session_error_message(
                        &client,
                        &shared_keys,
                        &player_pubkey,
                        &req.nonce,
                        "seat_busy",
                        "That seat already has an active session.",
                    )
                    .await
                    {
                        error!(error = %err, "failed to publish 30503 SessionError");
                    }
                    return;
                }
                Err(ec_data::SessionLeaseError::InvalidToken) => {
                    error!(game_id = %seat.game_id, "new session token rejected unexpectedly");
                    return;
                }
                Err(ec_data::SessionLeaseError::Store(err)) => {
                    error!(game_id = %seat.game_id, error = %err, "cannot create session lease");
                    return;
                }
            }

            match provision::provision_key(
                &shared_config,
                &seat,
                &req.ssh_pubkey,
                &game_dir,
                &session_token,
            ) {
                Ok(provisioned) => {
                    let player_name = match player_name_for_seat(&game_dir, seat.player) {
                        Ok(player_name) => player_name,
                        Err(err) => {
                            let _ = store.release_session_lease(&session_token);
                            error!(game_id = %seat.game_id, error = %err, "cannot load runtime player state");
                            if let Err(pub_err) = response::publish_session_error_message(
                                &client,
                                &shared_keys,
                                &player_pubkey,
                                &req.nonce,
                                "internal_error",
                                "Unable to read campaign runtime state right now.",
                            )
                            .await
                            {
                                error!(error = %pub_err, "failed to publish 30503 SessionError");
                            }
                            return;
                        }
                    };
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
                        let _ = store.release_session_lease(&session_token);
                        error!(error = %e, "failed to publish 30502 SessionReady");
                    }
                }
                Err(e) => {
                    let _ = store.release_session_lease(&session_token);
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

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn player_name_for_seat(
    game_dir: &Path,
    player_index_1_based: usize,
) -> Result<String, ec_data::CampaignStoreError> {
    let store = CampaignStore::open_default_in_dir(game_dir)?;
    let game_data = store.load_latest_runtime_game_data()?;
    Ok(game_data
        .player
        .records
        .get(player_index_1_based.saturating_sub(1))
        .map(|record| record.controlled_empire_name_summary())
        .unwrap_or_default())
}

fn resolve_claim_target<'a>(
    games: &'a [catalog::HostedGameEntry],
    game_id: Option<&str>,
    invite_code: &str,
) -> Option<&'a catalog::HostedGameEntry> {
    let normalized = invite_code.trim().to_ascii_lowercase();
    if let Some(game_id) = game_id {
        return games.iter().find(|entry| {
            entry.game.game_id == game_id
                && entry
                    .game
                    .seats
                    .iter()
                    .any(|seat| seat.invite_code == normalized)
        });
    }
    games.iter().find(|entry| {
        entry
            .game
            .seats
            .iter()
            .any(|seat| seat.invite_code == normalized)
    })
}
