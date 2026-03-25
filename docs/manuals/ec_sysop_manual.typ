// Esterian Conquest — Sysop Manual
// Typst source — generates US Letter PDF

#set document(
  title: "Esterian Conquest — Sysop Manual",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 3, day: 25),
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
  #text(size: 28pt, weight: "bold")[Esterian Conquest]
  #v(0.5em)
  #text(size: 18pt)[Sysop Manual]
  #v(2em)
  #text(size: 12pt, fill: luma(80))[Copyright © 2026 Mason A. Green]
  #v(0.5em)
  #text(size: 11pt, fill: luma(120))[Draft — March 2026]
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

Esterian Conquest (EC) is a multi-player strategy game originally written for
DOS-era BBS systems. The Rust-native stack reimplements the game engine, player
client, and admin tooling for modern systems while preserving classic
compatibility at a well-defined import/export boundary.

This manual is for the *sysop* — the person responsible for hosting a game
instance. It covers:

- installing and initializing a game
- configuration and theming
- BBS door, SSH, and local deployment
- player management
- turn processing and maintenance

For player-facing rules and gameplay, see the *Player Manual*.

== Terminology

/ game directory: The directory containing `ecgame.db` and related files for a
  single running game instance. Passed to all tools via `--dir`.

/ `ec-sysop`: The public Rust command-line sysop tool for campaign creation
  and maintenance.

/ `ec-game`: The Rust TUI player client.

/ `ecgame.db`: The SQLite database that is the runtime source of truth for the
  Rust engine.

/ `theme.kdl`: The KDL file that controls the visual style of `ec-game` for
  all players in this game directory.

/ `config.kdl`: The sysop-managed runtime configuration file in the game
  directory. Bootstrapped from the bundled default on first run. Controls
  snoop mode, session timeouts, inactivity thresholds, game name, and theme
  path. Changes take effect on the next `ec-game` startup.

// ─── 2. Installation ──────────────────────────────────────────────────────────

= Installation and Requirements

== System Requirements

- Linux, macOS, or Windows
- Rust toolchain (stable) to build from source, or a pre-built binary release
- A terminal emulator for local/SSH play; a BBS server with socket support for
  door deployment

== Building from Source

From the repository root:

```
cargo build --release -p ec-sysop -p ec-game
```

The release binaries will be in `target/release/`.

== Directory Layout

A running game occupies a single directory. A minimal populated game directory
looks like:

```
/path/to/mygame/
  ecgame.db          runtime database (SQLite)
  config.kdl         sysop runtime config (bootstrapped on first run)
  theme.kdl          visual theme (bootstrapped on first run)
  exports/           default export root for classic .DAT output
  queue/             default turn order queue directory
```

All tools take `--dir /path/to/mygame` to locate the game.

// ─── 3. Game Setup ────────────────────────────────────────────────────────────

= Game Setup

== Initializing a New Game

Create a new game with `ec-sysop new-game`:

```
ec-sysop new-game /path/to/mygame --players 4 --seed 1515
```

This creates a fresh campaign directory with `ecgame.db`, classic auxiliary
files, `config.kdl`, and `theme.kdl`.

`config.kdl` is the only sysop-edited KDL file in the public workflow.
An internal `ec-cli` setup-preset format still exists for reproducible tests
and harness work, but normal sysop operation does not use it.

The supported public creation flags are:

#table(
  columns: (auto, auto, 1fr),
  [*Flag*], [*Type*], [*Description*],
  [`--players`], [integer], [Number of empires. Supported range: 1–25. Defaults to `4`.],
  [`--seed`], [integer], [Optional campaign seed for reproducible map generation.],
)

// ─── 4. Game Directory Structure ─────────────────────────────────────────────

= Game Directory Structure

== Core Files

/ `ecgame.db`: The SQLite runtime database. All game state lives here. Do not
  edit manually; use `ec-sysop` for normal operator actions.

