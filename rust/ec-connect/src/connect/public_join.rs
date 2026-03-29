use crate::config::seed_default_relay;
use std::path::Path;

use nostr_sdk::Keys;

use crate::connect::game_discovery::discover_game_for_invite;
use crate::connect::resolve::ResolvedTarget;
use crate::connect::session::{
    DisambigMode, SessionOutcome, SessionPreparation, finish_prepared_session, prepare_session,
};

pub async fn run_public_join(
    player_keys: &Keys,
    target: ResolvedTarget,
    npub: &str,
    disambig: DisambigMode,
    maps_root: &Path,
) -> Result<SessionOutcome, Box<dyn std::error::Error + Send + Sync>> {
    match prepare_public_join(player_keys, target, npub, disambig, maps_root).await? {
        SessionPreparation::Ready(prepared) => Ok(finish_prepared_session(prepared, npub).await),
        SessionPreparation::Outcome(outcome) => Ok(outcome),
    }
}

pub async fn prepare_public_join(
    player_keys: &Keys,
    target: ResolvedTarget,
    npub: &str,
    disambig: DisambigMode,
    maps_root: &Path,
) -> Result<SessionPreparation, Box<dyn std::error::Error + Send + Sync>> {
    let Some(invite_code) = target.invite_code.clone() else {
        return Err("public join requires an invite code".into());
    };

    let discovered = discover_game_for_invite(player_keys, &target, &invite_code).await?;
    let mut session_target = target;
    session_target.server_host = discovered.ssh_host;
    session_target.server_port = discovered.ssh_port;
    session_target.game_id = Some(discovered.game_id);
    let _ = seed_default_relay(&session_target.relay_url);
    Ok(prepare_session(
        player_keys,
        session_target,
        npub,
        &discovered.gate_npub,
        disambig,
        maps_root,
    )
    .await)
}
