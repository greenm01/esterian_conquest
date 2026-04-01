use nc_data::{CampaignStore, CampaignStoreError, SessionLease};

use crate::serve::catalog::HostedGameEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveIdentitySession {
    pub game_id: String,
    pub player_record_index_1_based: usize,
    pub lease: SessionLease,
}

pub fn find_active_identity_session(
    games: &[HostedGameEntry],
    player_npub: &str,
    now_unix_seconds: u64,
) -> Result<Option<ActiveIdentitySession>, CampaignStoreError> {
    for game in games {
        let store = CampaignStore::open_default_in_dir(&game.dir)?;
        if let Some(lease) = store.live_session_for_npub(player_npub, now_unix_seconds)? {
            return Ok(Some(ActiveIdentitySession {
                game_id: game.game.game_id.clone(),
                player_record_index_1_based: lease.player_record_index_1_based,
                lease,
            }));
        }
    }

    Ok(None)
}