/ `theme.kdl`: Visual theme for `ec-game`. Bootstrapped from the bundled
  default on first run if absent. Sysop-owned once created; not silently
  overwritten. See @theming.

/ `config.kdl`: Sysop runtime configuration. Bootstrapped from the bundled
  default on first run. Edit to change snoop mode, session timeouts,
  inactivity thresholds, game name, or theme path. See @configuration.

== Subdirectories

/ `exports/`: Default root for classic `.DAT` export output. Can be overridden
  with `--export-root` or the `EC_CLIENT_EXPORT_ROOT` environment variable.

/ `queue/`: Default directory for turn order queue files. Can be overridden
  with `--queue-dir` or the `EC_CLIENT_QUEUE_DIR` environment variable.

// ─── 5. Configuration ─────────────────────────────────────────────────────────

= Configuration <configuration>

== config.kdl

`config.kdl` is the sysop runtime configuration file. It lives in the game
directory alongside `ecgame.db`. If absent when `ec-game` starts, it is
bootstrapped from the bundled default automatically.

Changes to `config.kdl` take effect on the next `ec-game` startup. The
settings are applied into the runtime database at that point; no manual
database edits are required.

=== Full Example

```kdl
// Display name shown in the main menu header.
game_name "Esterian Conquest"

// Theme file (relative to game directory, or absolute path).
// Remove this line to use theme.kdl in the same directory.
// theme "theme.kdl"

// Sysop snoop: set to #false to disable.
snoop #true

// Session timeout and timing policies.
session {
    // Minutes of inactivity before timeout (0–120).
    max_idle_minutes 10
    // Minimum time granted per session in minutes (0–120).
    minimum_time_minutes 0
    // Apply timeout to local (non-remote) sessions.
    local_timeout #false
    // Apply timeout to remote sessions.
    remote_timeout #true
}

// Inactivity thresholds (in turns). Set to 0 to disable.
inactivity {
    // Purge a player after this many inactive turns (0–100).
    purge_after_turns 0
    // Put a player on autopilot after this many inactive turns (0–100).
    autopilot_after_turns 0
}
```

=== Top-Level Fields

#table(
  columns: (auto, auto, auto, 1fr),
  [*Field*], [*Type*], [*Default*], [*Description*],
  [`game_name`], [string], [`"Esterian Conquest"`], [Display name shown in the main menu header.],
  [`theme`], [string], [_(absent)_], [Theme file path, relative to the game directory. Omit to use `theme.kdl`.],
  [`snoop`], [bool], [`#true`], [Enable sysop snoop mode.],
)

=== `session` Block

#table(
  columns: (auto, auto, auto, 1fr),
  [*Field*], [*Type*], [*Default*], [*Description*],
  [`max_idle_minutes`], [integer], [`10`], [Minutes of inactivity before session timeout. Range: 0–120.],
  [`minimum_time_minutes`], [integer], [`0`], [Minimum session time guarantee in minutes. Range: 0–120.],
  [`local_timeout`], [bool], [`#false`], [Apply timeout to local (non-remote) sessions.],
  [`remote_timeout`], [bool], [`#true`], [Apply timeout to remote sessions.],
)

=== `inactivity` Block

#table(
  columns: (auto, auto, auto, 1fr),
  [*Field*], [*Type*], [*Default*], [*Description*],
  [`purge_after_turns`], [integer], [`0`], [Turns of inactivity before a player is purged. `0` = disabled. Range: 0–100.],
  [`autopilot_after_turns`], [integer], [`0`], [Turns of inactivity before autopilot engages. `0` = disabled. Range: 0–100.],
)

#admonition("NOTE")[
  `config.kdl` is for sysop use only. Players do not interact with it.
  Fields not present in the file use their default values. Omitting a field
  is equivalent to setting it to its default.
]

== Environment Variables

