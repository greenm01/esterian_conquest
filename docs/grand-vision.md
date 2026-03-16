# Grand Vision: From BBS to the Decentralized Web

This document outlines the macro trajectory for the Esterian Conquest reimplementation. The project is split into two distinct evolutionary phases, bound together by a universal, faithful game engine.

## Phase 1: v1.6 - The Ultimate BBS Door

Our immediate goal is to build the definitive, modern-host-friendly drop-in replacement for the original DOS game.

- **Faithful Recreation:** The mechanics, turn processing, and underlying `.DAT` files remain 100% compatible with the original DOS release. The engine (`ec-maint`) acts as a "Rosetta Stone," completely capable of passing as the original oracle.
- **Classic Feel, Better UX:** The UI is driven by pure `crossterm` to enforce the rigid 80x25 CP437 display limits necessary for Telnet BBS integration. We preserve the original menus and ANSI artwork, but modernize the interaction loop by replacing annoying scrolling terminal inputs with clean, fixed-viewport data tables.
- **The Deployment:** Sysops can drop this Rust binary into their Enigma BBS or Synchronet setup natively on Linux/macOS/Windows, without the overhead and instability of DOSBox.

## Phase 2: v2.0 - The Nostr Era

Once the core engine is fully reverse-engineered and the mechanics are crystallized in Rust, we will decouple the client and server to bring Esterian Conquest into the decentralized web.

- **Nostr as the Transport Layer:** The asynchronous, daily-turn nature of EC maps perfectly to the Nostr protocol. Instead of logging into a centralized BBS, players use their `secp256k1` keys to submit encrypted turn orders to relays.
- **Headless Server & Encrypted State:** The `ec-maint` engine runs headlessly on a server, collecting daily orders. When maintenance runs, it generates the new game state for each player and broadcasts NIP-04/NIP-44 encrypted events back to the relay, preserving the "fog of war."
- **Modern TUI Client:** Freed from the constraints of 80x25 Telnet screens, the v2.0 client will use `Ratatui` to provide a highly responsive, modern, and beautiful terminal interface that can run natively on the player's local machine. 

## Beyond Classic: EC4X

This repository (`esterian_conquest`) is strictly dedicated to preserving and faithfully modernizing the *original* game's exact mechanics. It will never drift from the classic ruleset.

However, if you are looking for a game that takes these foundational mechanics and expands them, check out my sister project: **[ec4x](https://github.com/greenm01/ec4x)**.

Where `esterian_conquest` is the faithful preservation of a classic in Rust, `ec4x` is essentially "Esterian Conquest on steroids." Written in **Nim** with a **hand-rolled TUI**, it completely overhauls the game design, expanding it into a much deeper, modernized 4X strategy experience.
