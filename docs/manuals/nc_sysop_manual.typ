// Nostrian Conquest — Sysop Manual
// Typst source — generates US Letter PDF

#set document(
  title: "Nostrian Conquest — Sysop Manual",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 4, day: 2),
)

#set page(
  paper: "us-letter",
  margin: (x: 1in, y: 1in),
  footer: none,
)

#set text(
  font: "IBM Plex Serif",
  size: 11pt,
)

#show raw: set text(font: "IBM Plex Mono")

#set par(
  justify: true,
  leading: 0.65em,
)

#set heading(numbering: "1.")

// Bold header row styling for all tables
#show table.cell.where(y: 0): strong

// Admonition helper
#let admonition(kind, body) = {
  block(
    width: 100%,
    inset: 10pt,
    stroke: 0.5pt + luma(160),
    radius: 3pt,
    fill: luma(245),
  )[
    #text(weight: "bold")[#kind:] #body
  ]
}

// ─── Title Page ───────────────────────────────────────────────────────────────

#let manual_license_notice = [
  This manual © 2026 Mason A. Green and is licensed under CC BY-NC-SA 4.0.
]

#let numbered_footer = context align(center)[
  #set text(size: 9pt, fill: luma(120))
  Page #counter(page).get().first() of #counter(page).final().first()
]

#align(center + horizon)[
  #text(size: 28pt, weight: "bold")[Nostrian Conquest]
  #v(0.5em)
  #text(size: 18pt)[Sysop Manual]
  #v(1em)
  #text(size: 10pt, style: "italic")[A from-scratch Rust recreation inspired by the classic 1990s BBS door game Esterian Conquest.]
  #v(2em)
  #text(size: 11pt, fill: luma(120))[Version 1.0.0-beta.1 — Beta]
  #v(0.5em)
  #text(size: 11pt, fill: luma(120))[Revision date: April 2, 2026]
]

#pagebreak()

#align(center + horizon)[
  #v(3em)
  #image("assets/cc-by-nc-sa-4.0-badge.svg", width: 3.3in)
  #v(1em)
  #block(width: 80%)[
    #set text(size: 9pt, fill: luma(110))
    #manual_license_notice
  ]
  #v(0.5em)
  #text(size: 9pt, fill: luma(110))[
    License text: #link("https://creativecommons.org/licenses/by-nc-sa/4.0/")
  ]
]

#pagebreak()
#counter(page).update(1)
#set page(footer: numbered_footer)

// ─── Table of Contents ────────────────────────────────────────────────────────

#outline(
  title: "Contents",
  indent: 1.5em,
)

#pagebreak()

// ─── 1. Introduction ──────────────────────────────────────────────────────────

= Introduction

Nostrian Conquest is a multi-player strategy game inspired by a DOS-era BBS
classic and reimplemented in Rust for modern systems. The Rust-native stack
provides the game engine, player client, and admin tooling for modern hosts.

This manual is for the *sysop* — the person responsible for hosting a game
instance. It covers:

- choosing the right deployment path
- creating and maintaining a DB-only game
- running the recommended Nostr host
- running a local or direct SSH game
- putting `nc-door` on a BBS
- managing players and yearly maintenance

For player-facing rules and gameplay, see the *Player Manual*.

This manual is the authoritative sysop manual for the Rust edition of
Nostrian Conquest. The preserved original `.DOC` set in `original/v1.5/`
remains historical reference material and an ambiguity fallback for classic
operator intent, not a higher-authority replacement for the current Rust
manuals.


// ─── 2. Getting Started: Hosting a Campaign ───────────────────────────────────

= Getting Started: Hosting a Campaign

== System Requirements

- Linux, macOS, or Windows
- Rust toolchain (stable) for normal self-host and VPS deployment
- For the recommended public-hosted flow: an SSH-accessible host plus a Nostr
  relay reachable by players
- A terminal emulator for localhost or direct SSH play
- A BBS server with socket support only if you specifically want legacy door
  deployment

== Building from Source

From the repository root:

```
cargo build --release -p nc-sysop -p nc-game -p nc-connect
```

The release binaries will be in `target/release/`.

== Current Beta Distribution

During the current beta, public GitHub Releases include Windows x64, Linux x64,
and macOS Apple Silicon `nc-connect` player archives. Public releases also
include Windows x64, Windows x86 (32-bit), Windows 7+ x86 (32-bit), and Linux
x64 `nc-sysop` BBS/sysop packages.

Keep the binary roles straight.