#table(
  columns: (auto, 1fr),
  [*Variable*], [*Description*],
  [`EC_CLIENT_EXPORT_ROOT`], [Override the default export root directory.],
  [`EC_CLIENT_QUEUE_DIR`], [Override the default turn queue directory.],
  [`COLORTERM`], [Color depth hint. `truecolor` or `24bit` enables 24-bit color.],
  [`TERM`], [Terminal type. A value containing `256color` enables 256-color mode.],
)

// ─── 6. Theming ───────────────────────────────────────────────────────────────

= Theming <theming>

`ec-game` uses a file-driven theme system. The theme defines the visual
appearance for all players connecting to this game directory.

== Theme File Location

`ec-game` resolves the theme in this order:

1. If `<game_dir>/config.kdl` contains a `theme` directive, use that path
   (relative to `game_dir`).
2. Otherwise, use `<game_dir>/theme.kdl`.
3. If `theme.kdl` does not exist, bootstrap it from the bundled default.

`config.kdl` itself is bootstrapped on first run if absent, so it is always
present by the time this resolution runs. The `theme` directive within it is
optional; omitting it falls through to step 2.

On parse error, the bundled default is used silently so a corrupted theme never
prevents players from connecting.

== theme.kdl Format

A theme file is a KDL document. Each visual element is declared as a `style`
node with a name and child `fg`, `bg`, and optional `bold` fields.

```kdl
style "body" {
    fg "white"
    bg "black"
}

style "logo" {
    fg "bright_blue"
    bg "black"
    bold #true
}

style "selected" {
    fg "black"
    bg "bright_blue"
}
```

The star decoration colors are declared separately:

```kdl
star-colors "bright_blue" "bright_white" "white" "bright_yellow" "yellow" "bright_red"
```

== Color Formats

Three color formats are supported:

#table(
  columns: (auto, auto, 1fr),
  [*Format*], [*Example*], [*Notes*],
  [Named ANSI-16], [`"bright_blue"`], [Safe for all terminals including BBS/door clients. Recommended default.],
  [256-color index], [`"idx:208"`], [Requires `--color-mode 256` or truecolor. Downgraded to nearest named color in 16-color mode.],
  [24-bit hex RGB], [`"#ff8800"`], [Requires `--color-mode truecolor`. Downgraded gracefully in lower color modes.],
)

The bundled default theme uses named ANSI-16 colors only. Custom themes may use
richer formats when targeting modern terminals.

== Color Mode

`ec-game` selects a color mode at startup:

#table(
  columns: (auto, 1fr),
  [*Mode*], [*Description*],
  [`ansi16`], [Classic 16-color ANSI. Safe for BBS/door and all terminal types.],
  [`256`], [256-color xterm palette.],
  [`truecolor`], [24-bit RGB. Best for modern local or SSH terminals.],
  [`auto`], [Detected from `COLORTERM` and `TERM` environment variables.],
)

Override with the `--color-mode` flag:

```
ec-game --dir /path/to/mygame --player 1 --color-mode truecolor
```

When `--encoding cp437` is specified (BBS door mode), `ec-game` defaults to
`ansi16` regardless of the environment, unless `--color-mode` is set explicitly.

== Semantic Style Tokens

All required style tokens:

#table(
  columns: (auto, 1fr),
  [*Token*], [*Purpose*],
  [`body`], [Default body text.],
  [`title`], [Screen and section titles.],
  [`menu`], [Menu item text.],
  [`menu_hotkey`], [Menu hotkey letters.],
  [`prompt`], [Input prompt text.],
  [`prompt_hotkey`], [Prompt hotkey letters.],
  [`prompt_notice_action`], [Action-notice accent in prompts.],
  [`bright`], [Emphasized body text.],
  [`logo`], [Title logo decoration.],
  [`intro_accent`], [Intro screen accent color.],
  [`intro_tribute`], [Intro screen tribute line.],
  [`stardate_label`], [Stardate label.],
  [`stardate_week`], [Stardate week value.],
  [`stardate_year`], [Stardate year value.],
  [`error`], [Error messages.],
  [`notice`], [Warning / notice messages.],
  [`status_label`], [Status bar labels.],
  [`status_value`], [Status bar values.],
  [`table_chrome`], [Table borders and separators.],
  [`table_header`], [Table column headers.],
  [`table_body`], [Table body rows.],
  [`disabled_row`], [Disabled / unavailable table rows.],
  [`selected`], [Selected / highlighted row.],
  [`alert`], [High-priority alert text.],
  [`help_header`], [Help overlay section headers.],
  [`help_panel`], [Help overlay body text.],
  [`map_dot`], [Star map background dots.],
  [`map_crosshair`], [Star map crosshair / cursor.],
  [`map_center`], [Star map center marker.],
  [`quote`], [Quote / flavor text body.],
  [`quote_author`], [Quote attribution.],
  [`report_header`], [Report page headers.],
  [`indicator_on`], [Active indicator (e.g. status lit).],
  [`indicator_off`], [Inactive indicator.],
)

