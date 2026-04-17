use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_client::keychain::crypto::{decrypt_blob, encrypt_blob};
use nc_client::keychain::io::{parse_keychain_str, render_keychain};
use nc_client::keychain::{Keychain, active_identity_npub, now_iso8601, push_new_identity};
use rusqlite::{Connection, OptionalExtension, params};

const DB_FILENAME: &str = "helm.db";
const KEYCHAIN_KEY: &str = "active_keychain";
const RELAY_KEY: &str = "relay_url";

const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS app_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_secrets (
    key TEXT PRIMARY KEY,
    payload BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);
"#;

#[derive(Debug, Clone)]
pub struct BootSnapshot {
    pub has_keychain: bool,
    pub relay_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub keychain: Keychain,
    pub active_npub: String,
    pub active_nsec: String,
    pub active_handle: Option<String>,
}

#[derive(Debug)]
pub enum StorageCommand {
    LoadBoot {
        reply_to: Sender<Result<BootSnapshot, String>>,
    },
    SaveRelayUrl {
        relay_url: String,
        reply_to: Sender<Result<String, String>>,
    },
    CreateIdentity {
        handle: String,
        password: String,
        relay_url: String,
        reply_to: Sender<Result<StoredSession, String>>,
    },
    Unlock {
        password: String,
        reply_to: Sender<Result<StoredSession, String>>,
    },
}

#[derive(Debug)]
pub struct StorageActor {
    tx: Sender<StorageCommand>,
}

impl StorageActor {
    pub fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if let Err(err) = storage_loop(rx) {
                eprintln!("nc-helm storage actor failed: {err}");
            }
        });
        Self { tx }
    }

    pub fn load_boot(&self, reply_to: Sender<Result<BootSnapshot, String>>) {
        let _ = self.tx.send(StorageCommand::LoadBoot { reply_to });
    }

    pub fn create_identity(
        &self,
        handle: String,
        password: String,
        relay_url: String,
        reply_to: Sender<Result<StoredSession, String>>,
    ) {
        let _ = self.tx.send(StorageCommand::CreateIdentity {
            handle,
            password,
            relay_url,
            reply_to,
        });
    }

    pub fn save_relay_url(&self, relay_url: String, reply_to: Sender<Result<String, String>>) {
        let _ = self.tx.send(StorageCommand::SaveRelayUrl {
            relay_url,
            reply_to,
        });
    }

    pub fn unlock(&self, password: String, reply_to: Sender<Result<StoredSession, String>>) {
        let _ = self.tx.send(StorageCommand::Unlock { password, reply_to });
    }
}

fn storage_loop(rx: Receiver<StorageCommand>) -> Result<(), Box<dyn std::error::Error>> {
    let db = AppDatabase::open_default()?;
    while let Ok(command) = rx.recv() {
        match command {
            StorageCommand::LoadBoot { reply_to } => {
                let _ = reply_to.send(db.load_boot());
            }
            StorageCommand::SaveRelayUrl {
                relay_url,
                reply_to,
            } => {
                let _ = reply_to.send(db.save_relay_url(&relay_url));
            }
            StorageCommand::CreateIdentity {
                handle,
                password,
                relay_url,
                reply_to,
            } => {
                let _ = reply_to.send(db.create_identity(&handle, &password, &relay_url));
            }
            StorageCommand::Unlock { password, reply_to } => {
                let _ = reply_to.send(db.unlock(&password));
            }
        }
    }
    Ok(())
}

struct AppDatabase {
    conn: Connection,
}

impl AppDatabase {
    fn open_default() -> Result<Self, Box<dyn std::error::Error>> {
        Self::open(&db_path())
    }

    fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(INIT_SQL)?;
        Ok(Self { conn })
    }

    fn load_boot(&self) -> Result<BootSnapshot, String> {
        let relay_url = self
            .conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = ?1",
                params![RELAY_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|err| err.to_string())?;
        let has_keychain = self
            .conn
            .query_row(
                "SELECT 1 FROM app_secrets WHERE key = ?1",
                params![KEYCHAIN_KEY],
                |_row| Ok(true),
            )
            .optional()
            .map_err(|err| err.to_string())?
            .unwrap_or(false);
        Ok(BootSnapshot {
            has_keychain,
            relay_url,
        })
    }

    fn create_identity(
        &self,
        handle: &str,
        password: &str,
        relay_url: &str,
    ) -> Result<StoredSession, String> {
        let mut keychain = Keychain::empty();
        push_new_identity(&mut keychain, now_iso8601(), Some(handle.to_string()))
            .map_err(|err| err.to_string())?;
        let rendered = render_keychain(&keychain);
        let payload = encrypt_blob(&rendered, password).map_err(|err| err.to_string())?;
        self.conn
            .execute(
                "INSERT INTO app_secrets (key, payload, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
                params![KEYCHAIN_KEY, payload, now_unix_seconds()],
            )
            .map_err(|err| err.to_string())?;
        self.conn
            .execute(
                "INSERT INTO app_meta (key, value)
                 VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![RELAY_KEY, relay_url],
            )
            .map_err(|err| err.to_string())?;
        build_stored_session(keychain)
    }

    fn save_relay_url(&self, relay_url: &str) -> Result<String, String> {
        self.conn
            .execute(
                "INSERT INTO app_meta (key, value)
                 VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![RELAY_KEY, relay_url],
            )
            .map_err(|err| err.to_string())?;
        Ok(relay_url.to_string())
    }

    fn unlock(&self, password: &str) -> Result<StoredSession, String> {
        let payload = self
            .conn
            .query_row(
                "SELECT payload FROM app_secrets WHERE key = ?1",
                params![KEYCHAIN_KEY],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|err| err.to_string())?
            .ok_or_else(|| "No local keychain found.".to_string())?;
        let plaintext = decrypt_blob(&payload, password).map_err(|err| err.to_string())?;
        let keychain = parse_keychain_str(&plaintext).map_err(|err| err.to_string())?;
        build_stored_session(keychain)
    }
}

fn build_stored_session(keychain: Keychain) -> Result<StoredSession, String> {
    let active_identity = keychain
        .active_identity()
        .ok_or_else(|| "Keychain has no active identity.".to_string())?;
    let active_npub = active_identity_npub(&keychain).map_err(|err| err.to_string())?;
    Ok(StoredSession {
        active_handle: active_identity.handle.clone(),
        active_nsec: active_identity.nsec.clone(),
        active_npub,
        keychain,
    })
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn data_root() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|home| home.join(".local").join("share"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    base.join("nc")
}

fn db_path() -> PathBuf {
    data_root().join(DB_FILENAME)
}

#[cfg(test)]
mod tests {
    use super::AppDatabase;

    #[test]
    fn sqlite_keychain_round_trip_unlocks() {
        let path = std::env::temp_dir().join(format!(
            "nc-helm-storage-test-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        let db = AppDatabase::open(&path).expect("open app db");
        let created = db
            .create_identity("captain", "hunter2", "ws://127.0.0.1:8080")
            .expect("create identity");
        let unlocked = db.unlock("hunter2").expect("unlock keychain");
        assert_eq!(created.active_npub, unlocked.active_npub);
        assert_eq!(unlocked.active_handle.as_deref(), Some("captain"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn relay_url_round_trip_persists_in_meta_table() {
        let path = std::env::temp_dir().join(format!(
            "nc-helm-storage-relay-test-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        let db = AppDatabase::open(&path).expect("open app db");
        db.save_relay_url("ws://relay.example").expect("save relay");
        let snapshot = db.load_boot().expect("load boot snapshot");
        assert_eq!(snapshot.relay_url.as_deref(), Some("ws://relay.example"));
        let _ = std::fs::remove_file(path);
    }
}
