use nostr_sdk::{Event, ToBech32};
use url::{Host, Url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedGameDefinition {
    pub gate_npub: String,
    pub game_id: String,
    pub game_name: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub slots: Vec<PublishedSeatSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedSeatSlot {
    pub seat: u32,
    pub invite_code_hash: String,
    pub player_npub: Option<String>,
    pub status: String,
}

pub fn relay_url_to_invite_host(relay_url: &str) -> Result<String, String> {
    let parsed = Url::parse(relay_url)
        .map_err(|_| "relay URL must be a valid ws:// or wss:// URL".to_string())?;
    match parsed.scheme() {
        "ws" | "wss" => {}
        _ => return Err("relay URL must start with ws:// or wss://".to_string()),
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("relay URL must not include username or password".to_string());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("relay URL must not include query or fragment".to_string());
    }
    if parsed.path() != "/" && !parsed.path().is_empty() {
        return Err("relay URL must not include a path".to_string());
    }

    let host = match parsed.host() {
        Some(Host::Ipv4(host)) => host.to_string(),
        Some(Host::Ipv6(host)) => format!("[{host}]"),
        Some(Host::Domain(host)) => host.to_string(),
        None => return Err("relay URL must include a host".to_string()),
    };

    Ok(match parsed.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    })
}

pub fn invite_address_from_relay(words: &str, relay_url: &str) -> Result<String, String> {
    Ok(format!("{words}@{}", relay_url_to_invite_host(relay_url)?))
}

pub fn parse_game_definition(event: &Event) -> Option<PublishedGameDefinition> {
    let gate_npub = event.pubkey.to_bech32().ok()?;
    let mut game_id = None;
    let mut game_name = None;
    let mut ssh_host = None;
    let mut ssh_port = None;
    let mut slots = Vec::new();

    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        let Some(kind) = values.first().map(String::as_str) else {
            continue;
        };
        match kind {
            "d" if values.len() >= 2 => game_id = Some(values[1].clone()),
            "name" if values.len() >= 2 => game_name = Some(values[1].clone()),
            "ssh-host" if values.len() >= 2 => ssh_host = Some(values[1].clone()),
            "ssh-port" if values.len() >= 2 => {
                ssh_port = values[1].parse::<u16>().ok();
            }
            "slot" if values.len() >= 5 => slots.push(PublishedSeatSlot {
                seat: values[1].parse::<u32>().ok()?,
                invite_code_hash: values[2].clone(),
                player_npub: Some(values[3].clone()).filter(|value| !value.trim().is_empty()),
                status: values[4].clone(),
            }),
            _ => {}
        }
    }

    Some(PublishedGameDefinition {
        gate_npub,
        game_id: game_id?,
        game_name: game_name?,
        ssh_host: ssh_host?,
        ssh_port: ssh_port?,
        slots,
    })
}
