//! Local game cache types and I/O.
//!
//! The cache file lives at `~/.local/share/ec/cache.kdl` and holds one
//! entry per game the player has joined.  It is a plain (unencrypted) KDL
//! file; no secret data is stored here.
//!
//! Format:
//! ```kdl
//! game id="friday-night" name="Friday Night NC" player-name="House Vale" server="play.example.com" port=22 relay-url="wss://relay.example.com" seat=2 npub="npub1aaa..." gate-npub="npub1gate..." status="joined" joined="2026-03-26T12:00:00Z" last-connected="2026-03-28T19:30:00Z"
//! game id="saturday-showdown" name="Saturday Showdown" server="war.example.com" port=22 seat=5 npub="npub1aaa..." status="pending" invite-code="velvet-mountain" joined="2026-03-27T10:00:00Z"
//! ```
//!
//! The picker sorts games by `last-connected` descending (most recently
//! played first).  Games with no `last-connected` timestamp appear last,
//! ordered by `joined`.

pub mod io;

pub use io::{cache_path, load_cache, load_cache_from, save_cache, save_cache_to};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedGameStatus {
    Pending,
    Joined,
}

impl CachedGameStatus {
    pub fn as_kdl_value(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Joined => "joined",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "pending" => Self::Pending,
            _ => Self::Joined,
        }
    }
}

/// One joined-game entry in the local cache.
#[derive(Debug, Clone)]
pub struct CachedGame {
    /// Game identifier slug (matches the server's roster game ID).
    pub id: String,
    /// Human-readable game name.
    pub name: String,
    /// Server-reported empire name for this player in this game.
    pub player_name: Option<String>,
    /// Server hostname.
    pub server: String,
    /// SSH port.
    pub port: u16,
    /// Relay URL used for successful handshakes to this game, when known.
    pub relay_url: Option<String>,
    /// Player seat number (1-based).
    pub seat: u32,
    /// The identity (npub) that joined this game.
    pub npub: String,
    /// The gate's Nostr public key (bech32).  Empty string if not known
    /// (e.g. entries written by older versions of ec-connect).
    pub gate_npub: String,
    /// Local picker state for this game row.
    pub status: CachedGameStatus,
    /// Original invite code for incomplete hosted first joins.
    pub invite_code: Option<String>,
    /// ISO-8601 timestamp of first join.
    pub joined: String,
    /// ISO-8601 timestamp of most recent connection, if any.
    pub last_connected: Option<String>,
}

/// The local game cache: a list of joined games.
#[derive(Debug, Clone)]
pub struct GameCache {
    pub games: Vec<CachedGame>,
}

impl GameCache {
    /// Create an empty cache.
    pub fn empty() -> Self {
        GameCache { games: Vec::new() }
    }

    /// Upsert a game entry.  If a game with the same `id` already exists,
    /// it is replaced; otherwise the entry is appended.
    pub fn upsert(&mut self, game: CachedGame) {
        if let Some(pos) = self.games.iter().position(|g| g.id == game.id) {
            self.games[pos] = game;
        } else {
            self.games.push(game);
        }
    }

    /// Update the `last-connected` timestamp for the game with `id`.
    /// Does nothing if the game is not in the cache.
    pub fn touch(&mut self, id: &str, timestamp: &str) {
        if let Some(g) = self.games.iter_mut().find(|g| g.id == id) {
            g.last_connected = Some(timestamp.to_string());
        }
    }

    /// Merge refreshed game metadata into an existing cache row.
    ///
    /// Returns `true` when a matching row was found and updated.
    pub fn update_metadata(
        &mut self,
        id: &str,
        name: &str,
        player_name: Option<&str>,
        seat: u32,
    ) -> bool {
        let Some(game) = self.games.iter_mut().find(|g| g.id == id) else {
            return false;
        };
        game.name = name.to_string();
        game.player_name = player_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        game.seat = seat;
        game.status = CachedGameStatus::Joined;
        game.invite_code = None;
        true
    }

    /// Remove the cached game with the given `id`.
    ///
    /// Returns `true` when an entry was removed.
    pub fn remove(&mut self, id: &str) -> bool {
        if let Some(pos) = self.games.iter().position(|g| g.id == id) {
            self.games.remove(pos);
            true
        } else {
            false
        }
    }

    /// Remove every cached game joined with the given wallet identity `npub`.
    ///
    /// Returns the number of removed entries.
    pub fn remove_by_npub(&mut self, npub: &str) -> usize {
        let before = self.games.len();
        self.games.retain(|game| game.npub != npub);
        before.saturating_sub(self.games.len())
    }

    /// Return the gate npub for the given server hostname, if one has been
    /// cached from a previous successful session.
    ///
    /// Returns `None` when no game entry for this server has a non-empty
    /// `gate_npub` field (e.g. first-time connect, or old cache entries).
    pub fn gate_npub_for_server(&self, server_host: &str) -> Option<&str> {
        self.games
            .iter()
            .find(|g| g.server == server_host && !g.gate_npub.is_empty())
            .map(|g| g.gate_npub.as_str())
    }

    /// Return games sorted by `last-connected` descending, with games that
    /// have no `last-connected` appearing last (sorted by `joined`).
    pub fn sorted(&self) -> Vec<&CachedGame> {
        let mut with_ts: Vec<&CachedGame> = self
            .games
            .iter()
            .filter(|g| g.last_connected.is_some())
            .collect();
        let mut without_ts: Vec<&CachedGame> = self
            .games
            .iter()
            .filter(|g| g.last_connected.is_none())
            .collect();

        // ISO-8601 strings sort lexicographically as a proxy for chronological order.
        with_ts.sort_by(|a, b| {
            b.last_connected
                .as_deref()
                .cmp(&a.last_connected.as_deref())
        });
        without_ts.sort_by(|a, b| b.joined.cmp(&a.joined));

        with_ts.extend(without_ts);
        with_ts
    }
}
