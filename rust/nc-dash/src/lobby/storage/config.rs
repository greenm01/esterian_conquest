use kdl::KdlDocument;

use super::paths;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LobbyConfigRecord {
    pub relay: Option<String>,
}

pub fn config_path() -> std::path::PathBuf {
    paths::config_path()
}

pub fn parse_config_kdl(raw: &str) -> Result<LobbyConfigRecord, Box<dyn std::error::Error>> {
    let doc: KdlDocument = raw.parse()?;
    let relay = doc
        .nodes()
        .iter()
        .find(|node| node.name().value() == "config")
        .and_then(|node| node.get("relay"))
        .and_then(|value| value.as_string())
        .map(ToString::to_string);
    Ok(LobbyConfigRecord { relay })
}

pub fn render_config_kdl(record: &LobbyConfigRecord) -> String {
    match record.relay.as_deref() {
        Some(relay) => format!("config relay=\"{relay}\"\n"),
        None => "config\n".to_string(),
    }
}