== ANSI Off Mode

Players can toggle ANSI color off within a session (a session-level preference
only — it does not modify `theme.kdl`). ANSI Off applies a monochrome
projection over the loaded theme: white/black with preserved reverse-video
selection. A new session always starts with ANSI On.

// ─── 7. BBS Door Setup ────────────────────────────────────────────────────────

= BBS Door Setup

`ec-game` can run as a BBS door. In door mode it reads from and writes to
a socket instead of the local TTY, using CP437 encoding and 16-color ANSI.

== Flags for Door Mode

```
ec-game \
  --dir /path/to/mygame \
  --player <1-based index> \
  --encoding cp437 \
  --color-mode ansi16
```

`--encoding cp437` switches box-drawing characters and other extended
characters to the CP437 code page, which is required for correct rendering
in classic ANSI-aware BBS terminals (SyncTERM, NetRunner, etc.).

When `--encoding cp437` is active, `--color-mode` defaults to `ansi16`
automatically. Override only if you know your BBS clients support a richer
color depth.

== Drop File

`ec-game` can read a BBS drop file directly with `--dropfile <path>`:

```
ec-game \
  --dir /path/to/mygame \
  --player <1-based index> \
  --dropfile /path/to/DOOR32.SYS
```

Supported drop file formats (auto-detected by filename, case-insensitive):

- `DOOR32.SYS` — modern standard (Enigma, Mystic, Talisman, etc.)
- `DOOR.SYS` — legacy, widest BBS software support
- `CHAIN.TXT` — WWIV format

The drop file supplies the player alias and session time limit. Explicit CLI
flags always override drop file values. When `--dropfile` is given and
`--encoding` is not, encoding defaults to `cp437` automatically.

`--timeout <minutes>` sets a session time limit independently of a drop file.

#admonition("NOTE")[
  `--dropfile` currently supplies the alias and timeout only. The `--player`
  flag is still required; alias-to-empire-index resolution is a planned
  follow-up.
]

#admonition("NOTE")[
  The original DOS `ECGAME.EXE` v1.5 expects a strict 32-line WWIV-style
  `CHAIN.TXT` drop file. Enigma BBS-generated `DOOR.SYS` / `DORINFO` files
  will crash `ECGAME.EXE`. See `docs/sysop/enigma-bbs-setup.md` for the full
  legacy DOS door path if you need to host the original binary.
]

== Enigma BBS (Rust Client)

To run the native `ec-game` as an Enigma BBS door, use the `abracadabra`
module with `io: socket` and pass `--dir`, `--player`, `--encoding cp437`, and
`--color-mode ansi16` as arguments. The client will inherit the socket from
Enigma's stdio handoff.

If Enigma writes a `DOOR32.SYS`, you can pass it directly:

```
ec-game \
  --dir /path/to/mygame \
  --player <1-based index> \
  --dropfile /path/to/DOOR32.SYS
```

// ─── 8. SSH Access ────────────────────────────────────────────────────────────

= SSH Access

`ec-game` runs cleanly over SSH. No special flags are required for SSH
sessions on modern terminals.

