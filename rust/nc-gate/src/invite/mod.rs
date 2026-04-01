//! Invite code generation for nc-gate.
//!
//! Codes are two words from the Monero mnemonic wordlist, hyphenated and
//! lowercase (e.g. `velvet-mountain`). With 1626 words there are roughly
//! 2.6 million possible combinations, far more than needed for typical
//! server deployments.

pub mod generate;
pub mod wordlist;

pub use generate::{generate_invite_code, is_valid_invite_code};