`nc-connect` is the packaged player client for the recommended Nostr path.
`nc-game` is the direct localhost and SSH/VPS session client. `nc-door` is the
BBS door entrypoint on Windows and Linux. `nc-sysop` is the administrator's
tool for creation, settings, maintenance, and hosted operations.

For the Rust edition:

1. VPS sysops should build from tagged source with Cargo and use
   `scripts/install_vps.sh`.
2. Hosted players can use the public GitHub Releases `nc-connect` archives
   with the player manual on Windows, Linux, or macOS.
3. BBS sysops can use the public `nc-sysop` package on Linux x64 or Windows
   x64, or build from tagged source with Cargo.
4. Localhost sysops build from tagged source with Cargo and run `nc-game`
   directly.

The public Nostrian packages do not bundle preserved Esterian Conquest
executables, manuals, or DOS helper assets.

== Choose Your Deployment

NC has three practical ways to run the Rust game.

=== Nostr Host

Use this when a sysop wants the normal public multiplayer path. Players use
`nc-connect`. The host runs `nc-sysop nostr serve`, and the live game runs
inside `nc-game` over SSH.

1. Build the binaries.
2. Create a game directory with `nc-sysop new-game`.
3. Register the game with the host and start `nc-sysop nostr serve`.
4. Run `nc-sysop maint` when the year should advance.

The standard VPS layout is:

```
/usr/local/bin/nc-game
/usr/local/bin/nc-sysop
/usr/local/bin/nc-gate-keys
/etc/nc-gate/config.kdl
/etc/nc-gate/identity.kdl
/var/lib/nc-gate/keys/
/srv/nc/games/<slug>/ncgame.db
```

This path can live on a Linux VPS or any other SSH-reachable Linux host. The
full hosted setup is covered below under *Recommended Public Host*.

=== Localhost Session

Use this when a sysop wants direct same-machine play for himself, hotseat
testing, or a small trusted session.

1. Build the binaries from source.
2. Create a game directory with `nc-sysop new-game`.
3. Run `nc-game` directly.
4. Run `nc-sysop maint` when the year should advance.

The simplest localhost setup is:

```
nc-sysop new-game /home/sysop/nc-games/friday-night --name "Friday Night NC" --players 4
nc-game --dir /home/sysop/nc-games/friday-night --player 1
```

=== BBS Door Host

Use this when the sysop wants `nc-door` as a door under Mystic, ENiGMA½, or a
similar BBS.

1. Create the game directory.
2. Write a minimal per-game `config.kdl` with `players` and any fixed-seat
   `reservations`.
3. Initialize it with `nc-sysop new-game --bbs`.
4. Launch `nc-door` with a BBS dropfile.
5. Keep maintenance outside the door. Run `nc-sysop maint` from the host or
   the BBS event runner.

For BBS hosting, use the public `nc-sysop` package or build from source.
Localhost play remains a source-build path. VPS/Nostr hosting remains a
tagged-source Cargo workflow.

For the exact launcher setups, see:

- `docs/sysop/bbs/mystic-bbs-setup.md`
- `docs/sysop/bbs/enigma-bbs-setup.md`
- `docs/sysop/bbs/synchronet-bbs-setup.md`
- `docs/sysop/bbs/wwiv-bbs-setup.md`

== Game Directory Layout

Hosted/Nostr campaigns are DB-only:

```
/path/to/mygame/
  ncgame.db
```

BBS door campaigns keep a minimal live config file beside the runtime DB:

```
/path/to/mygame/
  config.kdl
  ncgame.db
```

All tools take `--dir /path/to/mygame` to locate the game.

== Initializing a New Game

Create a new game with `nc-sysop new-game`:

```
sudo -u ncgame nc-sysop new-game /srv/nc/games/friday-night --name "Friday Night NC" --players 4
```

This creates one runtime file: `ncgame.db`.

For BBS door campaigns, write `config.kdl` first and then initialize with:

```
nc-sysop new-game --bbs /path/to/mygame
```

#admonition("IMPORTANT")[
  On a VPS host installed with `scripts/install_vps.sh`, create hosted games as
  the `ncgame` service user. If you create `/srv/nc/games/<slug>` as `root`,
  the `nc-nostr.service` daemon may fail to write session leases and hosted
  joins can time out.
]

The supported public creation flags are:

- *`--name`:* String. Optional game title stored in `ncgame.db`. If omitted,
  `nc-sysop` derives a title from the directory slug.
- *`--players`:* Integer. Number of empires. Supported range: `1-25`. Default:
  `4`.
