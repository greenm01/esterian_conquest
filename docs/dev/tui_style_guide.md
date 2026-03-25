# `ec-client` TUI Style Guide

This document defines the visual and color standard for the Rust player TUI.

It exists to stop ad hoc color drift and to make the client theme portable
across Linux terminal emulators, macOS terminals, Windows terminals, SyncTERM,
and telnet-style ANSI clients.

Read it together with:

- [bbs_door_client_rust.md](bbs_door_client_rust.md)
- [ec-client-table-standard.md](ec-client-table-standard.md)

## Core Rules

- `ec-client` uses a fixed `80x25` playfield.
- The TUI color standard is ANSI 16-color, not truecolor.
- Interactive rendering should go through `crossterm` end-to-end.
- The whole TUI uses a black background.
- The default look is restrained and Tokyo-Night-inspired, but it is expressed
  through ANSI-safe semantic styles.
- Theme configuration is file-driven and lives in the game directory alongside
  `ecgame.db`:
  - `<game_dir>/theme.kdl` — the theme file, created from the bundled default
    on first run if absent
  - `<game_dir>/config.kdl` — optional sysop config; if present and contains a
    `theme` directive, that path (relative to `game_dir`) is used instead of
    `theme.kdl`
- On first run, `ec-client` bootstraps the default `theme.kdl` into the game
  directory if it is missing.
- Once created, `theme.kdl` is sysop-owned and is not silently overwritten.

## Semantic Style Tokens

The theme KDL should define semantic styles rather than screen-specific hacks.

Required tokens:

- `body`
- `title`
- `menu`
- `menu_hotkey`
- `prompt`
- `prompt_hotkey`
- `prompt_notice_action`
- `bright`
- `logo`
- `intro_accent`
- `intro_tribute`
- `stardate_label`
- `stardate_week`
- `stardate_year`
- `status_label`
- `status_value`
- `table_chrome`
- `table_header`
- `table_body`
- `disabled_row`
- `selected`
- `alert`
- `help_header`
- `help_panel`
- `map_dot`
- `map_crosshair`
- `map_center`
- `quote`
- `quote_author`
- `report_header`

Decorative star/logo accent colors may also be configured.

## Standard Palette Intent

Default semantic mapping:

| Element | Default ANSI intent | Notes |
| --- | --- | --- |
| Main body text, help text, reports, long descriptions | bright black | Calm, readable, log-like |
| Menu titles / section headers | bright blue | Clear hierarchy without harsh inverse bars |
| Intro logo | bright blue | ANSI 12 by default |
| Hotkeys / command letters | yellow | High visibility for actions |
| Active input / live cursor text | bright cyan | Interactive accent |
| Table headers | bright blue | Structured data label color |
| Table borders / separators | white | Lighter than body text, still subdued |
| Good / bad / info status values | bright green / bright red / bright cyan | Sparse emphasis only |
| Quote text / attribution | bright black / white | Decorative, low priority |
| Rare major alert | bright white on red or bright magenta | Reserved for high-signal moments |

## Prompt and Cursor Rules

- Live prompts must leave a space after `-> ` before the cursor.
- Prompt brackets keep neutral prompt coloring:
  - `<` and `>`
  - `[` and `]`
- The inner hotkey text renders in yellow.
- Menu command lines and table command lines should share the same prompt
  grammar and color treatment.

## Theme File Notes

The default bundled theme should be bootstrapped into the game directory on
first run. The bundled file is the fallback if the theme file is missing or
invalid, but invalid user files should not be rewritten automatically.

ANSI ON/OFF is a session-level terminal preference in the player TUI. It should
not rewrite `theme.kdl`; ANSI OFF should apply a monochrome projection over the
loaded theme for the current session while preserving black backgrounds and
reverse-video selection. A new client session should default back to ANSI ON.

The theme file supports three color formats:

- Named ANSI-16 colors (e.g. `bright_blue`, `yellow`, `bright_black`)
- 256-color palette index: `idx:N` or `index:N` (e.g. `idx:208`)
- 24-bit hex RGB: `#RRGGBB` (e.g. `#ff8800`)

The default bundled theme uses named ANSI-16 colors only, which are safe across
all supported terminal types including BBS/door clients. Sysops may use richer
color formats in custom themes when targeting modern terminals.

Supported named ANSI-16 values:

- `black`
- `bright_black`
- `red`
- `bright_red`
- `green`
- `bright_green`
- `yellow`
- `bright_yellow`
- `blue`
- `bright_blue`
- `magenta`
- `bright_magenta`
- `cyan`
- `bright_cyan`
- `white`
- `bright_white`
