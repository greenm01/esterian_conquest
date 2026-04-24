# Nostr Notes

This directory is no longer the current shipped gameplay path.

The active product today is:

- `nc-game` for direct localhost play
- `nc-door` for BBS hosting
- `nc-sysop` for campaign creation, settings, reservations, and maintenance

The older SSH/Nostr hosted stack is intentionally out of the active release
story. The docs in this directory now serve two purposes:

- legacy reference for the retired `nc-connect` / `nc-gate` design
- forward-looking design notes for a future relay-native hosted stack

The active localhost/BBS runtime database no longer carries the retired hosted
seat, publish-job, or session-lease tables. Any future hosted stack should use
its own storage boundary instead of reusing the old `ncgame.db` schema.

If you are looking for current operator guidance, use these instead:

- [NC Sysop Manual](../manuals/nc_sysop_manual.typ)
- [Release Policy](../release-policy.md)
- [README](../../README.md)

If you are looking for future hosted direction, start with:

- [architecture-v2.md](architecture-v2.md)
- [protocol.md](protocol.md)
- [../helm/lobby-architecture.md](../helm/lobby-architecture.md)

Those future-hosted docs now treat encrypted direct community chat (`30518`) as
the canonical lobby/community communication surface, while anonymous per-game
`30523` diplomacy is an in-game surface rather than a lobby thread surface.

Treat the remaining Nostr documents as non-current unless a file explicitly says
otherwise.
