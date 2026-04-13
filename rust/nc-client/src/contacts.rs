use std::collections::BTreeMap;

use nc_nostr::pubkeys::hex_to_npub;
use nostr_sdk::{PublicKey, ToBech32};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedContact {
    pub npub: String,
    pub label: String,
    pub nip05: Option<String>,
}

pub fn resolve_contact_input(input: &str) -> Result<ResolvedContact, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("contact cannot be empty".to_string());
    }
    if trimmed.contains('@') {
        return resolve_nip05_contact(trimmed);
    }
    let public_key = PublicKey::parse(trimmed)
        .map_err(|_| "contact must be a valid npub or NIP-05 address".to_string())?;
    let npub = public_key
        .to_bech32()
        .map_err(|err| format!("failed to format npub: {err}"))?;
    Ok(ResolvedContact {
        label: short_contact_label(&npub),
        npub,
        nip05: None,
    })
}

fn resolve_nip05_contact(input: &str) -> Result<ResolvedContact, String> {
    let (name, domain) = input
        .split_once('@')
        .ok_or_else(|| "NIP-05 address must look like name@example.com".to_string())?;
    if name.trim().is_empty() || domain.trim().is_empty() {
        return Err("NIP-05 address must look like name@example.com".to_string());
    }
    let url = format!(
        "https://{}/.well-known/nostr.json?name={}",
        domain.trim(),
        name.trim()
    );
    let response = reqwest::blocking::get(&url)
        .map_err(|err| format!("failed to resolve {input}: {err}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "failed to resolve {input}: HTTP {}",
            response.status()
        ));
    }
    let payload: Nip05WellKnown = response
        .json()
        .map_err(|err| format!("invalid NIP-05 response for {input}: {err}"))?;
    let pubkey_hex = payload
        .names
        .get(name.trim())
        .cloned()
        .ok_or_else(|| format!("NIP-05 response for {input} did not contain that name"))?;
    let npub = hex_to_npub(&pubkey_hex)
        .ok_or_else(|| format!("NIP-05 response for {input} returned an invalid pubkey"))?;
    Ok(ResolvedContact {
        npub,
        label: name.trim().to_string(),
        nip05: Some(input.trim().to_string()),
    })
}

pub fn short_contact_label(npub: &str) -> String {
    if npub.chars().count() <= 16 {
        return npub.to_string();
    }
    let chars = npub.chars().collect::<Vec<_>>();
    format!(
        "{}…{}",
        chars[..8].iter().collect::<String>(),
        chars[chars.len().saturating_sub(6)..].iter().collect::<String>()
    )
}

#[derive(Debug, Deserialize)]
struct Nip05WellKnown {
    #[serde(default)]
    names: BTreeMap<String, String>,
}
