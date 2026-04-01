//! Identity CLI subcommands: id, id --secret, id list, id new, id import, id switch N.
//!
//! All subcommands that read or modify the keychain prompt for a password on
//! stdin.  Password reading uses the `rpassword` crate so the typed
//! characters are not echoed.  In test mode the helpers accept an explicit
//! password string, keeping the module testable without stdin interaction.

use nostr_sdk::{Keys, ToBech32};

use crate::cache::io::cache_path;
use crate::keychain::io::{keychain_path, load_keychain_from, now_iso8601, save_keychain_to};
use crate::keychain::{
    Identity, Keychain, active_identity_npub, push_imported_identity, push_new_identity,
    set_identity_alias, switch_active_identity,
};
use crate::password::{
    keychain_exists, prompt_confirm_yn, prompt_line, prompt_new_password_with_warning,
    prompt_optional_alias, prompt_password,
};

// ---------------------------------------------------------------------------
// Public entry points (called from cli.rs)
// ---------------------------------------------------------------------------

/// `nc-connect id` — show active identity npub.
pub fn cmd_id_show() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let keychain = require_keychain(&password)?;
    println!("{}", active_identity_npub(&keychain)?);
    Ok(())
}

/// `nc-connect id --secret` — show active identity npub + nsec.
pub fn cmd_id_secret() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let keychain = require_keychain(&password)?;
    let id = require_active(&keychain)?;
    let keys = Keys::parse(&id.nsec)?;
    let npub = keys.public_key().to_bech32()?;
    println!("npub: {npub}");
    println!("nsec: {}", id.nsec);
    Ok(())
}

/// `nc-connect id list` — list all identities.
pub fn cmd_id_list() -> Result<(), Box<dyn std::error::Error>> {
    let password = prompt_password("Password: ")?;
    let keychain = require_keychain(&password)?;
    if keychain.identities.is_empty() {
        println!("No identities in keychain.");
        return Ok(());
    }
    for (i, id) in keychain.identities.iter().enumerate() {
        let keys = Keys::parse(&id.nsec)?;
        let npub = keys.public_key().to_bech32()?;
        let marker = if i == keychain.active { "*" } else { " " };
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

/// `nc-connect id new` — generate a new keypair and add it to the keychain.
pub fn cmd_id_new() -> Result<(), Box<dyn std::error::Error>> {
    let path = keychain_path();
    let password = prompt_keychain_password_for_write(&path)?;
    let mut keychain = load_keychain_from(&password, &path)?.unwrap_or_else(Keychain::empty);
    if !keychain.identities.is_empty() {
        let n = keychain.identities.len();
        println!(
            "Keychain already contains {} {}.",
            n,
            if n == 1 { "identity" } else { "identities" }
        );
        if !prompt_confirm_yn("Add another Nostr keypair? [y/N]: ")? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    let npub = push_new_identity(&mut keychain, now_iso8601())?;
    let index = keychain.identities.len().saturating_sub(1);
    set_identity_alias(&mut keychain, index, prompt_optional_alias()?)?;
    save_keychain_to(&keychain, &password, &path)?;

    println!("New Nostr keypair created: {npub}");
    Ok(())
}

/// `nc-connect id import` — import an existing nsec.
pub fn cmd_id_import() -> Result<(), Box<dyn std::error::Error>> {
    let nsec_input = prompt_line("Enter your nsec: ")?;
    let nsec_input = nsec_input.trim().to_string();

    let path = keychain_path();
    let password = prompt_keychain_password_for_write(&path)?;
    let mut keychain = load_keychain_from(&password, &path)?.unwrap_or_else(Keychain::empty);
    if !keychain.identities.is_empty() {
        let n = keychain.identities.len();
        println!(
            "Keychain already contains {} {}.",
            n,
            if n == 1 { "identity" } else { "identities" }
        );
        if !prompt_confirm_yn("Import Nostr keypair into this keychain? [y/N]: ")? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    let npub = push_imported_identity(&mut keychain, &nsec_input, now_iso8601())?;
    let index = keychain.identities.len().saturating_sub(1);
    set_identity_alias(&mut keychain, index, prompt_optional_alias()?)?;
    save_keychain_to(&keychain, &password, &path)?;

    println!("Nostr keypair imported: {npub}");
    Ok(())
}

/// `nc-connect id switch N` — change the active identity to index N.
pub fn cmd_id_switch(n_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let n: usize = n_str
        .parse()
        .map_err(|_| format!("invalid index: {n_str}"))?;

    let password = prompt_password("Password: ")?;
    let path = keychain_path();
    let mut keychain = require_keychain_at(&password, &path)?;

    let npub = switch_active_identity(&mut keychain, n)?;
    save_keychain_to(&keychain, &password, &path)?;
    println!("Active identity: [{n}] {npub}");
    Ok(())
}

/// `nc-connect id reset` — verify password, triple-confirm, then wipe keychain + cache.
pub fn cmd_id_reset() -> Result<(), Box<dyn std::error::Error>> {
    let path = keychain_path();
    if !keychain_exists(&path) {
        println!("No keychain found. Nothing to reset.");
        return Ok(());
    }

    // Verify the current password actually decrypts the keychain.
    let password = prompt_password("Current password: ")?;
    let _ = require_keychain_at(&password, &path)?;

    // Triple confirmation — plain stdin readline, no echo suppression needed.
    println!();
    println!("WARNING: This will permanently delete your keychain and all identities.");
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

    // Delete keychain.
    std::fs::remove_file(&path)?;

    // Delete cache if it exists (best-effort; not a fatal error if absent).
    let cp = cache_path();
    if cp.exists() {
        let _ = std::fs::remove_file(&cp);
    }

    println!("Keychain reset. Run nc-connect to create a new identity.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn prompt_keychain_password_for_write(
    path: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    if keychain_exists(path) {
        prompt_password("Password: ")
    } else {
        println!("No existing keychain found. Creating a new one.");
        prompt_new_password_with_warning()
    }
}

/// Load the keychain from the default path, returning an error if it doesn't exist.
fn require_keychain(password: &str) -> Result<Keychain, Box<dyn std::error::Error>> {
    let path = keychain_path();
    require_keychain_at(password, &path)
}

fn require_keychain_at(
    password: &str,
    path: &std::path::Path,
) -> Result<Keychain, Box<dyn std::error::Error>> {
    load_keychain_from(password, path)?
        .ok_or_else(|| "no keychain found; run `nc-connect id new` to create one".into())
}

/// Return the active identity from the keychain, or an error if none.
fn require_active(keychain: &Keychain) -> Result<&Identity, Box<dyn std::error::Error>> {
    keychain
        .active_identity()
        .ok_or_else(|| "keychain has no identities; run `nc-connect id new` to create one".into())
}
