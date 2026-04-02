# Release Policy

Nostrian Conquest is in active beta. Public release downloads are intentionally
conservative until the Rust-hosted path has been proven in several real VPS
games.

## Current Beta Policy

| Audience | Public Download Today | Expected Install Path |
|---|---|---|
| Normal player | Windows x64, Linux x64, or macOS Apple Silicon `nc-connect` archive plus the player manual PDF | Download the matching public player archive from GitHub Releases |
| Rust self-host sysop | Linux x64 `nc-sysop` archive, or tagged source release | Use the sysop package for localhost/BBS, or `cargo build --release` |
| Rust VPS sysop | Tagged source release | `cargo build --release` plus `scripts/install_vps.sh` |
| Linux BBS sysop | Linux x64 `nc-sysop` archive, or tagged source release | Use the sysop package for localhost/BBS, or `cargo build --release` |
| Windows BBS sysop | Windows x64 `nc-sysop` archive when built on a native Windows host | Use the Windows sysop package, or `cargo build --release` |

Public GitHub Releases currently publish the Windows x64, Linux x64, and macOS
Apple Silicon `nc-connect` archives, plus the signed `SHA256SUMS.txt` manifest
for public Rust download verification. The same release tooling also supports
Linux x64 and Windows x64 `nc-sysop` localhost/BBS packages. VPS remains
Cargo/source-only.

## Target Stable Policy

After the hosted Rust path has been proven in real games:

| Audience | Planned Public Download |
|---|---|
| Normal player | `nc-connect` archive plus the player manual PDF |
| Rust self-host sysop | Linux x64 or Windows x64 `nc-sysop` archive, plus tagged source release |
| Rust VPS sysop | Tagged source release |
| BBS sysop | Linux x64 or Windows x64 `nc-sysop` archive with `nc-game`, `nc-sysop`, example `config.kdl`, player manual PDF, and sysop manual PDF |

Classic DOS bundles remain developer and compatibility artifacts. They are not
the normal public Rust onboarding path.