Color mode is auto-detected from the environment:

- `COLORTERM=truecolor` → 24-bit RGB
- `TERM` containing `256color` → 256-color
- Otherwise → 16-color ANSI fallback

Force a specific mode with `--color-mode` if your SSH setup does not propagate
`COLORTERM` reliably:

```
ec-game --dir /path/to/mygame --player 1 --color-mode 256
```

UTF-8 encoding (the default) is correct for SSH sessions on modern terminals.
Use `--encoding cp437` only if you are proxying through a BBS or a CP437
terminal emulator over SSH.

// ─── 9. Local / Direct Play ───────────────────────────────────────────────────

= Local and Direct Play

For localhost play, run `ec-game` directly in your terminal:

```
ec-game --dir /path/to/mygame --player 1
```

Color mode and encoding default to `auto` / `utf8`, which is correct for
modern terminal emulators. The client detects `COLORTERM` and `TERM`
automatically.

// ─── 10. File-Based Turn Submission ──────────────────────────────────────────

= File-Based Turn Submission

Players on localhost or a shared host can use either the interactive TUI or `ec-game submit-turn`.

```
ec-game submit-turn --check --dir /path/to/mygame --player 1 --file /path/to/player1-turn.kdl
ec-game submit-turn --dir /path/to/mygame --player 1 --file /path/to/player1-turn.kdl
```

This is a supported file-based interface for manual workflows and custom client integration. It writes directly to `ecgame.db` after the submission validates cleanly. If any command in the file is invalid, the entire submission is rejected and nothing is written.

Do not treat this as a queue processor or upload inbox. `submit-turn` is a direct command that reads one KDL file and applies it immediately to the live campaign state.

// ─── 11. Turn Processing and Maintenance ─────────────────────────────────────

= Turn Processing and Maintenance

Run yearly maintenance with:

```
ec-sysop maint /path/to/mygame [turns]
```

`ec-sysop maint` advances the campaign in `ecgame.db`. EC does not schedule
maintenance by itself. In a real deployment, invoke `ec-sysop maint` from your
host scheduler or BBS tooling:

- a `systemd` timer
- `cron`
- a BBS event runner
- or manual sysop operation

Do not treat KDL config as a scheduler. `config.kdl` owns runtime policy such
as theming, snoop, and inactivity thresholds. Maintenance timing belongs to
the host.

// ─── 12. Player Management ────────────────────────────────────────────────────

= Player Management

Inactive-player policy is configured in `config.kdl` under the `inactivity`
block. The two public thresholds are:

- `purge_after_turns`
- `autopilot_after_turns`

These values are runtime policy, not setup-time game creation input.

// ─── 13. Classic Compatibility ────────────────────────────────────────────────

= Classic Compatibility

The Rust-native public deployment path is `ec-sysop` plus `ec-game`.

Classic `.DAT` import/export, oracle runs against the original binaries, and
other preservation workflows still exist, but they belong to the internal
`ec-cli` developer/compatibility surface rather than the normal sysop manual.

// ─── 14. CLI Reference ────────────────────────────────────────────────────────

= CLI Reference

== ec-sysop

```
ec-sysop <subcommand> [options]
```

#table(
  columns: (auto, 1fr),
  [*Subcommand*], [*Purpose*],
  [`new-game`], [Create a fresh campaign directory. Public flags: `--players` and `--seed`.],
  [`maint`], [Run one or more maintenance turns against `ecgame.db`.],
)

== ec-game

```
ec-game --dir <game_dir> --player <1-based index> [options]
ec-game submit-turn [--check] --dir <game_dir> --player <record> --file <turn.kdl>
```

Interactive client flags:

