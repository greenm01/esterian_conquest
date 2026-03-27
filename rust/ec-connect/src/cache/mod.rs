//! Local game cache types and I/O.
//!
//! The cache file lives at `~/.local/share/ec/cache.kdl` and holds one
//! entry per game the player has joined.  It is a plain (unencrypted) KDL
//! file; no secret data is stored here.
//!
//! Format:
//! ```kdl
//! game id="friday-night" name="Friday Night EC" server="play.example.com" port=22 seat=2 npub="npub1aaa..." joined="2026-03-26T12:00:00Z" last-connected="2026-03-28T19:30:00Z"
//! game id="saturday-showdown" name="Saturday Showdown" server="war.example.com" port=22 seat=5 npub="npub1aaa..." joined="2026-03-27T10:00:00Z"
//! ```
//!
//! The picker sorts games by `last-connected` descending (most recently
//! played first).  Games with no `last-connected` timestamp appear last,
//! ordered by `joined`.

pub mod io;

pub use io::{cache_path, load_cache, load_cache_from, save_cache, save_cache_to};

/// One joined-game entry in the local cache.
#[derive(Debug, Clone)]
pub struct CachedGame {
    /// Game identifier slug (matches the server's roster game ID).
    pub id: String,
    /// Human-readable game name.
    pub name: String,
    /// Server hostname.
    pub server: String,
    /// SSH port.
    pub port: u16,
    /// Player seat number (1-based).
    pub seat: u32,
    /// The identity (npub) that joined this game.
    pub npub: String,
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