- *`--seed`:* Integer. Optional campaign RNG seed. It controls map layout,
  starting positions, and all random events. If omitted, the engine picks a
  random seed and saves it to `ncgame.db`. The seed cannot be changed after
  creation.
- *`--bbs`:* Flag. Initialize a BBS campaign from an existing per-game
  `config.kdl`. In this mode, `--name` and `--players` are not accepted on the
  command line. Use `--seed` only if you intentionally want a reproducible map
  for that one campaign.

#admonition("NOTE")[Use a different seed for every game. Reusing the same seed produces the same map, starting positions, and event sequence every time. For BBS campaigns, keep that override on the `new-game --bbs --seed ...` command line instead of storing it in `config.kdl`.]

The target directory basename becomes the stable game slug. It must use only
lowercase ASCII letters, digits, and dashes. The slug is distinct from the
human-readable `game_name`, and both are distinct from the per-seat invite
codes used by `nc-connect`.

== Recommended Public Host

For new public campaigns, the recommended deployment path is `nc-sysop nostr`
plus `nc-connect`. In that model, the sysop runs the normal Rust engine on a
host, publishes the Nostr-facing daemon, and players join from their own
machines with invite codes. The daemon handles identity and seat routing; the
live game still runs inside `nc-game` over SSH. This keeps the original
asynchronous NC rhythm without requiring per-player Unix account management or
BBS door middleware.

If you are recruiting players for a live public game, point them first to
#link("https://nostrian-conquest.com")[nostrian-conquest.com]. That landing page
can carry the current public contact points and community links before you
issue seat-specific invites. At the moment, it should direct them to the
Discord channel at #link("https://discord.gg/FMr8sfBa")[discord.gg/FMr8sfBa].

A minimal hosted setup looks like:

```
sudo -u ncgame nc-sysop new-game /srv/nc/games/friday-night --name "Friday Night NC" --players 4
sudo nc-sysop host games add --config /etc/nc-gate/config.kdl --dir /srv/nc/games/friday-night
sudo systemctl restart nc-nostr.service
nc-sysop nostr init
nc-sysop nostr serve
```

The values handed to players come from `/etc/nc-gate/config.kdl`:

- `relay` is the Nostr relay URL
- `ssh-host` and `ssh-port` are published in relay discovery during the first session handshake

For a self-hosted relay, that `relay` URL must already work from outside the
box. If `nostr-rs-relay` is listening only on loopback, also run the public
HTTPS reverse proxy for the relay hostname before expecting hosted joins to
work.

After the daemon is running, view the hosted seat state and get the
ready-to-distribute invite commands:

```
nc-sysop nostr seats --dir /path/to/mygame
```

The output lists every seat. Pending seats show the canonical join line:

```
Seat 1  [pending]
  amber-river@relay.example.com

Seat 2  [claimed]
  npub1...
```

Send each player the raw invite code. The player:

1. Runs `nc-connect`.
2. Presses `N`.
3. Pastes the invite code and presses Enter.

That is the full player-side flow. The invite carries the seat token and relay
host, and `nc-connect` discovers the rest from the relay. No extra flags are
normally required.

On first join, `nc-connect` creates or unlocks the player's encrypted
identity and opens the SSH-backed `nc-game` session. The hosted seat is not
claimed until the player actually saves the in-game empire name. If the player
disconnects before that save, the invite is still pending and can be used
again. After a completed first join, `nc-connect` caches the game locally,
downloads the static starmap bundle, and returning players reconnect without
re-entering any flags.

One hosted identity can claim only one seat in a given game. If the same
keychain identity tries to redeem a second invite for that game, the daemon now
rejects it and expects the player to reconnect with the already-claimed seat.

If an invite code is lost or compromised, reissue it:

```
nc-sysop nostr reissue --dir /path/to/mygame --player 2
```

This generates a fresh code for that seat, clears the old hosted `npub`
binding, and republishes that game's public `30500` metadata when possible.
It does *not* reset the runtime empire. Use this path when you want a new NC
identity to take over the already-joined seat and keep the existing empire
state.

If you need to fully rewind one hosted seat back to a true pre-join state,
use the destructive reset flag instead:

```
nc-sysop nostr reissue --dir /path/to/mygame --player 2 --nuke-seat
```

`--nuke-seat` clears the hosted `npub`, rotates the invite, resets that
player slot back to the original seeded year-3000 baseline, and republishes
`30500` when possible. This is only allowed during year `3000` before the
first maintenance turn, and it refuses while any live hosted session lease
exists.