#table(
  columns: (auto, 1fr),
  [*Flag*], [*Description*],
  [`--dir <path>`], [Game directory containing `ecgame.db`. Required.],
  [`--player <N>`], [1-based empire index. Required.],
  [`--encoding <utf8|cp437>`], [Output encoding. Default: `utf8`. Use `cp437` for BBS/door mode.],
  [`--color-mode <ansi16|256|truecolor|auto>`], [Color depth. Default: `auto` (env-detected). CP437 mode defaults to `ansi16`.],
  [`--dropfile <path>`], [Parse a BBS drop file (DOOR32.SYS, DOOR.SYS, or CHAIN.TXT). Supplies alias and timeout. Defaults encoding to `cp437`. Explicit flags always override.],
  [`--timeout <minutes>`], [Session time limit in minutes. Overrides any drop file value.],
  [`--export-root <path>`], [Override export directory. Default: `<game_dir>/exports`.],
  [`--queue-dir <path>`], [Override turn queue directory. Default: `<game_dir>/queue`.],
)

`submit-turn` flags:

#table(
  columns: (auto, 1fr),
  [*Flag*], [*Description*],
  [`--check`], [Validate the KDL file without mutating the campaign.],
  [`--dir <path>`], [Game directory containing `ecgame.db`. Required.],
  [`--player <N>`], [1-based empire index. Required, and must match the KDL header.],
  [`--file <path>`], [Turn submission KDL file to validate or apply. Required.],
)

#admonition("NOTE")[`ec-game submit-turn` is all-or-nothing. If any command in the file is invalid, the entire submission is rejected and nothing is written to `ecgame.db`.]

// ─── 15. Theme Token Reference ────────────────────────────────────────────────

= Theme Token Reference

The default bundled theme values are listed below for reference. All colors use
named ANSI-16 values.

#table(
  columns: (auto, auto, auto, auto),
  [*Token*], [*fg*], [*bg*], [*bold*],
  [`body`], [`white`], [`black`], [—],
  [`title`], [`bright_blue`], [`black`], [yes],
  [`menu`], [`white`], [`black`], [—],
  [`menu_hotkey`], [`yellow`], [`black`], [yes],
  [`prompt`], [`white`], [`black`], [—],
  [`prompt_hotkey`], [`yellow`], [`black`], [yes],
  [`prompt_notice_action`], [`bright_cyan`], [`black`], [yes],
  [`bright`], [`bright_white`], [`black`], [yes],
  [`logo`], [`bright_blue`], [`black`], [yes],
  [`intro_accent`], [`bright_blue`], [`black`], [—],
  [`intro_tribute`], [`bright_magenta`], [`black`], [—],
  [`stardate_label`], [`cyan`], [`black`], [—],
  [`stardate_week`], [`bright_cyan`], [`black`], [—],
  [`stardate_year`], [`yellow`], [`black`], [—],
  [`error`], [`red`], [`black`], [yes],
  [`notice`], [`bright_red`], [`black`], [yes],
  [`status_label`], [`white`], [`black`], [—],
  [`status_value`], [`white`], [`black`], [—],
  [`table_chrome`], [`blue`], [`black`], [—],
  [`table_header`], [`cyan`], [`black`], [yes],
  [`table_body`], [`bright_white`], [`black`], [—],
  [`disabled_row`], [`bright_black`], [`black`], [—],
  [`selected`], [`black`], [`bright_blue`], [—],
  [`alert`], [`bright_white`], [`red`], [yes],
  [`help_header`], [`bright_blue`], [`black`], [yes],
  [`help_panel`], [`white`], [`black`], [—],
  [`map_dot`], [`green`], [`black`], [—],
  [`map_crosshair`], [`bright_red`], [`black`], [yes],
  [`map_center`], [`bright_white`], [`black`], [yes],
  [`quote`], [`white`], [`black`], [—],
  [`quote_author`], [`white`], [`black`], [—],
  [`report_header`], [`cyan`], [`black`], [—],
  [`indicator_on`], [`bright_green`], [`black`], [yes],
  [`indicator_off`], [`bright_black`], [`black`], [—],
)

Star decoration colors (6 slots, cycling):

```kdl
star-colors "bright_blue" "bright_white" "white" "bright_yellow" "yellow" "bright_red"
```
