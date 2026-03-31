use nostr_sdk::{Keys, ToBech32};

use crate::cache::GameCache;
use crate::wallet::Wallet;
use crate::wallet::io::{load_wallet_from, save_wallet_to};
use crate::wallet::{Identity, push_identity_from_input};

use super::state::PickerSession;

impl PickerSession {
    pub fn header_identity_label(&self) -> String {
        super::render::short_npub(&self.npub)
    }

    pub fn active_identity_type(&self) -> &'static str {
        self.wallet
            .active_identity()
            .map(|identity| identity.identity_type.as_str())
            .unwrap_or("local")
    }

    pub fn refresh_active_identity(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let identity = self
            .wallet
            .active_identity()
            .ok_or("wallet has no active identity")?;
        self.keys = Keys::parse(&identity.nsec)?;
        self.npub = self.keys.public_key().to_bech32()?;
        Ok(())
    }

    pub fn normalize_for_gui(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.wallet.identities.len() <= 1 && self.wallet.active == 0 {
            self.refresh_active_identity()?;
            return Ok(false);
        }
        let identity = self
            .wallet
            .active_identity()
            .cloned()
            .ok_or("wallet has no active identity")?;
        self.wallet = Wallet {
            active: 0,
            identities: vec![identity],
        };
        self.refresh_active_identity()?;
        Ok(true)
    }

    pub fn replace_active_identity(
        &mut self,
        input: &str,
        cache: &mut GameCache,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let old_npub = self.npub.clone();
        let mut wallet = Wallet::empty();
        push_identity_from_input(&mut wallet, input, crate::wallet::io::now_iso8601())?;
        self.wallet = wallet;
        self.refresh_active_identity()?;
        if self.npub != old_npub {
            let _ = cache.remove_by_npub(&old_npub);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn selected_identity(&self, index: usize) -> Option<&Identity> {
        self.wallet.identities.get(index)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        save_wallet_to(
            &self.wallet,
            &self.password,
            &crate::wallet::io::wallet_path(),
        )
    }
}

pub fn load_picker_session(password: String) -> Result<PickerSession, Box<dyn std::error::Error>> {
    let path = crate::wallet::io::wallet_path();
    let wallet = load_wallet_from(&password, &path)?.unwrap_or_else(Wallet::empty);

    let identity = wallet
        .active_identity()
        .ok_or("wallet has no active identity")?;
    let keys = Keys::parse(&identity.nsec)?;
    let npub = keys.public_key().to_bech32()?;

    let mut session = PickerSession {
        password,
        wallet,
        keys,
        npub,
    };
    if session.normalize_for_gui()? {
        session.save()?;
    }
    Ok(session)
}
