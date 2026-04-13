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
    pub host_contact_npub: Option<String>,
    pub host_contact_label: Option<String>,
    pub host_contact_nip05: Option<String>,
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
pub struct NoticeEntry {
    pub notice_id: String,
    pub sender_npub: String,
    pub sender_handle: Option<String>,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactEntry {
    pub npub: String,
    pub label: String,
    pub nip05: Option<String>,
    pub source: String,
    pub blocked: bool,
    pub hidden: bool,
    pub unread_count: u32,
    pub last_activity_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactMessageEntry {
    pub message_id: String,
    pub contact_npub: String,
    pub sender_npub: String,
    pub sender_label: Option<String>,
    pub body: String,
    pub outgoing: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameInboxMessageEntry {
    pub message_id: String,
    pub game_id: String,
    pub other_empire_id: u8,
    pub other_empire_name: String,
    pub sender_empire_id: u8,
    pub sender_empire_name: String,
    pub body: String,
    pub outgoing: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameRosterEntry {
    pub game_id: String,
    pub empire_id: u8,
    pub empire_name: String,
    pub is_self: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientCache {
    pub games: Vec<CachedGame>,
    pub notices: Vec<NoticeEntry>,
    pub direct_contacts: Vec<ContactEntry>,
    pub direct_messages: Vec<ContactMessageEntry>,
    pub game_inbox_messages: Vec<GameInboxMessageEntry>,
    pub rosters: Vec<GameRosterEntry>,
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

    pub fn upsert_contact(&mut self, entry: ContactEntry) {
        if let Some(existing) = self
            .direct_contacts
            .iter_mut()
            .find(|existing| existing.npub == entry.npub)
        {
            let blocked = existing.blocked;
            let hidden = existing.hidden;
            let unread_count = existing.unread_count;
            let last_activity_at = existing.last_activity_at.clone();
            *existing = ContactEntry {
                blocked,
                hidden,
                unread_count,
                last_activity_at,
                ..entry
            };
        } else {
            self.direct_contacts.push(entry);
        }
        self.direct_contacts.sort_by(|left, right| {
            left.label
                .to_lowercase()
                .cmp(&right.label.to_lowercase())
                .then_with(|| left.npub.cmp(&right.npub))
        });
    }

    pub fn upsert_contact_message(&mut self, entry: ContactMessageEntry) {
        if let Some(existing) = self
            .direct_messages
            .iter_mut()
            .find(|existing| existing.message_id == entry.message_id)
        {
            *existing = entry;
        } else {
            self.direct_messages.push(entry);
        }
        self.direct_messages
            .sort_by(|left, right| left.created_at.cmp(&right.created_at));
    }

    pub fn upsert_game_inbox_message(&mut self, entry: GameInboxMessageEntry) {
        if let Some(existing) = self
            .game_inbox_messages
            .iter_mut()
            .find(|existing| existing.message_id == entry.message_id)
        {
            *existing = entry;
        } else {
            self.game_inbox_messages.push(entry);
        }
        self.game_inbox_messages
            .sort_by(|left, right| left.created_at.cmp(&right.created_at));
    }

    pub fn replace_roster(&mut self, game_id: &str, entries: Vec<GameRosterEntry>) {
        self.rosters.retain(|entry| entry.game_id != game_id);
        self.rosters.extend(entries);
        self.rosters.sort_by(|left, right| {
            left.game_id
                .cmp(&right.game_id)
                .then_with(|| left.empire_id.cmp(&right.empire_id))
        });
    }

    pub fn note_contact_activity(
        &mut self,
        npub: &str,
        created_at: &str,
        incoming_unread_delta: u32,
    ) {
        let Some(contact) = self
            .direct_contacts
            .iter_mut()
            .find(|contact| contact.npub == npub)
        else {
            return;
        };
        contact.last_activity_at = Some(created_at.to_string());
        if !contact.blocked {
            if incoming_unread_delta != 0 {
                contact.hidden = false;
            }
            contact.unread_count = contact.unread_count.saturating_add(incoming_unread_delta);
        }
    }

    pub fn mark_contact_read(&mut self, npub: &str) {
        if let Some(contact) = self
            .direct_contacts
            .iter_mut()
            .find(|contact| contact.npub == npub)
        {
            contact.unread_count = 0;
        }
    }

    pub fn set_contact_blocked(&mut self, npub: &str, blocked: bool) {
        if let Some(contact) = self
            .direct_contacts
            .iter_mut()
            .find(|contact| contact.npub == npub)
        {
            contact.blocked = blocked;
            if blocked {
                contact.unread_count = 0;
            }
        }
    }

    pub fn set_contact_hidden(&mut self, npub: &str, hidden: bool) {
        if let Some(contact) = self
            .direct_contacts
            .iter_mut()
            .find(|contact| contact.npub == npub)
        {
            contact.hidden = hidden;
            if hidden {
                contact.unread_count = 0;
            }
        }
    }
}

pub fn cache_path() -> PathBuf {
    data_root().join("cache.kdl")
}

pub fn load_cache(password: &str) -> Result<Option<ClientCache>, Box<dyn std::error::Error>> {
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

pub fn save_cache(cache: &ClientCache, password: &str) -> Result<(), Box<dyn std::error::Error>> {
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
                    host_contact_npub: opt_string(node, "host-contact-npub"),
                    host_contact_label: opt_string(node, "host-contact-label"),
                    host_contact_nip05: opt_string(node, "host-contact-nip05"),
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
            "notice" => {
                cache.notices.push(NoticeEntry {
                    notice_id: req_string(node, "id", "notice")?,
                    sender_npub: req_string(node, "sender-npub", "notice")?,
                    sender_handle: opt_string(node, "sender-handle"),
                    body: req_string(node, "body", "notice")?,
                    created_at: req_string(node, "created-at", "notice")?,
                });
            }
            "contact" => {
                cache.direct_contacts.push(ContactEntry {
                    npub: req_string(node, "npub", "contact")?,
                    label: req_string(node, "label", "contact")?,
                    nip05: opt_string(node, "nip05"),
                    source: req_string(node, "source", "contact")?,
                    blocked: opt_integer(node, "blocked").unwrap_or(0) != 0,
                    hidden: opt_integer(node, "hidden").unwrap_or(0) != 0,
                    unread_count: opt_integer(node, "unread-count")
                        .and_then(|value| u32::try_from(value).ok())
                        .unwrap_or(0),
                    last_activity_at: opt_string(node, "last-activity-at"),
                });
            }
            "contact-message" => {
                cache.direct_messages.push(ContactMessageEntry {
                    message_id: req_string(node, "id", "contact-message")?,
                    contact_npub: req_string(node, "contact-npub", "contact-message")?,
                    sender_npub: req_string(node, "sender-npub", "contact-message")?,
                    sender_label: opt_string(node, "sender-label"),
                    body: req_string(node, "body", "contact-message")?,
                    outgoing: opt_integer(node, "outgoing").unwrap_or(0) != 0,
                    created_at: req_string(node, "created-at", "contact-message")?,
                });
            }
            "game-mail" => {
                cache.game_inbox_messages.push(GameInboxMessageEntry {
                    message_id: req_string(node, "id", "game-mail")?,
                    game_id: req_string(node, "game-id", "game-mail")?,
                    other_empire_id: req_integer(node, "other-empire-id", "game-mail")? as u8,
                    other_empire_name: req_string(node, "other-empire-name", "game-mail")?,
                    sender_empire_id: req_integer(node, "sender-empire-id", "game-mail")? as u8,
                    sender_empire_name: req_string(node, "sender-empire-name", "game-mail")?,
                    body: req_string(node, "body", "game-mail")?,
                    outgoing: opt_integer(node, "outgoing").unwrap_or(0) != 0,
                    created_at: req_string(node, "created-at", "game-mail")?,
                });
            }
            "roster" => {
                cache.rosters.push(GameRosterEntry {
                    game_id: req_string(node, "game-id", "roster")?,
                    empire_id: req_integer(node, "empire-id", "roster")? as u8,
                    empire_name: req_string(node, "empire-name", "roster")?,
                    is_self: opt_integer(node, "is-self").unwrap_or(0) != 0,
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
        if let Some(host_contact_npub) = game.host_contact_npub.as_deref() {
            out.push_str(&format!(
                " host-contact-npub=\"{}\"",
                escape(host_contact_npub)
            ));
        }
        if let Some(host_contact_label) = game.host_contact_label.as_deref() {
            out.push_str(&format!(
                " host-contact-label=\"{}\"",
                escape(host_contact_label)
            ));
        }
        if let Some(host_contact_nip05) = game.host_contact_nip05.as_deref() {
            out.push_str(&format!(
                " host-contact-nip05=\"{}\"",
                escape(host_contact_nip05)
            ));
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
    for contact in &cache.direct_contacts {
        out.push_str(&format!(
            "contact npub=\"{}\" label=\"{}\" source=\"{}\"",
            escape(&contact.npub),
            escape(&contact.label),
            escape(&contact.source),
        ));
        if let Some(nip05) = contact.nip05.as_deref() {
            out.push_str(&format!(" nip05=\"{}\"", escape(nip05)));
        }
        if contact.blocked {
            out.push_str(" blocked=1");
        }
        if contact.hidden {
            out.push_str(" hidden=1");
        }
        if contact.unread_count != 0 {
            out.push_str(&format!(" unread-count={}", contact.unread_count));
        }
        if let Some(last_activity_at) = contact.last_activity_at.as_deref() {
            out.push_str(&format!(
                " last-activity-at=\"{}\"",
                escape(last_activity_at)
            ));
        }
        out.push('\n');
    }
    for entry in &cache.direct_messages {
        out.push_str(&format!(
            "contact-message id=\"{}\" contact-npub=\"{}\" sender-npub=\"{}\" body=\"{}\" outgoing={} created-at=\"{}\"",
            escape(&entry.message_id),
            escape(&entry.contact_npub),
            escape(&entry.sender_npub),
            escape(&entry.body),
            if entry.outgoing { 1 } else { 0 },
            escape(&entry.created_at),
        ));
        if let Some(sender_label) = entry.sender_label.as_deref() {
            out.push_str(&format!(" sender-label=\"{}\"", escape(sender_label)));
        }
        out.push('\n');
    }
    for entry in &cache.game_inbox_messages {
        out.push_str(&format!(
            "game-mail id=\"{}\" game-id=\"{}\" other-empire-id={} other-empire-name=\"{}\" sender-empire-id={} sender-empire-name=\"{}\" body=\"{}\" outgoing={} created-at=\"{}\"",
            escape(&entry.message_id),
            escape(&entry.game_id),
            entry.other_empire_id,
            escape(&entry.other_empire_name),
            entry.sender_empire_id,
            escape(&entry.sender_empire_name),
            escape(&entry.body),
            if entry.outgoing { 1 } else { 0 },
            escape(&entry.created_at),
        ));
        out.push('\n');
    }
    for entry in &cache.rosters {
        out.push_str(&format!(
            "roster game-id=\"{}\" empire-id={} empire-name=\"{}\" is-self={}\n",
            escape(&entry.game_id),
            entry.empire_id,
            escape(&entry.empire_name),
            if entry.is_self { 1 } else { 0 }
        ));
    }
    out
}

fn req_string(
    node: &kdl::KdlNode,
    key: &str,
    node_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    opt_string(node, key).ok_or_else(|| format!("missing {key} on {node_name} node").into())
}

fn req_integer(
    node: &kdl::KdlNode,
    key: &str,
    node_name: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    opt_integer(node, key).ok_or_else(|| format!("missing {key} on {node_name} node").into())
}

fn opt_string(node: &kdl::KdlNode, key: &str) -> Option<String> {
    node.get(key)
        .and_then(kdl::KdlValue::as_string)
        .map(str::to_string)
}

fn opt_integer(node: &kdl::KdlNode, key: &str) -> Option<i64> {
    node.get(key)
        .and_then(kdl::KdlValue::as_integer)
        .and_then(|value| i64::try_from(value).ok())
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
