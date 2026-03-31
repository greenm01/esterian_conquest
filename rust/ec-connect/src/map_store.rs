use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sha2::{Digest, Sha256};
use url::Url;

use crate::connect::map_fetch::MapBundlePayload;

pub fn default_maps_root() -> PathBuf {
    let base = dirs::document_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join("Documents"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join("ec").join("maps")
}

pub fn resolve_maps_root(config_override: Option<&Path>, cli_override: Option<&Path>) -> PathBuf {
    cli_override
        .map(PathBuf::from)
        .or_else(|| config_override.map(PathBuf::from))
        .unwrap_or_else(default_maps_root)
}

pub fn map_bundle_dir(
    maps_root: &Path,
    relay_url: &str,
    game_id: &str,
) -> PathBuf {
    maps_root
        .join(relay_bucket_name(relay_url))
        .join(sanitize_component(game_id))
}

pub fn save_map_bundle(
    bundle: &MapBundlePayload,
    relay_url: &str,
    maps_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let final_dir = map_bundle_dir(maps_root, relay_url, &bundle.game_id);
    let parent = final_dir
        .parent()
        .ok_or("map bundle directory has no parent")?;
    fs::create_dir_all(parent)?;

    let staging_dir = parent.join(format!(
        ".{}.tmp-{}-{}",
        sanitize_component(&bundle.game_id),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir)?;
    }
    fs::create_dir_all(&staging_dir)?;

    for file in &bundle.files {
        if file.codec != "zstd+base64" {
            return Err(format!("unsupported map codec: {}", file.codec).into());
        }

        let compressed = BASE64.decode(file.content.as_bytes())?;
        let bytes = zstd::stream::decode_all(std::io::Cursor::new(compressed))?;
        let digest = sha256_hex(&bytes);
        if digest != file.sha256 {
            return Err(format!("sha256 mismatch for {}", file.name).into());
        }

        let path = staging_dir.join(&file.name);
        let mut out = fs::File::create(&path)?;
        out.write_all(&bytes)?;
    }

    if final_dir.exists() {
        fs::remove_dir_all(&final_dir)?;
    }
    fs::rename(&staging_dir, &final_dir)?;
    Ok(final_dir)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn relay_bucket_name(relay_url: &str) -> String {
    let Ok(url) = Url::parse(relay_url) else {
        return sanitize_component(relay_url);
    };

    let host = url
        .host_str()
        .map(sanitize_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let Some(port) = url.port() else {
        return host;
    };
    if Some(port) == default_port_for_scheme(url.scheme()) {
        host
    } else {
        format!("{host}_{port}")
    }
}

fn default_port_for_scheme(scheme: &str) -> Option<u16> {
    match scheme {
        "wss" | "https" => Some(443),
        "ws" | "http" => Some(80),
        _ => None,
    }
}

fn sanitize_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}
