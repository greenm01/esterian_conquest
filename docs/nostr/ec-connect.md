# ec-connect

`ec-connect` is the player-side client binary. It manages Nostr identity,
authenticates with game servers, and launches SSH-backed `ec-game` sessions.
During the current beta, the public GitHub player download lives on the repo's
GitHub Releases page. Public `ec-connect` player archives are available for
Windows x64, Linux x64, and macOS Apple Silicon. The packaged desktop client
supports Windows, Linux, and macOS, and the Linux build supports both X11 and
Wayland from the same package. `ec-connect-cli` remains available from Cargo
or source builds for advanced manual workflows, but it is not part of the
normal player handoff or the GUI release archive. Like `ec-game` and
`ec-sysop`, it should currently be treated as beta-quality software and
playtested accordingly.

## Player Experience

In the recommended public flow, the sysop gives the player one invite string in
the form `token@relay-host[:port]`. The player joins once with `--join`.
`ec-connect` discovers the gate key from the relay's public 30500 game
definition, launches the normal session handshake and terminal bridge, and
then persists the game locally once the in-game claim is confirmed.

### First Launch

```
$ ec-connect --join velvet-mountain@relay.example.com
[centered password window]
This password encrypts your wallet.
If you lose it, you will be locked out.
No IT support.
New password: ********
Confirm password: ********
Identity created.

Joining game... Welcome! You are Player 2 in "Friday Night EC."

[ec-game launches — full TUI session]
```

The player sees the game immediately. They never interact with SSH, Nostr
relays, or key management directly.

### Returning Player

```
$ ec-connect
[centered password window]
Password: ********

[picker opens]
Enter reconnect on the selected game.

[ec-game launches]
```

No invite code is needed. The picker remembers joined games locally, so a
returning player unlocks once, reconnects from the list, and returns to the
picker when the game session ends.

### Picker Mode

When run with no arguments, `ec-connect` first shows a centered masked
password window and then opens the fixed `80x25` main menu:

```
$ ec-connect
[centered password window]
Password: ********

ESTERIAN CONQUEST CONNECT                              alice-main
┌─────────────┬─────────────────┬──────────────────┬───────────────────┬──────┐
│Empire       │Game             │Server            │Gate               │  Seat│
├─────────────┼─────────────────┼──────────────────┼───────────────────┼──────┤
│House Vale   │Friday Night EC  │play.example.com:22│npub1gate…42k9m1 │     2│
│Aurora Crown │Saturday Showdown│war.example.com:22 │npub1gate…8n2xsw │     5│
└─────────────┴─────────────────┴──────────────────┴───────────────────┴──────┘
COMMANDS <- J K ^U ^D <N> <W> <I> <M> <L> <Q> ->
```

`J` and `K` move the selection, and the arrow keys also work. `Enter`
connects. `W` opens the wallet screen. `L` locks immediately, and `Alt-L`
also works from text-entry prompts. `Q` now asks for confirmation before
exiting the local shell, `Esc` mirrors `Q` implicitly, and `?` opens a
screen-specific help popup that explains the visible command-line buttons.
When the game session ends, the player returns to this menu.

### Join Flow from Picker

Pressing `N` in the picker opens an inline invite prompt:

```
CONNECT COMMAND <- Invite code <Q> <?> -> velvet-mountain@relay.example.com
```

In the packaged GUI, paste works with `Ctrl-V`, `Ctrl-Shift-V`,
`Shift-Insert`, or right-click.

After a successful join, the new game appears in the list and the player
is connected immediately. Public first joins no longer claim the seat until
the player actually saves the in-game empire name. If the player disconnects
before that save, the game stays in the picker as `Pending` and `Enter` retries
the original invite code. After a completed first join, `ec-connect` promotes
that row to `Joined`, refreshes the seat state, and then requests the static
starmap bundle. The download is best-effort: if it fails, the player still
enters the game and can retry later from the picker. Invalid invite codes stay
in the prompt and show an error notice instead of dropping out of the shell.