If a player reports that a pending invite cannot be found on the relay, check
and repair the published hosted metadata directly:

```
nc-sysop nostr verify --dir /path/to/mygame
nc-sysop nostr publish --dir /path/to/mygame
```

== Hosted Player Identity Management

Hosted seats are bound to the first player identity that completes the
in-game join and saves the empire name. In practice this means the seat is
tied to one `npub` until the sysop changes it. Returning players should
reconnect with the same local NC keychain identity they used when they finished
that first join.

Players should not expect to paste the same invite into a brand-new keychain and
take over an already-claimed seat. `nc-gate` treats that as a different player
identity and rejects the join.

Players also should not expect one keychain identity to hold two seats in the
same hosted game. Seat 1 and seat 2 in one campaign must be claimed by
different identities, even if the same human is testing both paths.

If a player loses or forgets the original local identity, the supported
recovery path is:

1. Reissue that seat with `nc-sysop nostr reissue`.
2. Send the player the new invite.
3. Have the player redeem it from the new keychain identity so that identity
   takes over the existing empire.

Reissuing is the deliberate “move this seat to a new identity” action. It
clears the old hosted `npub` binding and rotates the invite code at the same
time, but it preserves the runtime empire.

If the goal is instead “forget this joined player and make the seat brand-new
again,” use `nc-sysop nostr reissue --nuke-seat`. That path destroys the old
runtime player slot and should be treated as a first-turn recovery tool, not a
mid-campaign transfer workflow.

Hosted seat claims are stored in `ncgame.db`. That SQLite state is the
authority for invite codes, claim status, and bound player `npub`s. Hosted
claim state no longer lives in a per-game roster file.

#admonition("NOTE")[
  `/etc/nc-gate/config.kdl` is host-owned. Game-registry edits such as
  `host games add` and `host games remove` should be run as root. Restart
  `nc-nostr.service` after changing the game list so the daemon reloads the
  config.
]

// ─── 3. Game Directory Structure ─────────────────────────────────────────────

= Game Directory Structure

/ `ncgame.db`: The SQLite runtime database. All game state lives here. Hosted
  seat claims live here too. Do not edit it by hand.

/ `config.kdl`: Present only for BBS door campaigns. It holds `players` and
  optional seat `reservations`. Hosted/Nostr campaigns do not use it.

// ─── 4. Configuration ─────────────────────────────────────────────────────────

= Configuration <configuration>

Keep these three paths separate:

=== Hosted / Nostr Host

- Live state: `/etc/nc-gate/config.kdl` plus per-game `ncgame.db`
- Runtime path: `nc-gate` launches hosted `nc-game` sessions
- Operator rule: use `nc-sysop settings set` on the game directory; `maint-all`
  scheduling applies here

=== Direct `nc-game`

- Live state: per-game `ncgame.db`
- Runtime path: `nc-game --dir ... --player ...` on localhost or over SSH
- Operator rule: this uses the same non-BBS settings surface as hosted games,
  but there is no gate, no relay, and no normal `maint-all` use

=== BBS Door Host

- Live state: per-game `config.kdl` plus `ncgame.db`
- Runtime path: `nc-door` with a dropfile; ENiGMA½ also passes `--socket-port`
  and Windows Synchronet passes `--socket-descriptor`
- Operator rule: BBS `config.kdl` supports only `players` and `reservations`;
  `settings set` does not apply
- Common hosts: Mystic, ENiGMA½, and Synchronet on Windows or Linux

Use `nc-sysop settings ...` to inspect any non-BBS campaign:

```
nc-sysop settings show --dir /srv/nc/games/friday-night
nc-sysop settings set --dir /srv/nc/games/friday-night --game-name "Friday Night NC"
nc-sysop settings reserve --dir /srv/nc/games/friday-night --player 1 --alias SYSOP
```

For the full non-BBS settings reference, including the raw `settings show`
shape and the advanced carried-forward state fields, see
`docs/sysop/rust/campaign-settings.md`.

=== BBS `config.kdl`

BBS campaigns use a minimal live per-game `config.kdl` instead of the non-BBS
settings surface:

```kdl
players 4
reservations {
  seat player=1 alias="SYSOP"
  seat player=2 alias="NightShade"
}
```

Supported BBS fields are only:

- `players`
- `reservations`

`nc-sysop settings show --dir /path/to/mygame` reports BBS campaigns in a
different shape:

