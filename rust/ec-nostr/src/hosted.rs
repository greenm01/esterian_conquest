use url::{Host, Url};

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