### Nostr User

A player who already has a Nostr identity can import their key before
joining:

```
$ ec-connect id import
Enter your nsec: nsec1...
Identity imported.

$ ec-connect --join copper-sunrise@relay.example.com
[centered password window]
Password: ********
Joining game... Welcome! You are Player 3 in "Friday Night EC."

[ec-game launches]
```

## Identity Management

The `W` wallet screen in the packaged GUI is intentionally single-identity.
On first launch it creates one local identity automatically. Later, `R`
replaces that current identity: paste an `nsec` to import one you already
saved elsewhere, or leave the field blank to generate a fresh local identity.
In the wallet detail popup, `Ctrl-P` copies the full `npub` and `Ctrl-S`
copies the full `nsec`. The CLI companion remains available from Cargo/source
builds for advanced multi-identity and automation-heavy workflows.

On first wallet creation from `ec-connect id new` or `ec-connect id import`,
the CLI prints the same left-justified wallet-loss warning before asking for
`New password:` and `Confirm password:` with masked input.

### Wallet

Player identities are stored in an encrypted wallet at:

```
~/.local/share/ec/wallet.kdl
```

The packaged GUI keeps exactly one identity active at a time and treats
imports as replacement. The wallet is encrypted with ChaCha20-Poly1305 using
a key derived from the player's password via PBKDF2-HMAC-SHA256.

This wallet is local player state, not authoritative hosted game state.

Decrypted wallet format:

```kdl
wallet active="0"
identity nsec="nsec1..." type="local" created="2026-03-26T12:00:00Z"
```

Identity types:

| Type | Description |
|------|-------------|
| `local` | Auto-generated by ec-connect on first launch |
| `imported` | Player imported their own Nostr secret key |

### CLI Companion Subcommands

```
ec-connect-cli id              Show active identity (npub)
ec-connect-cli id --secret     Show active identity (npub + nsec for backup)
ec-connect-cli id list         List all wallet identities
ec-connect-cli id new          Generate a new keypair in the wallet
ec-connect-cli id import       Import an existing nsec (bech32 or hex)
ec-connect-cli id switch N     Switch active identity to index N
```

### Key Import

Accepted formats for importing an existing Nostr identity:

- Bech32 secret key (`nsec1...`)
- 64-character hex secret key

Future: NIP-46 remote signer support (`bunker://...`) for players who use
hardware signers or browser extensions and prefer not to paste their
private key.

### Key Backup and Recovery

The player's nsec is the only thing that binds them to their game seats.
If the wallet is lost and the nsec is not backed up, the player is
permanently locked out of all games joined with that identity. The admin
can reissue an invite code for the seat, but the old identity's history
is orphaned.

Players should back up their nsec to a password manager or other secure
storage. In the packaged GUI player, the wallet detail popup shows the full
`npub` and `nsec` and can copy them directly to the clipboard. In the
Cargo/source companion CLI, `ec-connect-cli id --secret` displays the active
identity's `npub` and `nsec` for the same purpose.

## Configuration

Config file at:

```
~/.config/ec/config.kdl
```

This file is optional. Without it, `ec-connect` works using invite code
suffixes and command-line arguments. The config adds convenience for
players who connect to the same servers regularly.

Format:

```kdl
// Default Nostr relay for session handshakes
relay "wss://relay.example.com"

// Optional override for downloaded starmap bundles
maps-dir "/home/alice/Documents/Esterian Maps"

// Server bookmarks
server "friday" host="play.example.com" port=22
server "local" host="localhost" port=2222

// Default server (used by the CLI companion's direct mode)
default "friday"
lock-timeout-minutes 5
```

Fields:

| Field | Description |
|-------|-------------|
| `relay` | Default Nostr relay URL for session handshakes. Used when no relay is specified by invite code suffix or command-line flag. |
| `maps-dir` | Optional root directory for downloaded starmap bundles. If omitted, `ec-connect` uses the platform default local data directory. |
| `server` | Named bookmark for a game server. `host` is required; `port` defaults to 22. |
| `default` | Name of the server bookmark to use when `ec-connect-cli` is invoked in direct mode with no argument and there is only one game cached for that server. |
| `lock-timeout-minutes` | Idle timeout for the local `ec-connect` shell. Default `5`. Set `0` to disable idle locking. |

Individual server bookmarks do not carry their own relay URL. All servers
on the same `ec-connect` installation use the top-level relay. If a
player needs different relays for different servers, that can be added
later as a `relay` attribute on the server node.

In the TUI, `R` edits this same default relay value. If an older cached
game is missing its per-game relay, `ec-connect` can prompt once for the
correct relay and then save it back onto that cached row.

### Relay Resolution Priority

When connecting, `ec-connect` resolves the Nostr relay URL from these
sources, in order:

1. Explicit command-line flag (`--relay wss://...`)
2. Invite code suffix (`velvet-mountain@relay.example.com` implies
   `wss://relay.example.com`)
3. Config file `relay` field
4. Fallback: derive from server hostname (`wss://SERVER`)

TLS detection: localhost and private IP ranges use `ws://`, all other
hosts use `wss://`.

## Local Game Cache

After a successful reconnection, or after a first join has been confirmed by
the in-game claim/save path, `ec-connect` writes a cache entry so the picker
can display joined games without querying relays.

Cache file at:

```
~/.local/share/ec/cache.kdl
```

Format:

```kdl
game id="friday-night" name="Friday Night EC" player-name="House Vale" server="play.example.com" port=22 relay-url="wss://relay.example.com" seat=2 npub="npub1aaa..." gate-npub="npub1gate..." joined="2026-03-26T12:00:00Z" last-connected="2026-03-28T19:30:00Z"
game id="saturday-showdown" name="Saturday Showdown" server="war.example.com" port=22 seat=5 npub="npub1aaa..." gate-npub="npub1gate..." joined="2026-03-27T10:00:00Z"
```

Fields:

