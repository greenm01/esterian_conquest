use std::fs;
use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use nc_connect::connect::map_fetch::{MapBundlePayload, MapFilePayload};
use nc_connect::map_store::{
    default_maps_root, map_bundle_dir, resolve_maps_root, save_map_bundle,
};
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn tmp_maps_root(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("nc_connect_maps_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp maps root should exist");
    path
}

fn encoded_file(name: &str, contents: &str) -> MapFilePayload {
    let bytes = contents.as_bytes();
    let compressed =
        zstd::stream::encode_all(std::io::Cursor::new(bytes), 0).expect("compress should work");
    MapFilePayload {
        name: name.to_string(),
        codec: "zstd+base64".to_string(),
        sha256: sha256_hex(bytes),
        content: BASE64.encode(compressed),
    }
}

#[test]
fn default_maps_root_ends_with_documents_ec_maps() {
    let root = default_maps_root();
    let rendered = root.to_string_lossy();
    assert!(rendered.ends_with("Documents/nc/maps") || rendered.ends_with("Documents\\nc\\maps"));
}

#[test]
fn resolve_maps_root_prefers_cli_over_config() {
    let config = PathBuf::from("/tmp/config-maps");
    let cli = PathBuf::from("/tmp/cli-maps");
    assert_eq!(
        resolve_maps_root(Some(config.as_path()), Some(cli.as_path())),
        cli
    );
}

#[test]
fn map_bundle_dir_uses_relay_host_and_game_id() {
    let root = PathBuf::from("/tmp/maps");
    let dir = map_bundle_dir(&root, "wss://relay.example.com", "friday-night");
    assert_eq!(
        dir,
        PathBuf::from("/tmp/maps/relay.example.com/friday-night")
    );
}

#[test]
fn map_bundle_dir_keeps_custom_relay_port_suffix() {
    let root = PathBuf::from("/tmp/maps");
    let dir = map_bundle_dir(&root, "wss://relay.example.com:7447", "friday-night");
    assert_eq!(
        dir,
        PathBuf::from("/tmp/maps/relay.example.com_7447/friday-night")
    );
}

#[test]
fn save_map_bundle_decodes_and_writes_all_files() {
    let root = tmp_maps_root("write");
    let bundle = MapBundlePayload {
        game_id: "friday-night".into(),
        game_name: "Friday Night EC".into(),
        seat: 2,
        files: vec![
            encoded_file("starmap.txt", "STAR MAP"),
            encoded_file("starmap.csv", "x,y\n1,2"),
            encoded_file("starmap-DETAILS.csv", "x,y,known_name\n1,2,Foosville"),
        ],
    };

    let saved =
        save_map_bundle(&bundle, "wss://relay.example.com", &root).expect("bundle should save");
    assert_eq!(saved, root.join("relay.example.com").join("friday-night"));
    assert_eq!(
        fs::read_to_string(saved.join("starmap.txt")).unwrap(),
        "STAR MAP"
    );
    assert_eq!(
        fs::read_to_string(saved.join("starmap.csv")).unwrap(),
        "x,y\n1,2"
    );
    assert!(
        fs::read_to_string(saved.join("starmap-DETAILS.csv"))
            .unwrap()
            .contains("Foosville")
    );

    let _ = fs::remove_dir_all(&root);
}