```text
mode=bbs
players=4
reservation seat=1 alias=SYSOP
reservation seat=2 alias=NightShade
```

Do not put `game_name`, `theme`, `snoop`, `session`, `inactivity`, or
`maintenance` in the BBS file. Do not put `seed` there either. It is a one-shot
`new-game` command line override, not live BBS config.

For BBS campaigns, `nc-sysop settings reserve` and `settings unreserve` edit
this file for you. `settings set` is a non-BBS command surface.

== Display Defaults

The default display look is controlled by `default_theme_key`. That key names a
compiled-in color set. It is not a file path.

Players may still choose a local color theme inside `nc-game`. That choice is
stored in `ncgame.db` as a player preference.

In BBS door mode, `nc-door` does not use `default_theme_key` at runtime. Door
sessions always force the bundled `mag16` palette so ANSI16 terminals and BBS
clients get a predictable color-safe baseline.

// ─── 6. SSH Access ────────────────────────────────────────────────────────────

= SSH Access

The recommended hosted path above already uses SSH under the hood: players
enter through `nc-connect`, and the daemon opens a PTY running `nc-game`.
You can also run `nc-game` over SSH directly when you want a private
shared-host setup, manual debugging, or a simple trusted deployment without
the Nostr invite flow.

`nc-game` runs cleanly over SSH. No special flags are required for modern
terminal sessions.

Color mode is auto-detected from the environment:

- `COLORTERM=truecolor` → 24-bit RGB
- `TERM` containing `256color` → 256-color
- Otherwise → 16-color ANSI fallback

Force a specific mode with `--color-mode` if your SSH setup does not propagate
`COLORTERM` reliably:

```
nc-game --dir /path/to/mygame --player 1 --color-mode 256
```

UTF-8 encoding (the default) is correct for SSH sessions on modern terminals.
Use `--encoding cp437` only if you are proxying through a BBS or a CP437
terminal emulator over SSH.

// ─── 7. Localhost Session Setup ───────────────────────────────────────────────

= Localhost Session Setup

Localhost play remains a fully supported secondary mode for solo campaigns,
hotseat sessions, and sysop testing. Run `nc-game` directly in your terminal:

```
nc-game --dir /path/to/mygame --player 1
```

Color mode and encoding default to `auto` / `utf8`, which is correct for
modern terminal emulators. The client detects `COLORTERM` and `TERM`
automatically.

// ─── 8. BBS Door Setup ────────────────────────────────────────────────────────

= BBS Door Setup

`nc-door` is the standard BBS entrypoint on Windows and Linux. Use it when the
host is a BBS. Keep `nc-game` for direct localhost and SSH/VPS sessions. In
door mode, `nc-door` uses CP437 and 16-color ANSI for classic terminal clients.

== Flags for Door Mode

```
nc-door --dir /path/to/mygame --dropfile /path/to/DOOR32.SYS --encoding cp437 --color-mode ansi16
```

`--encoding cp437` switches box-drawing characters and other extended
characters to the CP437 code page, which is required for correct rendering
in classic ANSI-aware BBS terminals (SyncTERM, NetRunner, etc.).

When `--encoding cp437` is active, `--color-mode` defaults to `ansi16`
automatically. Override only if you know your BBS clients support a richer
color depth.

Door sessions also force the bundled `mag16` theme regardless of the campaign's
normal `default_theme_key`. The in-game *A>nsi color ON/OFF* command toggles
between that ANSI16 palette and a greyscale monochrome projection.

== Drop File

`nc-door` can read a BBS drop file directly with `--dropfile <path>`:

```
nc-door --dir /path/to/mygame --dropfile /path/to/DOOR32.SYS
```

Supported drop file formats (auto-detected by filename, case-insensitive):

- `DOOR32.SYS` — modern standard (Enigma, Mystic, Talisman, etc.)
- `DOOR.SYS` — legacy, widest BBS software support
- `CHAIN.TXT` — WWIV format

The drop file supplies the player alias and session time limit. Explicit CLI
flags always override drop file values. When `--dropfile` is given and
`--encoding` is not, encoding defaults to `cp437` automatically.

`--timeout <minutes>` sets a session time limit independently of a drop file.

Reserve seats in the BBS campaign's `config.kdl` when you want the caller
alias to determine the empire automatically. You can either edit the file
directly or use:

```sh
nc-sysop settings reserve --dir /path/to/mygame --player 1 --alias SYSOP
nc-sysop settings reserve --dir /path/to/mygame --player 2 --alias NightShade
```

