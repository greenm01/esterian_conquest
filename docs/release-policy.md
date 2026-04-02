# Release Policy

Nostrian Conquest is in active beta. Public release downloads are intentionally
conservative until the Rust-hosted path has been proven in several real VPS
games.

## Current Binary Roles

The public Rust stack uses three binaries with fixed roles.

`nc-connect` is the packaged player client for the recommended Nostr flow.
`nc-game` is the direct interactive client for localhost sessions and the
host-side SSH/VPS session binary. `nc-door` is the BBS door entrypoint on both
Windows and Linux. `nc-sysop` is the administrator's tool for creating games,
editing settings, running maintenance, and operating hosted campaigns.

## Current Beta Policy

Public GitHub Releases currently publish the Windows x64, Linux x64, and macOS
Apple Silicon `nc-connect` player archives, together with the player manual PDF
and the signed `SHA256SUMS.txt` manifest.

Public GitHub Releases also publish Windows x64 and Linux x64 `nc-sysop`
archives for BBS and sysop use. Those archives are the public BBS/sysop
packages. They are expected to carry `nc-door`, `nc-sysop`, the sysop manual,
the player manual, and the minimal example campaign files needed for a normal
door host handoff.

Localhost play is supported on Windows, Linux, and macOS, but it is currently a
source-build workflow. In that mode, the sysop builds `nc-game` and `nc-sysop`
from tagged source and runs `nc-game` directly in a local terminal session.

Rust VPS hosting also remains source-build only. The supported path is a Linux
host with `nc-game` and `nc-sysop`, installed from tagged source and staged
with `scripts/install_vps.sh`.

In short, the current beta contract is:

1. Players use the public `nc-connect` package on Windows, Linux, or macOS for
   the recommended Nostr path.
2. BBS sysops use the public Windows or Linux `nc-sysop` package, whose BBS
   entrypoint is `nc-door`.
3. Localhost sysops build from source and run `nc-game` plus `nc-sysop`.
4. VPS sysops build from source on Linux and run `nc-game` plus `nc-sysop`.

## Target Stable Policy

After the hosted Rust path has been proven in real games:

The normal public player download remains the `nc-connect` archive plus the
player manual PDF.

The normal public BBS/sysop download remains the Windows x64 or Linux x64
`nc-sysop` archive. Its public contract is the same as the beta contract: ship
`nc-door` as the BBS binary and `nc-sysop` as the administrator's tool.

Localhost and VPS workflows may gain additional packaging later, but they are
not part of the current public package promise. Until that changes, localhost
and VPS remain documented source-build paths.

Classic DOS bundles remain developer and compatibility artifacts. They are not
the normal public Rust onboarding path.
