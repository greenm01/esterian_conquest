pub mod crypto;
pub mod io;

use nostr_sdk::{Keys, ToBech32};

pub use io::{keychain_path, load_keychain, load_keychain_from, now_iso8601, save_keychain, save_keychain_to};

pub const MAX_IDENTITIES: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityType {
    Local,
    Imported,
}

impl IdentityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Imported => "imported",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "local" => Ok(Self::Local),
            "imported" => Ok(Self::Imported),
            other => Err(format!("unknown identity type: {other}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub nsec: String,
    pub identity_type: IdentityType,
    pub created: String,
    pub handle: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Keychain {
    pub active: usize,
    pub identities: Vec<Identity>,
}

impl Keychain {
    pub fn empty() -> Self {
        Self {
            active: 0,
            identities: Vec::new(),
        }
    }

    pub fn active_identity(&self) -> Option<&Identity> {
        self.identities.get(self.active)
    }

    pub fn active_identity_mut(&mut self) -> Option<&mut Identity> {
        self.identities.get_mut(self.active)
    }
}

pub fn identity_npub(identity: &Identity) -> Result<String, Box<dyn std::error::Error>> {
    let keys = Keys::parse(&identity.nsec)?;
    Ok(keys.public_key().to_bech32()?)
}

pub fn active_identity_npub(keychain: &Keychain) -> Result<String, Box<dyn std::error::Error>> {
    let identity = keychain
        .active_identity()
        .ok_or("keychain has no active identity")?;
    identity_npub(identity)
}

pub fn push_new_identity(
    keychain: &mut Keychain,
    created: String,
    handle: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    if keychain.identities.len() >= MAX_IDENTITIES {
        return Err(format!("keychain already has {MAX_IDENTITIES} identities (maximum)").into());
    }

    let keys = Keys::generate();
    let npub = keys.public_key().to_bech32()?;
    keychain.identities.push(Identity {
        nsec: keys.secret_key().to_bech32()?,
        identity_type: IdentityType::Local,
        created,
        handle: sanitize_handle(handle),
    });
    Ok(npub)
}

pub fn set_active_handle(keychain: &mut Keychain, handle: Option<String>) -> Result<(), String> {
    let identity = keychain
        .active_identity_mut()
        .ok_or_else(|| "keychain has no active identity".to_string())?;
    identity.handle = sanitize_handle(handle);
    Ok(())
}

pub fn active_keys(keychain: &Keychain) -> Result<Keys, Box<dyn std::error::Error>> {
    let identity = keychain
        .active_identity()
        .ok_or("keychain has no active identity")?;
    Ok(Keys::parse(&identity.nsec)?)
}

fn sanitize_handle(handle: Option<String>) -> Option<String> {
    handle
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