With that in place, a reserved caller can launch with `--dropfile` alone.
If the caller alias is not reserved, `nc-door` still uses the same
dropfile-only door flow:

- if the alias already matches a stored joined player handle, that caller
  resumes the matching empire automatically
- if the alias does not match an existing joined empire, the caller lands on
  the BBS first-time menu and can claim the lowest-numbered open unreserved
  empire only when the join is actually confirmed
- if the game is full, the caller still reaches the BBS first-time menu, but
  `J` is refused with the normal no-open-empires notice

#admonition("NOTE")[
  The original DOS `ECGAME.EXE` v1.5 expects a strict 32-line WWIV-style
  `CHAIN.TXT` drop file. Enigma BBS-generated `DOOR.SYS` / `DORINFO` files
  will crash `ECGAME.EXE`. See the legacy DOS compatibility section in
  `docs/sysop/bbs/enigma-bbs-setup.md` if you need to host the original
  binary.
]

== Modern BBS Hosts

The native Rust `nc-door` binary is now verified on Mystic, ENiGMA½, and
Synchronet, and WWIV now has a validated Linux path. The shared core launch
shape is simple: pass `--dir` and a dropfile. ENiGMA½ also passes
`--socket-port`, and Windows Synchronet also passes `--socket-descriptor`.
The helper wrapper at `tools/bbs/run_nc_rust.sh` remains useful only for
source-tree testing and is not the normal packaged door path.

Mystic uses the direct dropfile path:

```
nc-door --dir /path/to/mygame --dropfile /path/to/DOOR32.SYS
```

ENiGMA½ uses the same dropfile plus its temporary localhost socket server:

```
nc-door --dir /path/to/mygame --dropfile /path/to/DOOR32.SYS --socket-port {srvPort}
```

Windows Synchronet also passes the inherited socket descriptor:

```
nc-door --dir /path/to/mygame --dropfile %f --socket-descriptor %H
```

Linux Synchronet is validated with the same `DOOR32` dropfile shape. If
your Synchronet install mangles a long native `cmd=` line, keep the full
`nc-door --dir ... --dropfile ...` invocation inside a tiny wrapper and pass
only `%f` into that wrapper.

WWIV is documented separately as a validated Linux `CHAIN.TXT` path:

```
nc-door --dir /path/to/mygame --dropfile /path/to/CHAIN.TXT
```

Treat native Windows WWIV as unvalidated for now. In one local Windows 11 test
with an older 2023 WWIV Windows package, Defender quarantined the downloaded
binary and `bbs.exe` then crashed before login in both remote and local-node
flows.

For the exact launcher setups, see
`docs/sysop/bbs/mystic-bbs-setup.md`,
`docs/sysop/bbs/enigma-bbs-setup.md`,
`docs/sysop/bbs/synchronet-bbs-setup.md`, and
`docs/sysop/bbs/wwiv-bbs-setup.md`.

// ─── 9. File-Based Turn Submission ──────────────────────────────────────────

= File-Based Turn Submission

For localhost, shared-host, or custom-client workflows, players can submit
orders by writing a KDL turn file and passing it to `nc-game submit-turn`.
Use `--check` to validate without writing, then run without it to apply:

```
nc-game submit-turn --check --dir /path/to/mygame --player 1 --file /path/to/player1-turn.kdl
nc-game submit-turn --dir /path/to/mygame --player 1 --file /path/to/player1-turn.kdl
```

The `--player` value must match the `turn player=...` header in the file. If
any order in the file is invalid, the entire submission is rejected and nothing
is written. This is a direct apply command, not a queue or upload inbox.

A minimal turn file:

```kdl
turn player=1 year=3000
tax rate=37
```

A fuller example:

```kdl
turn player=1 year=3000

tax rate=37

planet record=16 {
  build points=4 kind="scout"
}

fleet record=1 {
  order speed=3 kind="scout_system" x=16 y=13
}

message to=2 subject="Border" body="Watching the north lane."
```

For the full node reference and schema, see `docs/sysop/rust/turn-kdl.md`.

// ─── 10. Turn Processing and Maintenance ─────────────────────────────────────

= Turn Processing and Maintenance

Run yearly maintenance with:

```
nc-sysop maint /path/to/mygame [turns]
nc-sysop maint-all [--config /etc/nc-gate/config.kdl]
```

`nc-sysop maint` advances the campaign in `ncgame.db` immediately. It does not
change the schedule fields. NC does not schedule maintenance by itself. For
one direct localhost game, one manual SSH/VPS game, or one BBS game, invoke
`nc-sysop maint` from your own scheduler or event tooling:

