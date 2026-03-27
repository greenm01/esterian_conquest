//! Daemon identity: Nostr keypair stored in `identity.kdl`.
//!
//! The identity is generated once on `ec-gate init` and never changed.
//! It is used to sign all events published by ec-gate.

pub mod io;

use nostr_sdk::Keys;

pub use io::{identity_path, load_identity, save_identity};

/// The daemon's Nostr identity, loaded from `identity.kdl`.
pub struct DaemonIdentity {
    /// The loaded keypair. Used to sign Nostr events.
    pub keys: Keys,
    /// ISO-8601 UTC timestamp recorded when the identity was first created.
    pub created: String,
}