| Field | Description |
|-------|-------------|
| `id` | Game identifier slug (matches the server's hosted game ID) |
| `name` | Human-readable game name |
| `player-name` | Cached server-reported empire name for the active identity in that game |
| `server` | Server hostname |
| `port` | SSH port |
| `relay-url` | Exact relay URL used for successful sessions to this game, when known |
| `seat` | Player seat number (1-based) |
| `npub` | The identity that joined this game |
| `gate-npub` | The Nostr public key for the daemon that served this game. Used for reconnects and manual map downloads. |
| `joined` | Timestamp when this identity first completed the hosted join claim |
| `last-connected` | Timestamp of most recent connection (updated each session) |

The cache uses the 30502 SessionReady payload as the initial source of
`game_id`, `game_name`, `seat`, and `player_name`, but first invite joins are
only written durably after the post-session 30507 refresh confirms that this
identity actually claimed the seat. If the player leaves before naming the
empire, no durable row is written and the invite must be reused. Returning
reconnects for already-claimed seats can still refresh the cache immediately.
After a successful hosted session ends, `ec-connect` performs one lightweight
30507 state refresh so the cached `player-name` can pick up changes made
inside `ec-game`, such as first-time empire naming. If that refresh fails on a
returning reconnect, the existing cache row is kept as-is. The `npub` comes
from the active wallet identity. `relay-url` is copied from the resolved
target used for the successful handshake so picker reconnects can reuse the
same relay even when it is not the derived default. If an older cached row
still has no saved `relay-url` and the player's config also has no default
relay, the picker prompts for one before the handshake starts and saves it
back onto that row after a successful reconnect. `last-connected` is updated
on each successful SSH connection for games that are already in the cache.

This cache is local convenience state only. The server-side hosted seat
binding remains the source of truth.

The picker sorts games by `last-connected` descending, so the most
recently played game appears first.

## Desktop Launch Modes

### Public GUI

```
ec-connect                           Open the picker
ec-connect --join <INVITE-CODE>      Open the GUI and prefill a public join
```

`INVITE-CODE` is the public invite string:
`velvet-mountain@relay.example.com` or
`velvet-mountain@relay.example.com:7447`.

### Advanced CLI Companion

```
cargo run -q -p ec-connect --bin ec-connect-cli -- <args...>
```

The Cargo/source-only companion keeps the old direct terminal workflows,
including direct server connects, identity subcommands, and manual overrides.

### CLI Companion Options

```
--gate <NPUB>           Gate daemon Nostr public key (optional override / fallback)
--relay <URL>          Override Nostr relay URL
--maps-dir <PATH>      Override where downloaded starmap bundles are stored
--version              Print version
--help                 Print help
```

For normal public joins, the packaged GUI only needs the invite code and relay.
Supply the lower-level overrides through `ec-connect-cli` when the sysop does
not publish public 30500 definitions or when you are doing manual troubleshooting.

## Main Menu

The no-argument shell is a fixed `80x25` crossterm screen aligned with the
`ec-game` command-line style. It is not a ratatui app anymore.

### Layout

```
 ESTERIAN CONQUEST ── CONNECT

 Your Games
 > Friday Night EC     play.example.com    Seat 2    (3 hours ago)
   Saturday Showdown   war.example.com     Seat 5    (2 days ago)

 ─────────────────────────────────────────────────────────────────
 [J] Join new game   [M] Maps   [I] Identity info   [Q] Quit
```

Columns: empire name, game name, server address, shortened gate `npub`,
and seat number. The selected first-column cell is highlighted. If the
game list is empty (first launch with no `--join`), the screen shows a message
prompting the player to join a game.

### Controls

| Key | Action |
|-----|--------|
| Up/Down | Move selection |
| Enter | Connect to selected game |
| `J` | Enter invite code to join a new game |
| `M` | Open the maps popup to change the default save location and re-download the selected game's static starmap bundle |
| `R` | Edit the default relay URL used for joins and legacy cache rows |
| `I` | Show active identity (npub, number of identities in wallet) |
| `Q` / Esc | Quit |

### Join Prompt

Pressing `J` replaces the bottom bar with an inline text input:

```
 Enter invite code: _
```

The player types the invite code (with or without server suffix) and
presses Enter. `ec-connect` resolves the server, runs the Nostr
handshake, and if successful, adds the game to the list and connects
immediately. Esc cancels and returns to the game list.

After a successful first-time join, `ec-connect` performs a second Nostr
request to download the campaign's static map bundle. The bundle is
saved locally before SSH starts if the transfer succeeds. If the bundle
cannot be fetched or written, `ec-connect` shows a warning but still
continues into gameplay.

### Identity Display

Pressing `I` shows a brief overlay or status line:

```
 Identity: npub1aaa...xyz (local)   [1 of 2 identities]
```

Full packaged-player identity management lives under the `W` wallet screen.
The detail popup shows the complete `npub` and `nsec` and supports clipboard
copy for backup. Importing a different identity replaces the current one and
clears cached picker rows for the old `npub`.

### Manual Map Re-Download

Pressing `M` in the picker opens a popup with the current maps root.
The player can change that default save location and then press `Enter`
to download the selected game's current static map bundle again. The
bundle is still written atomically under the usual per-server and
per-game subdirectory beneath the chosen maps root.

The static star layout does not change during the campaign, so
`ec-connect` does not refresh maps automatically on normal reconnects.
Players fetch once on first join, then only when they explicitly ask to
download again.

### Post-Session Behavior

When a game session ends (player quits `ec-game` or connection drops),
the picker screen reappears. The `last-connected` timestamp for the game
is updated in the cache. The player can connect to another game, join a
new one, or quit.

In CLI direct mode (`ec-connect-cli friday`), the process exits after the game
session ends instead of returning to the picker.

## Multi-Game on Same Server

A player's npub can be in multiple games on the same server. The
disambiguation flow uses the local game cache and a fallback protocol
exchange:

### Cached Game ID

When connecting to a server where the player has a cached game entry,
`ec-connect` includes the game's `id` slug in the 30501 SessionRequest
as a `game-id` tag. `ec-gate` uses this to route directly to the correct
game without ambiguity.

### Ambiguous Reconnection

If the player connects to a server with no cached game ID (cache cleared,
new machine, or first time using the picker for a server they previously
joined through the CLI companion's direct mode), and their npub is in
multiple games on that server:

1. `ec-connect` sends a 30501 SessionRequest with no `game-id` tag.
2. `ec-gate` finds multiple hosted-game matches for the npub.
3. `ec-gate` returns a 30503 SessionError with error code
   `multiple_games` and a game list in the encrypted payload.
4. `ec-connect` displays the game list as a picker:

```
 Multiple games found on play.example.com:
 > Friday Night EC     Seat 2
   Saturday Showdown   Seat 5

 Select a game (Enter), or Esc to cancel.
```

5. Player selects a game. `ec-connect` retries with the `game-id` tag.
6. Both games are added to the local cache for future disambiguation.

### CLI Direct Mode Disambiguation

In CLI direct mode (`ec-connect-cli friday`), if the server has multiple games
for the player and no cached game ID, `ec-connect` falls back to a
simple numbered prompt:

```
Multiple games on play.example.com:
  1. Friday Night EC (Seat 2)
  2. Saturday Showdown (Seat 5)
Select [1-2]: 1
Connecting...
```

## Session Lifecycle

### Connect

1. Read and decrypt wallet (prompt for password if not cached).
2. Resolve server hostname and relay URL. If the server argument is a
   bookmark, look it up in config. Otherwise use the hostname directly
   and resolve the relay via the priority chain.
3. For a first public invite join, discover the game's 30500 definition
   and resolve the target game ID, gate npub, and SSH host metadata from
   the published pending invite hash.
4. Generate an ephemeral ed25519 SSH keypair for this session. This
   keypair is never stored; it lives only in memory for the duration of
   the connection.
5. Connect to the Nostr relay via WebSocket.
6. Publish a signed 30501 SessionRequest containing the player's npub,
   the ephemeral SSH public key, the resolved game ID, and the invite
   code for a first join.
7. Subscribe to 30502 (SessionReady) and 30503 (SessionError) events
   addressed to the player's npub.
8. Wait for a response from `ec-gate` (timeout: 15 seconds).

### Session Established

9. On receiving 30502 SessionReady: decrypt the NIP-44 payload to get
   the SSH host, port, server host key fingerprint, game ID, game name,
   and seat number.
10. If this is a returning reconnect to an already-claimed seat, refresh
    the local cache immediately from the SessionReady payload.
11. After the SSH session exits, refresh hosted seat metadata. For a
    first-time invite join, only write the durable picker row if that
    refresh confirms this identity claimed the seat in game. Then issue
    a 30504 MapRequest.
12. Disconnect from the Nostr relay. The relay is no longer needed.
13. Tear down the picker screen. In the packaged desktop GUI, switch into
    the embedded terminal view. In the CLI companion, put the local
    terminal into raw mode.
14. Open an SSH connection to the server using the ephemeral keypair.
    Verify the host key fingerprint matches the one in the SessionReady
    payload (or a cached known host).
15. Forward input and output through the active frontend. The desktop GUI
    renders the SSH session inside its embedded terminal view; the CLI
    companion attaches stdin/stdout directly. Forward terminal resize
    events as SSH window-change requests.
15. The player is now in `ec-game`.

### Disconnect

16. When the SSH channel closes (player quits the game or connection
    drops), restore the local terminal to normal mode.
17. The ephemeral SSH keypair is discarded (it was only in memory).
18. In GUI picker mode: redraw the picker screen with updated
    `last-connected` timestamps. In CLI direct mode: exit.

### Error

On receiving 30503 SessionError: decrypt the payload and handle based on
the error code:

- `multiple_games`: show game selection (see Multi-Game section above),
  then retry.
- All other errors: display the error message to the player and return
  to the picker (or exit in CLI direct mode).

Common errors:

- `invalid_code` — the invite code does not match any pending seat
- `code_claimed` — the invite code has already been claimed by another
  player
- `unknown_player` — the player's npub is not enrolled in any hosted game and no
  invite code was provided
- `game_not_active` — the game is not currently accepting connections
- `rate_limited` — too many session requests from this npub

## Live Session Rendering

The packaged desktop player keeps the SSH-backed `ec-game` session inside the
`ec-connect` window. It forwards keyboard input to the SSH channel, renders the
remote ANSI/VT terminal stream in-process, and forwards resize events as SSH
window-change requests so `ec-game` sees the correct terminal dimensions.

This is transparent to the player. The experience is still the classic
`ec-game` session, with the latency of the SSH connection.

### GUI and CLI Paths

Packaged desktop builds use the native GUI frontend for both the picker and the
live session. The Cargo/source-only `ec-connect-cli` companion keeps the older
raw-terminal bridge path for manual troubleshooting or terminal-first
workflows.

`russh` (the SSH client library) is pure Rust and works on Windows,
macOS, and Linux without platform-specific dependencies.

## Relay Selection

When connecting, `ec-connect` needs to know which Nostr relay to use for
the authentication handshake. The relay URL can come from:

1. Explicit command-line flag (`--relay wss://relay.example.com`)
2. Invite code suffix (`velvet-mountain@relay.example.com`)
3. Config file `relay` field
4. Fallback: derive from server hostname (`wss://SERVER`)

Priority is: explicit flag > invite code suffix > config file > fallback.

TLS detection: localhost and private IP ranges use `ws://`, all other
hosts use `wss://`.

## File Locations

By default, `ec-connect` stores maps under the platform's real Documents
folder and keeps its wallet/cache files in the platform-appropriate
config/data locations.

| File | Default Linux-style path | Purpose |
|------|------|---------|
| Config | `~/.config/ec/config.kdl` | Server bookmarks, default relay, optional `maps-dir` override |
| Wallet | `~/.local/share/ec/wallet.kdl` | Encrypted identity store |
| Cache | `~/.local/share/ec/cache.kdl` | Joined games and connection history |
| Maps root | `~/Documents/ec/maps/` | Downloaded static map bundles |

Within the maps root, bundles are stored as:

```text
<maps-root>/<relay_host>[_port]/<game_id>/
  starmap.txt
  starmap.csv
  starmap-DETAILS.csv
```

Path resolution priority for the maps root is:

1. `--maps-dir <PATH>` on the current `ec-connect` invocation
2. `maps-dir` in `config.kdl`
3. Platform default Documents directory (`~/Documents/ec/maps` or `%USERPROFILE%\Documents\ec\maps`)

In picker mode, if the player changes the maps root from the `M` popup,
that choice is written back to `maps-dir` and becomes the active maps
root for the rest of the current picker session.

## Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `nostr-sdk` | Nostr protocol: event signing, NIP-44 encryption, relay WebSocket |
| `russh` | SSH client: ephemeral key auth, channel I/O, window resize |
| `ratatui` | Picker screen TUI rendering |
| `crossterm` | Local terminal: raw mode, resize detection, ratatui backend |
| `chacha20poly1305` | Wallet encryption (or via nostr-sdk's crypto) |
| `pbkdf2` | Password-based key derivation for wallet |
| `kdl` | Wallet, config, and cache file parsing |
| `dirs` | Platform-appropriate config/data directory resolution |
| `tokio` | Async runtime for relay + SSH I/O |
| `clap` | CLI argument parsing |
