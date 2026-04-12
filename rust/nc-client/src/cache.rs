use std::fs;
use std::io::Write;
use std::path::PathBuf;

use kdl::KdlDocument;

use crate::keychain::crypto::{decrypt_blob, encrypt_blob};
use crate::paths::data_root;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedGame {
    pub id: String,
    pub name: String,
    pub host_alias: Option<String>,
    pub relay_url: String,
    pub daemon_pubkey: String,
    pub seat: Option<u32>,
    pub status: String,
    pub invite_address: Option<String>,
    pub last_turn: Option<u32>,
    pub last_hash: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxEntry {
    pub kind: String,
    pub request_id: Option<String>,
    pub game_id: String,
    pub game_name: Option<String>,
    pub status: String,
    pub message: String,
    pub invite_address: Option<String>,
    pub turn: Option<u32>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeEntry {
    pub notice_id: String,
    pub sender_npub: String,
    pub sender_handle: Option<String>,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadEntry {
    pub message_id: String,
    pub game_id: String,
    pub sender_role: String,
    pub sender_npub: String,
    pub sender_handle: Option<String>,
    pub body: String,
    pub outgoing: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientCache {
    pub games: Vec<CachedGame>,
    pub inbox: Vec<InboxEntry>,
    pub notices: Vec<NoticeEntry>,
    pub threads: Vec<ThreadEntry>,
}

impl ClientCache {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn upsert_game(&mut self, game: CachedGame) {
        if let Some(existing) = self.games.iter_mut().find(|existing| existing.id == game.id) {
            *existing = game;
        } else {
            self.games.push(game);
        }
    }

    pub fn upsert_inbox(&mut self, entry: InboxEntry) {
        if let Some(existing) = self.inbox.iter_mut().find(|existing| {
            existing.kind == entry.kind
                && existing.request_id == entry.request_id
                && existing.game_id == entry.game_id
        }) {
            *existing = entry;
        } else {
            self.inbox.push(entry);
        }
    }

    pub fn upsert_notice(&mut self, entry: NoticeEntry) {
        if let Some(existing) = self
            .notices
            .iter_mut()
            .find(|existing| existing.notice_id == entry.notice_id)
        {
            *existing = entry;
        } else {
            self.notices.push(entry);
        }
        self.notices
            .sort_by(|left, right| left.created_at.cmp(&right.created_at));
    }

    pub fn upsert_thread(&mut self, entry: ThreadEntry) {
        if let Some(existing) = self
            .threads
            .iter_mut()
            .find(|existing| existing.message_id == entry.message_id)
        {
            *existing = entry;
        } else {
            self.threads.push(entry);
        }
        self.threads
            .sort_by(|left, right| left.created_at.cmp(&right.created_at));
    }
}

pub fn cache_path() -> PathBuf {
    data_root().join("cache.kdl")
}

pub fn load_cache(
    password: &str,
) -> Result<Option<ClientCache>, Box<dyn std::error::Error>> {
    load_cache_from(password, &cache_path())
}

pub fn load_cache_from(
    password: &str,
    path: &std::path::Path,
) -> Result<Option<ClientCache>, Box<dyn std::error::Error>> {
    match fs::read(path) {
        Ok(blob) => {
            let kdl_str = decrypt_blob(&blob, password)?;
            Ok(Some(parse_cache_str(&kdl_str)?))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn save_cache(
    cache: &ClientCache,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    save_cache_to(cache, password, &cache_path())
}

pub fn save_cache_to(
    cache: &ClientCache,
    password: &str,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = render_cache(cache);
    let blob = encrypt_blob(&text, password)?;
    let tmp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&blob)?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn parse_cache_str(kdl: &str) -> Result<ClientCache, Box<dyn std::error::Error>> {
    let doc: KdlDocument = kdl.parse()?;
    let mut cache = ClientCache::empty();

    for node in doc.nodes() {
        match node.name().value() {
            "game" => {
                cache.games.push(CachedGame {
                    id: req_string(node, "id", "game")?,
                    name: req_string(node, "name", "game")?,
                    host_alias: opt_string(node, "host-alias"),
                    relay_url: req_string(node, "relay-url", "game")?,
                    daemon_pubkey: req_string(node, "daemon-pubkey", "game")?,
                    seat: opt_integer(node, "seat").map(|value| value as u32),
                    status: req_string(node, "status", "game")?,
                    invite_address: opt_string(node, "invite"),
                    last_turn: opt_integer(node, "last-turn").map(|value| value as u32),
                    last_hash: opt_string(node, "last-hash"),
                    updated_at: req_string(node, "updated-at", "game")?,
                });
            }
            "inbox" => {
                cache.inbox.push(InboxEntry {
                    kind: req_string(node, "kind", "inbox")?,
                    request_id: opt_string(node, "request-id"),
                    game_id: req_string(node, "game-id", "inbox")?,
                    game_name: opt_string(node, "game-name"),
                    status: req_string(node, "status", "inbox")?,
                    message: req_string(node, "message", "inbox")?,
                    invite_address: opt_string(node, "invite"),
                    turn: opt_integer(node, "turn").map(|value| value as u32),
                    updated_at: req_string(node, "updated-at", "inbox")?,
                });
            }
            "notice" => {
                cache.notices.push(NoticeEntry {
                    notice_id: req_string(node, "id", "notice")?,
                    sender_npub: req_string(node, "sender-npub", "notice")?,
                    sender_handle: opt_string(node, "sender-handle"),
                    body: req_string(node, "body", "notice")?,
                    created_at: req_string(node, "created-at", "notice")?,
                });
            }
            "thread" => {
                cache.threads.push(ThreadEntry {
                    message_id: req_string(node, "id", "thread")?,
                    game_id: req_string(node, "game-id", "thread")?,
                    sender_role: req_string(node, "sender-role", "thread")?,
                    sender_npub: req_string(node, "sender-npub", "thread")?,
                    sender_handle: opt_string(node, "sender-handle"),
                    body: req_string(node, "body", "thread")?,
                    outgoing: opt_integer(node, "outgoing").unwrap_or(0) != 0,
                    created_at: req_string(node, "created-at", "thread")?,
                });
            }
            _ => {}
        }
    }

    Ok(cache)
}

pub fn render_cache(cache: &ClientCache) -> String {
    let mut out = String::new();
    for game in &cache.games {
        out.push_str(&format!(
            "game id=\"{}\" name=\"{}\" relay-url=\"{}\" daemon-pubkey=\"{}\" status=\"{}\" updated-at=\"{}\"",
            escape(&game.id),
            escape(&game.name),
            escape(&game.relay_url),
            escape(&game.daemon_pubkey),
            escape(&game.status),
            escape(&game.updated_at),
        ));
        if let Some(host_alias) = game.host_alias.as_deref() {
            out.push_str(&format!(" host-alias=\"{}\"", escape(host_alias)));
        }
        if let Some(seat) = game.seat {
            out.push_str(&format!(" seat={seat}"));
        }
        if let Some(invite) = game.invite_address.as_deref() {
            out.push_str(&format!(" invite=\"{}\"", escape(invite)));
        }
        if let Some(last_turn) = game.last_turn {
            out.push_str(&format!(" last-turn={last_turn}"));
        }
        if let Some(last_hash) = game.last_hash.as_deref() {
            out.push_str(&format!(" last-hash=\"{}\"", escape(last_hash)));
        }
        out.push('\n');
    }
    for entry in &cache.inbox {
        out.push_str(&format!(
            "inbox kind=\"{}\" game-id=\"{}\" status=\"{}\" message=\"{}\" updated-at=\"{}\"",
            escape(&entry.kind),
            escape(&entry.game_id),
            escape(&entry.status),
            escape(&entry.message),
            escape(&entry.updated_at),
        ));
        if let Some(request_id) = entry.request_id.as_deref() {
            out.push_str(&format!(" request-id=\"{}\"", escape(request_id)));
        }
        if let Some(game_name) = entry.game_name.as_deref() {
            out.push_str(&format!(" game-name=\"{}\"", escape(game_name)));
        }
        if let Some(invite) = entry.invite_address.as_deref() {
            out.push_str(&format!(" invite=\"{}\"", escape(invite)));
        }
        if let Some(turn) = entry.turn {
            out.push_str(&format!(" turn={turn}"));
        }
        out.push('\n');
    }
    for notice in &cache.notices {
        out.push_str(&format!(
            "notice id=\"{}\" sender-npub=\"{}\" body=\"{}\" created-at=\"{}\"",
            escape(&notice.notice_id),
            escape(&notice.sender_npub),
            escape(&notice.body),
            escape(&notice.created_at),
        ));
        if let Some(sender_handle) = notice.sender_handle.as_deref() {
            out.push_str(&format!(" sender-handle=\"{}\"", escape(sender_handle)));
        }
        out.push('\n');
    }
    for entry in &cache.threads {
        out.push_str(&format!(
            "thread id=\"{}\" game-id=\"{}\" sender-role=\"{}\" sender-npub=\"{}\" body=\"{}\" outgoing={} created-at=\"{}\"",
            escape(&entry.message_id),
            escape(&entry.game_id),
            escape(&entry.sender_role),
            escape(&entry.sender_npub),
            escape(&entry.body),
            if entry.outgoing { 1 } else { 0 },
            escape(&entry.created_at),
        ));
        if let Some(sender_handle) = entry.sender_handle.as_deref() {
            out.push_str(&format!(" sender-handle=\"{}\"", escape(sender_handle)));
        }
        out.push('\n');
    }
    out
}

fn req_string(
    node: &kdl::KdlNode,
    key: &str,
    node_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    node.get(key)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .ok_or_else(|| format!("{node_name} node missing `{key}`").into())
}

fn opt_string(node: &kdl::KdlNode, key: &str) -> Option<String> {
    node.get(key)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn opt_integer(node: &kdl::KdlNode, key: &str) -> Option<i64> {
    node.get(key)
        .and_then(|value| value.as_integer())
        .and_then(|value| i64::try_from(value).ok())
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
