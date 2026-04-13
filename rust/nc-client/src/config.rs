use std::fs;
use std::io::Write;
use std::path::PathBuf;

use kdl::KdlDocument;

use crate::paths::config_root;
use crate::relay::{RelayStatus, validate_relay_url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayEntry {
    pub url: String,
    pub is_default: bool,
    pub status: RelayStatus,
    pub last_error: Option<String>,
    pub last_checked: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClientConfig {
    pub relay: Option<String>,
    pub relays: Vec<RelayEntry>,
}

impl ClientConfig {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn default_relay_url(&self) -> Option<&str> {
        self.relays
            .iter()
            .find(|relay| relay.is_default)
            .map(|relay| relay.url.as_str())
            .or(self.relay.as_deref())
    }

    pub fn upsert_relay(&mut self, url: String) -> &mut RelayEntry {
        if let Some(index) = self.relays.iter().position(|relay| relay.url == url) {
            return &mut self.relays[index];
        }
        self.relays.push(RelayEntry {
            url,
            is_default: false,
            status: RelayStatus::Unknown,
            last_error: None,
            last_checked: None,
        });
        self.relays.last_mut().expect("relay pushed")
    }

    pub fn set_default_relay(&mut self, url: &str) {
        for relay in &mut self.relays {
            relay.is_default = relay.url == url;
        }
        self.relay = Some(url.to_string());
        if self.relays.iter().all(|relay| relay.url != url) {
            let relay = self.upsert_relay(url.to_string());
            relay.is_default = true;
        }
    }

    pub fn normalize_relays(&mut self) {
        if self.relays.is_empty() {
            self.relay = self
                .relay
                .as_deref()
                .and_then(|value| validate_relay_url(value).ok().flatten());
            if let Some(url) = self.relay.clone() {
                let relay = self.upsert_relay(url.clone());
                relay.is_default = true;
            }
            return;
        }

        let mut saw_default = false;
        for relay in &mut self.relays {
            if relay.is_default {
                if saw_default {
                    relay.is_default = false;
                } else {
                    saw_default = true;
                }
            }
        }

        if !saw_default {
            if let Some(default_url) = self
                .relay
                .as_deref()
                .and_then(|value| validate_relay_url(value).ok().flatten())
            {
                self.set_default_relay(&default_url);
            } else if let Some(first_url) = self.relays.first().map(|relay| relay.url.clone()) {
                self.set_default_relay(&first_url);
            }
        } else {
            self.relay = self
                .relays
                .iter()
                .find(|relay| relay.is_default)
                .map(|relay| relay.url.clone());
        }
    }
}

pub fn config_path() -> PathBuf {
    config_root().join("config.kdl")
}

pub fn load_config() -> Result<ClientConfig, Box<dyn std::error::Error>> {
    load_config_from(&config_path())
}

pub fn load_config_from(
    path: &std::path::Path,
) -> Result<ClientConfig, Box<dyn std::error::Error>> {
    match fs::read_to_string(path) {
        Ok(text) => parse_config_str(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ClientConfig::empty()),
        Err(e) => Err(e.into()),
    }
}

pub fn save_config(config: &ClientConfig) -> Result<(), Box<dyn std::error::Error>> {
    save_config_to(config, &config_path())
}

pub fn save_config_to(
    config: &ClientConfig,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = render_config(config);
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn parse_config_str(kdl: &str) -> Result<ClientConfig, Box<dyn std::error::Error>> {
    let doc: KdlDocument = kdl.parse()?;
    let mut config = ClientConfig::empty();

    for node in doc.nodes() {
        if node.name().value() != "relay" {
            continue;
        }
        let url = node
            .get(0usize)
            .and_then(|value| value.as_string())
            .ok_or("relay node requires a string argument")?
            .to_string();
        let is_default = node
            .get("default")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let status = node
            .get("status")
            .and_then(|value| value.as_string())
            .and_then(RelayStatus::from_str)
            .unwrap_or(RelayStatus::Unknown);
        let last_error = node
            .get("last-error")
            .and_then(|value| value.as_string())
            .map(str::to_string);
        let last_checked = node
            .get("checked")
            .and_then(|value| value.as_string())
            .map(str::to_string);
        let entry = config.upsert_relay(url.clone());
        entry.is_default = is_default;
        entry.status = status;
        entry.last_error = last_error;
        entry.last_checked = last_checked;
        if is_default {
            config.relay = Some(url);
        }
    }

    config.normalize_relays();
    Ok(config)
}

pub fn render_config(config: &ClientConfig) -> String {
    let mut out = String::new();
    for relay in &config.relays {
        out.push_str(&format!("relay \"{}\"", escape(&relay.url)));
        if relay.is_default {
            out.push_str(" default=#true");
        }
        if relay.status != RelayStatus::Unknown {
            out.push_str(&format!(" status=\"{}\"", relay.status.as_str()));
        }
        if let Some(last_error) = relay
            .last_error
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            out.push_str(&format!(" last-error=\"{}\"", escape(last_error)));
        }
        if let Some(last_checked) = relay
            .last_checked
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            out.push_str(&format!(" checked=\"{}\"", escape(last_checked)));
        }
        out.push('\n');
    }
    out
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
