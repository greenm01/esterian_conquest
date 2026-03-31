use std::path::{Path, PathBuf};

use nostr_sdk::Client;

use crate::config::{GateConfig, config_path, load_config};
use crate::identity::{identity_path, load_identity};
use crate::serve::{catalog, game_def};

pub struct PublishedGameDefinitionReceipt {
    pub relay_url: String,
    pub game_id: String,
    pub event_id: String,
}

pub fn publish_game_definition_for_dir(
    dir: &Path,
    config_path_override: Option<PathBuf>,
    identity_path_override: Option<PathBuf>,
) -> Result<PublishedGameDefinitionReceipt, Box<dyn std::error::Error>> {
    let config_path = config_path_override.unwrap_or_else(config_path);
    let identity_path = identity_path_override.unwrap_or_else(identity_path);
    let config = load_config(&config_path)
        .map_err(|err| format!("cannot load config at {}: {err}", config_path.display()))?;
    let identity = load_identity(&identity_path).map_err(|err| {
        format!(
            "cannot load daemon identity at {}: {err}",
            identity_path.display()
        )
    })?;

    let dir = dir.to_path_buf();
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move { publish_game_definition_for_dir_async(&config, &identity.keys, dir).await })
}

async fn publish_game_definition_for_dir_async(
    config: &GateConfig,
    keys: &nostr_sdk::Keys,
    dir: PathBuf,
) -> Result<PublishedGameDefinitionReceipt, Box<dyn std::error::Error>> {
    let games = catalog::load_hosted_games(std::slice::from_ref(&dir))?;
    let Some(entry) = games.into_iter().next() else {
        return Err(format!("no hosted game found in {}", dir.display()).into());
    };

    let client = Client::new(keys.clone());
    client
        .add_relay(config.relay.as_str())
        .await
        .map_err(|err| format!("could not add relay {}: {err}", config.relay))?;
    client.connect().await;

    let publish_result = game_def::publish_game_definition(
        &client,
        keys,
        &entry.game,
        &config.ssh_host,
        config.ssh_port,
    )
    .await;

    client.disconnect().await;

    let event_id = publish_result.map_err(|err| format!("could not publish 30500: {err}"))?;
    Ok(PublishedGameDefinitionReceipt {
        relay_url: config.relay.clone(),
        game_id: entry.game.game_id,
        event_id,
    })
}
