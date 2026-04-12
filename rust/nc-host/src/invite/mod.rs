//! Invite code generation for nc-host.
//!
//! Codes are two words from the Monero mnemonic wordlist, hyphenated and
//! lowercase (e.g. `velvet-mountain`). With 1626 words there are roughly
//! 2.6 million possible combinations.

pub mod generate;
pub mod wordlist;

pub use generate::generate_invite_code;
