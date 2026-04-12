# Grand Vision: From BBS to the Decentralized Web

This document outlines the macro trajectory for the Esterian Conquest reimplementation. The project is split into two distinct evolutionary phases, bound together by a universal, faithful game engine.

## Phase 1: The Ultimate BBS Door

Our immediate goal is to build the definitive, modern-host-friendly drop-in replacement for the original DOS game.

- **Faithful Recreation:** The mechanics, turn processing, and underlying `.DAT` files remain 100% compatible with the original DOS release. The engine (`ec-maint`) acts as a "Rosetta Stone," completely capable of passing as the original oracle.
- **Classic Feel, Better UX:** The UI is driven by pure `crossterm` to enforce the rigid 80x25 CP437 display limits necessary for Telnet BBS integration. We preserve the original menus and ANSI artwork, but modernize the interaction loop by replacing annoying scrolling terminal inputs with clean, fixed-viewport data tables.
- **The Deployment:** Sysops can drop this Rust binary into their Enigma BBS or Synchronet setup natively on Linux/macOS/Windows, without the overhead and instability of DOSBox.

## Phase 2: Decentralized Nostr Multiplayer

The current shipped gameplay stack is localhost plus BBS door play. The old
SSH/Nostr path is no longer the live product direction. The next hosted step is
instead a cleaner relay-native design built around `nc-host` and `nc-dash`:

- **Nostr as the Full Transport Layer:** The asynchronous, daily-turn nature of EC maps perfectly to the Nostr protocol. Instead of bridging a remote PTY over SSH, players submit encrypted turn orders directly to relays, and `ec-maint` publishes per-player results back as encrypted events, preserving the "fog of war."
- **Headless Server:** the engine runs on a schedule, collects daily orders from
  the relay, processes maintenance, and broadcasts new game state to each
  player's key.
- **Local Native Client:** `nc-dash` grows a lobby and hosted-play mode that
  renders game state locally instead of bridging a remote PTY over SSH.

## Beyond Classic: EC4X

This repository (`esterian_conquest`) is strictly dedicated to preserving and faithfully modernizing the original game's exact mechanics. It will never drift from the classic ruleset.

However, if you are looking for a game that takes these foundational mechanics and expands them, check out my sister project: **[ec4x](https://github.com/greenm01/ec4x)**.

Where `esterian_conquest` is the faithful preservation of a classic in Rust, `ec4x` is essentially "Esterian Conquest on steroids." Written in **Nim** with a **hand-rolled TUI**, it completely overhauls the game design, expanding it into a much deeper, modernized 4X strategy experience.
