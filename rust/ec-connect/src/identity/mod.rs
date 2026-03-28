//! Identity CLI subcommands: id, id --secret, id list, id new, id import, id switch N.
//!
//! All subcommands that read or modify the wallet prompt for a password on
//! stdin.  Password reading uses the `rpassword` crate so the typed
//! characters are not echoed.  In test mode the helpers accept an explicit
//! password string, keeping the module testable without stdin interaction.

use nostr_sdk::{Keys, ToBech32};

use crate::cache::io::cache_path;
use crate::password::{
    prompt_confirm_yn, prompt_line, prompt_new_password_with_warning, prompt_optional_alias,
    prompt_password, wallet_exists,
};
use crate::wallet::io::{load_wallet_from, now_iso8601, save_wallet_to, wallet_path};
use crate::wallet::{
    active_identity_npub, push_imported_identity, push_new_identity, set_identity_alias,
    switch_active_identity, Identity, Wallet,
};

// ---------------------------------------------------------------------------
// Public entry points (called from cli.rs)
// ---------------------------------------------------------------------------

/// `ec-connect id` — show active identity npub.
pub fn cmd_id_show() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let wallet = require_wallet(&password)?;
    println!("{}", active_identity_npub(&wallet)?);
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
            "{marker} [{i}] {}{npub}  ({})  created {}",
            id.alias
                .as_deref()
                .map(|alias| format!("{alias}  "))
                .unwrap_or_default(),
            id.identity_type.as_str(),
            id.created,
        );
    }
    Ok(())
}

/// `ec-connect id new` — generate a new keypair and add it to the wallet.
pub fn cmd_id_new() -> Result<(), Box<dyn std::error::Error>> {
    let path = wallet_path();
    let password = prompt_wallet_password_for_write(&path)?;
    let mut wallet = load_wallet_from(&password, &path)?.unwrap_or_else(Wallet::empty);
    if !wallet.identities.is_empty() {
        let n = wallet.identities.len();
        println!(
            "Wallet already contains {} {}.",
            n,
            if n == 1 { "identity" } else { "identities" }
        );
        if !prompt_confirm_yn("Add another Nostr keypair? [y/N]: ")? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    let npub = push_new_identity(&mut wallet, now_iso8601())?;
    let index = wallet.identities.len().saturating_sub(1);
    set_identity_alias(&mut wallet, index, prompt_optional_alias()?)?;
    save_wallet_to(&wallet, &password, &path)?;

    println!("New Nostr keypair created: {npub}");
    Ok(())
}

/// `ec-connect id import` — import an existing nsec.
pub fn cmd_id_import() -> Result<(), Box<dyn std::error::Error>> {
    let nsec_input = prompt_line("Enter your nsec: ")?;
    let nsec_input = nsec_input.trim().to_string();

    let path = wallet_path();
    let password = prompt_wallet_password_for_write(&path)?;
    let mut wallet = load_wallet_from(&password, &path)?.unwrap_or_else(Wallet::empty);
    if !wallet.identities.is_empty() {
        let n = wallet.identities.len();
        println!(
            "Wallet already contains {} {}.",
            n,
            if n == 1 { "identity" } else { "identities" }
        );
        if !prompt_confirm_yn("Import Nostr keypair into this wallet? [y/N]: ")? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    let npub = push_imported_identity(&mut wallet, &nsec_input, now_iso8601())?;
    let index = wallet.identities.len().saturating_sub(1);
    set_identity_alias(&mut wallet, index, prompt_optional_alias()?)?;
    save_wallet_to(&wallet, &password, &path)?;

    println!("Nostr keypair imported: {npub}");
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

    let npub = switch_active_identity(&mut wallet, n)?;
    save_wallet_to(&wallet, &password, &path)?;
    println!("Active identity: [{n}] {npub}");
    Ok(())
}

/// `ec-connect id reset` — verify password, triple-confirm, then wipe wallet + cache.
pub fn cmd_id_reset() -> Result<(), Box<dyn std::error::Error>> {
    let path = wallet_path();
    if !wallet_exists(&path) {
        println!("No wallet found. Nothing to reset.");
        return Ok(());
    }

    // Verify the current password actually decrypts the wallet.
    let password = prompt_password("Current password: ")?;
    let _ = require_wallet_at(&password, &path)?;

    // Triple confirmation — plain stdin readline, no echo suppression needed.
    println!();
    println!("WARNING: This will permanently delete your wallet and all identities.");
    println!("         There is no recovery unless you have a backup of your nsec.");
    println!();
    for (i, prompt) in [
        "Type YES to confirm reset (1/3): ",
        "Type YES to confirm reset (2/3): ",
        "Type YES to confirm reset (3/3): ",
    ]
    .iter()
    .enumerate()
    {
        let answer = prompt_line(prompt)?;
        if answer.trim() != "YES" {
            println!("Reset cancelled ({}/3).", i + 1);
            return Ok(());
        }
    }

    // Delete wallet.
    std::fs::remove_file(&path)?;

    // Delete cache if it exists (best-effort; not a fatal error if absent).
    let cp = cache_path();
    if cp.exists() {
        let _ = std::fs::remove_file(&cp);
    }

    println!("Wallet reset. Run ec-connect to create a new identity.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn prompt_wallet_password_for_write(
    path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    if wallet_exists(path) {
        prompt_password("Password: ")
    } else {
        println!("No existing wallet found. Creating a new one.");
        prompt_new_password_with_warning()
    }
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
