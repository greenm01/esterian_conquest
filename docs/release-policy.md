# Release Policy

Nostrian Conquest is in active beta. Public release downloads are intentionally
conservative until the Rust-hosted path has been proven in several real VPS
games.

## Current Beta Policy

| Audience | Public Download Today | Expected Install Path |
|---|---|---|
| Normal player | Windows x64, Linux x64, or macOS Apple Silicon `ec-connect` archive plus the player manual PDF | Download the matching public player archive from GitHub Releases |
| Rust self-host sysop | Tagged source release | `cargo build --release` |
| Rust VPS sysop | Tagged source release | `cargo build --release` plus `scripts/install_vps.sh` |
| BBS sysop | No public Rust door package yet | Build from source, or use a direct/private beta build |
| Windows BBS sysop | No public package | Best-effort source build only |

Public GitHub Releases currently publish the Windows x64, Linux x64, and macOS
Apple Silicon `ec-connect` archives, plus the signed `SHA256SUMS.txt` manifest
for public player verification.

## Target Stable Policy

After the hosted Rust path has been proven in real games:

| Audience | Planned Public Download |
|---|---|
| Normal player | `ec-connect` archive plus the player manual PDF |
| Rust self-host sysop | Tagged source release |
| Rust VPS sysop | Tagged source release |
| BBS sysop | Linux x64 door package with `ec-game`, `ec-sysop`, BBS docs, player manual PDF, and sysop manual PDF |
| Windows BBS sysop | Still best-effort source build unless promoted later |

Classic DOS bundles remain developer and compatibility artifacts. They are not
the normal public Rust onboarding path.
