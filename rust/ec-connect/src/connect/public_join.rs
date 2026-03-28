use std::path::Path;

use nostr_sdk::Keys;

use crate::connect::game_discovery::discover_game_for_invite;
use crate::connect::resolve::ResolvedTarget;
use crate::connect::seat_claim::{SeatClaimResult, claim_seat_and_wait};
use crate::connect::session::{
    DisambigMode, SessionOutcome, build_cached_game, cache_joined_game, run_session,
};

pub async fn run_public_join(
    player_keys: &Keys,
    target: ResolvedTarget,
    npub: &str,
    disambig: DisambigMode,
    maps_root: &Path,
) -> Result<SessionOutcome, Box<dyn std::error::Error + Send + Sync>> {
    let Some(invite_code) = target.invite_code.clone() else {
        return Err("public join requires an invite code".into());
    };

    let discovered = discover_game_for_invite(player_keys, &target, &invite_code).await?;
    match claim_seat_and_wait(player_keys, &target, &invite_code, &discovered).await? {
        SeatClaimResult::Claimed(claimed) => {
            let mut session_target = target;
            session_target.game_id = Some(claimed.game_id.clone());
            session_target.invite_code = None;
            cache_joined_game(build_cached_game(
                &claimed.game_id,
                &claimed.game_name,
                None,
                &session_target,
                npub,
                &claimed.gate_npub,
                claimed.seat,
            ));
            Ok(run_session(
                player_keys,
                session_target,
                npub,
                &claimed.gate_npub,
                disambig,
                maps_root,
            )
            .await)
        }
        SeatClaimResult::Error(err) => Ok(SessionOutcome::Error(format!(
            "{}: {}",
            err.error, err.message
        ))),
        SeatClaimResult::Timeout => Ok(SessionOutcome::Error(
            "invite claim timed out (no game update from relay)".to_string(),
        )),
    }
}
