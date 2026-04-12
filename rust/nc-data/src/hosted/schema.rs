pub const SCHEMA_VERSION: &str = "1.0.0";

pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS game_metadata (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'setup',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    current_year INTEGER NOT NULL DEFAULT 3000,
    current_turn INTEGER NOT NULL DEFAULT 0,
    players INTEGER NOT NULL DEFAULT 4,
    recruiting TEXT NOT NULL DEFAULT 'none',
    lobby_visibility TEXT NOT NULL DEFAULT 'public',
    host_alias TEXT,
    summary TEXT,
    maintenance_enabled INTEGER NOT NULL DEFAULT 1,
    maintenance_interval_minutes INTEGER NOT NULL DEFAULT 1440,
    maintenance_next_due_unix_seconds INTEGER,
    catalog_dirty_since INTEGER
);

CREATE TABLE IF NOT EXISTS seats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id TEXT NOT NULL REFERENCES game_metadata(id),
    seat_number INTEGER NOT NULL,
    invite_code TEXT NOT NULL,
    invite_code_hash TEXT NOT NULL,
    player_pubkey TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    claimed_at INTEGER,
    created_at INTEGER NOT NULL,
    UNIQUE(game_id, seat_number),
    UNIQUE(game_id, invite_code_hash)
);

CREATE TABLE IF NOT EXISTS invite_requests (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    player_pubkey TEXT NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    processed_at INTEGER,
    decision_message TEXT,
    issued_invite_code TEXT,
    decision_published_at INTEGER,
    FOREIGN KEY (game_id) REFERENCES game_metadata(id)
);

CREATE TABLE IF NOT EXISTS turn_queue (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    turn INTEGER NOT NULL,
    player_pubkey TEXT NOT NULL,
    commands TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    submitted_at INTEGER NOT NULL,
    processed_at INTEGER,
    error_message TEXT,
    FOREIGN KEY (game_id) REFERENCES game_metadata(id)
);

CREATE TABLE IF NOT EXISTS outbox (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    kind INTEGER NOT NULL,
    pubkey TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    published_at INTEGER,
    relay_url TEXT,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (game_id) REFERENCES game_metadata(id)
);

CREATE TABLE IF NOT EXISTS thread_messages (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    player_pubkey TEXT NOT NULL,
    sender_role TEXT NOT NULL,
    sender_pubkey TEXT NOT NULL,
    sender_handle TEXT,
    body TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (game_id) REFERENCES game_metadata(id)
);

CREATE INDEX IF NOT EXISTS idx_seats_game_id ON seats(game_id);
CREATE INDEX IF NOT EXISTS idx_invite_requests_game_id ON invite_requests(game_id);
CREATE INDEX IF NOT EXISTS idx_invite_requests_status ON invite_requests(status);
CREATE INDEX IF NOT EXISTS idx_turn_queue_game_id ON turn_queue(game_id);
CREATE INDEX IF NOT EXISTS idx_turn_queue_turn ON turn_queue(game_id, turn);
CREATE INDEX IF NOT EXISTS idx_outbox_game_id ON outbox(game_id);
CREATE INDEX IF NOT EXISTS idx_outbox_status ON outbox(status);
CREATE INDEX IF NOT EXISTS idx_thread_messages_game_player
    ON thread_messages(game_id, player_pubkey, created_at);
"#;
