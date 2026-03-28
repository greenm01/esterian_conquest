use nostr_sdk::{Keys, ToBech32};

use crate::wallet::Wallet;
use crate::wallet::io::{load_wallet_from, now_iso8601, save_wallet_to};
use crate::wallet::{push_new_identity, switch_active_identity};

use super::state::PickerSession;

impl PickerSession {
    pub fn header_identity_label(&self) -> String {
        self.active_alias()
            .map(str::to_string)
            .unwrap_or_else(|| super::render::short_npub(&self.npub))
    }

    pub fn active_alias(&self) -> Option<&str> {
        self.wallet
            .active_identity()
            .and_then(|identity| identity.alias.as_deref())
            .filter(|alias| !alias.is_empty())
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

    pub fn switch_active(&mut self, index: usize) -> Result<String, Box<dyn std::error::Error>> {
        let npub = switch_active_identity(&mut self.wallet, index)?;
        self.refresh_active_identity()?;
        Ok(npub)
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
    let mut wallet = load_wallet_from(&password, &path)?.unwrap_or_else(Wallet::empty);
    if wallet.identities.is_empty() {
        let npub = push_new_identity(&mut wallet, now_iso8601())?;
        save_wallet_to(&wallet, &password, &path)?;
        eprintln!("Identity created: {npub}");
    }

    let identity = wallet
        .active_identity()
        .ok_or("wallet has no active identity")?;
    let keys = Keys::parse(&identity.nsec)?;
    let npub = keys.public_key().to_bech32()?;

    Ok(PickerSession {
        password,
        wallet,
        keys,
        npub,
    })
}
