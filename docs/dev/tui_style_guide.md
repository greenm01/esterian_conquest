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
- Theme configuration is file-driven:
  - Linux: `~/.config/ec-rust/theme.kdl`
  - Windows: platform-standard user config directory under `ec-rust`
  - macOS: platform-standard user config/app-support directory under `ec-rust`
- On first run, `ec-client` should create the default `theme.kdl` if it is
  missing.
- Once created, that file is user-owned and should not be silently overwritten.

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

Decorative star/logo accent colors may also be configured, but they should
still come from the same ANSI-16 palette.

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

The default bundled theme should be copied to the user config path on first
run. The bundled file is the fallback if the user file is missing or invalid,
but invalid user files should not be rewritten automatically.

ANSI ON/OFF is a session-level terminal preference in the player TUI. It should
not rewrite `theme.kdl`; ANSI OFF should apply a monochrome projection over the
loaded theme for the current session while preserving black backgrounds and
reverse-video selection. A new client session should default back to ANSI ON.

The theme file should use ANSI-16 color names only, such as:

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
