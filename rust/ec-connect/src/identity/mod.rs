//! Identity CLI subcommands: id, id --secret, id list, id new, id import, id switch N.
//!
//! All subcommands that read or modify the wallet prompt for a password on
//! stdin.  Password reading uses the `rpassword` crate so the typed
//! characters are not echoed.  In test mode the helpers accept an explicit
//! password string, keeping the module testable without stdin interaction.

use nostr_sdk::{Keys, ToBech32};

use crate::wallet::io::{load_wallet_from, now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{Identity, IdentityType, Wallet};

// ---------------------------------------------------------------------------
// Public entry points (called from cli.rs)
// ---------------------------------------------------------------------------

/// `ec-connect id` — show active identity npub.
pub fn cmd_id_show() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let wallet = require_wallet(&password)?;
    let id = require_active(&wallet)?;
    let keys = Keys::parse(&id.nsec)?;
    let npub = keys.public_key().to_bech32()?;
    println!("{npub}");
    Ok(())
}

/// `ec-connect id --secret` — show active identity npub + nsec.
pub fn cmd_id_secret() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let wallet = require_wallet(&password)?;
    let id = require_active(&wallet)?;
    let keys = Keys::parse(&id.nsec)?;
    let npub = keys.public_key().to_bech32()?;
    println!("npub: {npub}");
    println!("nsec: {}", id.nsec);
    Ok(())
}

/// `ec-connect id list` — list all identities.
pub fn cmd_id_list() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let wallet = require_wallet(&password)?;
    if wallet.identities.is_empty() {
        println!("No identities in wallet.");
        return Ok(());
    }
    for (i, id) in wallet.identities.iter().enumerate() {
        let keys = Keys::parse(&id.nsec)?;
        let npub = keys.public_key().to_bech32()?;
        let marker = if i == wallet.active { "*" } else { " " };
        println!(
            "{marker} [{i}] {npub}  ({})  created {}",
            id.identity_type.as_str(),
            id.created,
        );
    }
    Ok(())
}

/// `ec-connect id new` — generate a new keypair and add it to the wallet.
pub fn cmd_id_new() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let path = wallet_path();
    let mut wallet = load_wallet_from(&password, &path)?.unwrap_or_else(Wallet::empty);

    if wallet.identities.len() >= 10 {
        return Err("wallet already has 10 identities (maximum)".into());
    }

    let keys = Keys::generate();
    let nsec = keys.secret_key().to_bech32()?;
    let npub = keys.public_key().to_bech32()?;
    let created = now_iso8601();

    wallet.identities.push(Identity {
        nsec,
        identity_type: IdentityType::Local,
        created,
    });
    save_wallet_to(&wallet, &password, &path)?;

    println!("New identity created: {npub}");
    Ok(())
}

/// `ec-connect id import` — import an existing nsec.
pub fn cmd_id_import() -> Result<(), Box<dyn std::error::Error>> {
    let nsec_input = prompt_line("Enter your nsec: ")?;
    let nsec_input = nsec_input.trim().to_string();

    // Validate by parsing — accepts bech32 nsec or 64-char hex.
    let keys = Keys::parse(&nsec_input).map_err(|e| format!("invalid nsec: {e}"))?;
    let nsec = keys.secret_key().to_bech32()?;
    let npub = keys.public_key().to_bech32()?;

    let password = prompt_password("Password: ")?;
    let path = wallet_path();
    let mut wallet = load_wallet_from(&password, &path)?.unwrap_or_else(Wallet::empty);

    if wallet.identities.len() >= 10 {
        return Err("wallet already has 10 identities (maximum)".into());
    }

    // Reject duplicate.
    for id in &wallet.identities {
        if id.nsec == nsec {
            return Err("this identity is already in the wallet".into());
        }
    }

    wallet.identities.push(Identity {
        nsec,
        identity_type: IdentityType::Imported,
        created: now_iso8601(),
    });
    save_wallet_to(&wallet, &password, &path)?;

    println!("Identity imported: {npub}");
    Ok(())
}

/// `ec-connect id switch N` — change the active identity to index N.
pub fn cmd_id_switch(n_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let n: usize = n_str
        .parse()
        .map_err(|_| format!("invalid index: {n_str}"))?;

    let password = prompt_password("Password: ")?;
    let path = wallet_path();
    let mut wallet = require_wallet_at(&password, &path)?;

    if n >= wallet.identities.len() {
        return Err(format!(
            "index {n} out of range (wallet has {} identit{})",
            wallet.identities.len(),
            if wallet.identities.len() == 1 {
                "y"
            } else {
                "ies"
            },
        )
        .into());
    }

    wallet.active = n;
    save_wallet_to(&wallet, &password, &path)?;

    let keys = Keys::parse(&wallet.identities[n].nsec)?;
    let npub = keys.public_key().to_bech32()?;
    println!("Active identity: [{n}] {npub}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Prompt for a password without echoing.
fn prompt_password(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    rpassword::prompt_password(prompt).map_err(|e| e.into())
}

/// Prompt for a plain (echoed) line of input.
fn prompt_line(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::{self, Write};
    print!("{prompt}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf)
}

/// Load the wallet from the default path, returning an error if it doesn't exist.
fn require_wallet(password: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
    let path = wallet_path();
    require_wallet_at(password, &path)
}

fn require_wallet_at(
    password: &str,
    path: &std::path::Path,
) -> Result<Wallet, Box<dyn std::error::Error>> {
    load_wallet_from(password, path)?
        .ok_or_else(|| "no wallet found; run `ec-connect id new` to create one".into())
}

/// Return the active identity from the wallet, or an error if none.
fn require_active(wallet: &Wallet) -> Result<&Identity, Box<dyn std::error::Error>> {
    wallet
        .active_identity()
        .ok_or_else(|| "wallet has no identities; run `ec-connect id new` to create one".into())
}
