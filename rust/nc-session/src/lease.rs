//! Session lease management — heartbeat and cleanup for hosted sessions.

use nc_data::CampaignStore;

/// Guards a hosted session lease. Releases on drop.
pub struct SessionLeaseGuard {
    store: CampaignStore,
    pub player_npub: String,
    pub session_token: String,
    ttl_seconds: u64,
}

impl SessionLeaseGuard {
    pub fn activate(
        store: CampaignStore,
        session_token: String,
        now_unix_seconds: u64,
        ttl_seconds: u64,
        player_npub: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        store.activate_session_lease(&session_token, now_unix_seconds, ttl_seconds)?;
        Ok(Self {
            store,
            player_npub,
            session_token,
            ttl_seconds,
        })
    }

    pub fn heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.store
            .heartbeat_session_lease(&self.session_token, unix_now(), self.ttl_seconds)?;
        Ok(())
    }

    pub fn release(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.store.release_session_lease(&self.session_token)?;
        Ok(())
    }
}

impl Drop for SessionLeaseGuard {
    fn drop(&mut self) {
        let _ = self.store.release_session_lease(&self.session_token);
    }
}

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
