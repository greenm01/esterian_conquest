# Esterian Conquest Manuals

_Inspired by Esterian Conquest (c) 1992 Bentley C. Griffith.
This is an independent reimplementation and is not affiliated with the original._

These are the reference manuals for players and operators.

They are based on readable Markdown transcriptions of the preserved Esterian
Conquest `v1.5` manuals, with light framing so they work as the current
documentation surface for the Rust continuation of the game.

The original `.DOC` files remain preserved in
[`original/v1.5`](../../original/v1.5) as the
source artifacts. These Markdown copies exist for easier reading, linking, and
quoting.

## Recommended Reading Order

- [ec_qstart.md](ec_qstart.md)
  - fastest way for a new player to understand the game loop
- [ec_player.md](ec_player.md)
  - main mechanics and command reference
- [ec_readme.md](ec_readme.md)
  - legacy DOS deployment and release context

## Source Mapping

| Markdown transcription | Original artifact | Purpose |
| --- | --- | --- |
| [ec_qstart.md](ec_qstart.md) | [ECQSTART.DOC](../../original/v1.5/ECQSTART.DOC) | Quick-start guide for new players |
| [ec_player.md](ec_player.md) | [ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC) | Full player manual |
| [ec_readme.md](ec_readme.md) | [ECREADME.DOC](../../original/v1.5/ECREADME.DOC) | Legacy DOS deployment and release context |

## Using EC Rust

- treat these manuals as the current gameplay reference unless a Rust-specific
  doc explicitly overrides a workflow
- treat the original `.DOC` files as the preserved source of truth behind the
  transcriptions
- treat Rust client/UI differences as interface changes, not automatic rules
  changes