- a `systemd` timer
- `cron`
- a BBS event runner
- or manual sysop operation

For multi-game hosted/Nostr operation, prefer a single global timer that runs
`nc-sysop maint-all`. It reads the configured game directories from the gate
config, advances only games whose schedule is enabled and already due, and
skips games with live session leases so a player is never interrupted by
maintenance. After a scheduled run, it writes the next due time back into
`ncgame.db`.

To put one hosted game on a real schedule, turn scheduling on and set the
first due time yourself:

```
nc-sysop settings set --dir /srv/nc/games/friday-night --maintenance-enabled on --maintenance-interval-minutes 10080 --maintenance-next-due 1775347200
```

That example enables weekly scheduling. `10080` is seven days in minutes.
`1775347200` is just a sample Unix timestamp. Replace it with the first due
time you actually want.

Treat these schedule fields as hosted `maint-all` metadata. They do not create
their own timer. They are not BBS `config.kdl` fields. If you run a direct
localhost or manual SSH game, you can ignore them and schedule `maint
/path/to/mygame` yourself.

// ─── 11. Player Management ────────────────────────────────────────────────────

= Player Management

Non-BBS campaigns keep player reservations and the carried-forward
inactivity-related state in `ncgame.db`. BBS door campaigns do not use a
separate inactivity block in `config.kdl`; caller idle handling belongs to the
BBS software.

== Reserving Seats

To reserve empire slots for specific BBS users, edit the per-game BBS
`config.kdl` or use:

```sh
nc-sysop settings reserve --dir /path/to/mygame --player 1 --alias SYSOP
nc-sysop settings reserve --dir /path/to/mygame --player 2 --alias NightShade
```

Each `seat` entry binds one 1-based empire slot to one caller alias. Alias
matching is ASCII case-insensitive, so `NightShade`, `nightshade`, and
`NIGHTSHADE` are treated as the same reservation.

With reservations in place, launch `nc-door` with `--dropfile` and let the
caller alias choose the empire automatically:

```sh
nc-door --dir /path/to/mygame --dropfile /path/to/DOOR32.SYS
```

Important rules:

- if the caller alias is reserved, `--dropfile` alone is enough
- if the caller alias is not reserved and already matches a stored joined
  player handle, that caller resumes the matching empire automatically
- if the caller alias is not reserved and does not match an existing joined
  player handle, the caller starts on the BBS first-time menu
- if a reservation conflicts with an already-stored different player handle, `nc-door` will stop with a clear error so the sysop can reconcile the reservation and the runtime state
- if no open unreserved empires remain, `J` from the BBS first-time menu is
  refused and the caller stays on that menu

Reserving a seat does not by itself join or pre-name the empire. It only
routes the caller to the intended slot. The usual first-time join flow still
claims an empire only on successful join confirmation, and that first
successful join records the caller alias into the player record for later
dropfile logins.

For localhost and direct console play, use `nc-game` and follow the localhost
setup section earlier in this manual instead of the BBS dropfile path.

// ─── 12. Terminology ──────────────────────────────────────────────────────────

= Terminology

/ game directory: The directory containing `ncgame.db` for one running game.
  Passed to all tools with `--dir`.

/ `nc-sysop`: The public Rust command-line sysop tool for campaign creation
  maintenance, and Nostr hosting.

/ `nc-connect`: The beta-quality player-side connection client for the
  recommended hosted flow. It manages the player's Nostr identity, joins
  games by invite code, downloads the static starmap bundle on first join,
  and opens the SSH-backed `nc-game` session.

/ `nc-game`: The Rust TUI player client for direct localhost and SSH/VPS
  sessions.

/ `nc-door`: The Rust BBS door entrypoint. It runs the same game flow as
  `nc-game`, but it is the staged binary for Windows and Linux BBS hosts.

/ `ncgame.db`: The SQLite database that is the runtime source of truth for the
  Rust engine.

/ non-BBS campaign settings: The sysop-managed runtime policy rows stored in
  `ncgame.db` for hosted/Nostr and direct `nc-game` campaigns. They control
  game name, the default compiled-in color set, hosted maintenance metadata,
  optional alias reservations, and a small set of carried-forward classic
  setup fields.

/ `config.kdl`: Present only for BBS door campaigns. It holds `players` and
  optional seat `reservations`.

// ─── 13. CLI Reference ────────────────────────────────────────────────────────

= CLI Reference

== nc-sysop

```
nc-sysop <subcommand> [options]
```

- *`new-game`:* Create a new campaign directory. Hosted/Nostr campaigns use
  `--name`, `--players`, and `--seed`. BBS campaigns use `new-game --bbs` with
  a minimal per-game `config.kdl`.
- *`settings show|set|reserve|unreserve`:* Inspect or edit non-BBS runtime
  policy in `ncgame.db`, or edit BBS seat reservations in per-game
  `config.kdl`.
- *`host games list|add|remove`:* Inspect or edit the global game registry in
  `/etc/nc-gate/config.kdl`.
- *`host status`:* Summarize the configured host, served game directories,
  claim counts, busy state, and maintenance-due state.
- *`nostr init`:* Initialize the Nostr-hosting identity and config for the
  recommended public multiplayer path.
- *`nostr serve`:* Run the Nostr-facing daemon that authenticates players and
  launches `nc-game` sessions.
- *`nostr seats`:* List the hosted seat state stored in `ncgame.db` for one
  game directory.
- *`nostr reissue`:* Generate a fresh invite code for one hosted seat, clear
  its old hosted player binding, preserve the existing runtime empire, and
  republish that game's public `30500` metadata when possible.
- *`nostr reissue --nuke-seat`:* Destructively reset one hosted seat back to a
  true pre-join year-3000 baseline, clear its hosted binding, and republish
  that game's public `30500` metadata when possible.
- *`nostr claim`:* Manually bind one hosted seat to a specific `npub` and
  republish that game's public `30500` metadata when possible.
- *`nostr publish`:* Republish one game's public `30500` metadata to the
  configured relay immediately.
- *`nostr verify`:* Compare one game's local hosted-seat state against the
  latest published `30500` on the configured relay.
- *`maint`:* Run one or more maintenance turns against `ncgame.db`.
- *`maint-all`:* Sweep every game registered in the gate config, skip games
  that are not due or that currently have active sessions, and advance only
  games whose schedule is enabled.

== nc-game and nc-door

```
nc-game --dir <game_dir> [--player <1-based index>] [options]
nc-door --dir <game_dir> --dropfile <path> [options]
nc-game submit-turn [--check] --dir <game_dir> --player <record> --file <turn.kdl>
```

Use `nc-game` for direct localhost and SSH/VPS sessions. Use `nc-door` for BBS
door launches. The interactive flags below apply to both binaries unless a host
setup guide says otherwise.

Interactive client flags:

- *`--dir <path>`:* Game directory containing `ncgame.db`. Required.
- *`--player <N>`:* 1-based empire index for direct localhost or SSH play.
  Normal BBS dropfile launches can omit it.
- *`--encoding <utf8|cp437>`:* Output encoding. Default: `utf8`. Use `cp437`
  for BBS or door mode.
- *`--color-mode <ansi16|256|truecolor|auto>`:* Color depth. Default: `auto`
  from the environment. CP437 mode defaults to `ansi16`.
- *`--dropfile <path>`:* Parse a BBS drop file (`DOOR32.SYS`, `DOOR.SYS`, or
  `CHAIN.TXT`). It supplies the alias and timeout, defaults encoding to
  `cp437`, and resolves the player seat through BBS `config.kdl` reservations
  or stored joined-player aliases.
- *`--socket-port <value>`:* Connect back to a localhost door socket. Mainly
  for ENiGMA½ `abracadabra` in `socket` mode.
- *`--socket-descriptor <value>`:* Native Windows door socket handle. Mainly
  for Synchronet-style socket door launches.
- *`--session-token <hex>`:* Hosted-session lease token injected by `nc-gate`
  during Nostr or SSH login. Normal local and BBS launches do not pass this
  flag.
- *`--timeout <minutes>`:* Session time limit in minutes. It overrides any
  drop file value.
- *`--export-root <path>`:* Optional map or export staging root for local or
  BBS file handoff workflows.
- *`--queue-dir <path>`:* Override turn queue directory. Default:
  `<game_dir>/queue`.

`submit-turn` flags:

- *`--check`:* Validate the KDL file without mutating the campaign.
- *`--dir <path>`:* Game directory containing `ncgame.db`. Required.
- *`--player <N>`:* 1-based empire index. Required, and it must match the KDL
  header.
- *`--file <path>`:* Turn submission KDL file to validate or apply. Required.

#admonition("NOTE")[`nc-game submit-turn` is all-or-nothing. If any command in the file is invalid, the entire submission is rejected and nothing is written to `ncgame.db`.]
