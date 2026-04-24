# Scripts

This directory holds repeatable local test-game setup helpers for the
SQLite-native Rust workflow.

These scripts are intended for:

- creating fresh joinable test games quickly
- creating richer stress-test campaigns for UI work
- creating classic-probe campaigns that open in original `ECGAME` with a busy
  player-1 report backlog
- bootstrapping a VPS host for DB-only Rust campaigns
- launching `nc-game` against a chosen campaign and player seat
- building standalone release bundles for playtesting

The current boundary is:

- `nc-game` runs from `ncgame.db`
- `maint-rust` runs from `ncgame.db`
- `nc-cli` is the setup and classic `.DAT` bridge/tooling surface

So these scripts call `nc-cli` to build or mutate campaigns, then launch
`nc-game` directly.

## Prerequisites

Run all examples from the repo root:

```bash
cd /path/to/esterian_conquest
```

The scripts expect:

- a working Rust toolchain
- `cargo` available in `PATH`
- the repo workspace intact under `rust/`

## Localhost Hosted Lab

### `install_nc_host_user_service.sh`

Dev-only localhost installer for the relay-native hosted lab.

It:

- builds `target/debug/nc-host`
- writes `~/.config/nc-host/host.kdl`
- writes `~/.config/nc-host/host.nsec` if missing
- installs `~/.config/systemd/user/nc-host.service`
- uses `~/.local/share/nc-host/games` as the local hosted games root
- removes stale `ec-gate.service` and `ec4x-daemon.service` if present
- reloads `systemd --user`
- restarts `nostr-relay.service`
- enables and starts `nc-host.service`

Example:

```bash
./scripts/install_nc_host_user_service.sh
```

Override the local hosted games root:

```bash
./scripts/install_nc_host_user_service.sh --games-root /tmp/nc-host-games
```

Use this when you want a stable localhost `nc-host` + `nc-helm` lab instead of
manually restarting binaries in separate terminals.

## Available Scripts

### `install_vps.sh`

Root-only idempotent bootstrap for the recommended VPS layout.

It:

- creates the dedicated `ecgame` service user
- ensures the service user has a real shell for forced SSH commands
- creates `/etc/nc-gate`, `/var/lib/nc-gate/keys`, and `/srv/ec/games`
- installs `nc-game` and `nc-sysop` into `/usr/local/bin`
- installs `/usr/local/bin/nc-gate-keys`
- writes `/etc/nc-gate/config.kdl`
- installs the `nc-nostr.service`, `nc-maint-all.service`, and `nc-maint-all.timer` units
- installs an `sshd` drop-in for the service user
- initializes `/etc/nc-gate/identity.kdl` if missing

It does not replace a public relay front end. If you self-host
`nostr-rs-relay` on the same VPS and keep it bound to `127.0.0.1:8080`,
you still need an HTTPS reverse proxy such as Caddy or nginx serving the
relay hostname on `443`.

Example:

```bash
sudo ./scripts/install_vps.sh \
  --relay wss://relay.example.com \
  --ssh-host play.example.com
```

This script never creates classic `.DAT` files or copies DOS artifacts into
per-game directories. Hosted Rust campaigns remain DB-only.

Host game-registry edits still happen as `root` because `/etc/nc-gate/config.kdl`
is host-owned. After `host games add/remove`, restart `nc-nostr.service` so the
daemon reloads the updated game list.

Create hosted games under `/srv/ec/games` as the `ecgame` service user, not as
plain `root`, so the daemon can write session leases into `ncgame.db`.

### `new_test_game.py`

Creates a fresh joinable game directory using the normal Rust sysop flow.

Example:

```bash
python3 scripts/new_test_game.py /tmp/ec-join-test --players 9 --force
```

Notes:

- `--players` is required
- `--seed` is optional
- `--force` removes the target directory first

This is the right default when you want a clean first-time/join flow for UI
testing.

### `setup_ui_stress_game.py`

Creates a richer four-player campaign specifically for testing the client and
maintenance behavior.

It currently:

- builds a year-`3010` game
- names all four players and empires
- gives each empire multiple owned planets
- gives player 1 a much larger and more varied fleet roster
- assigns a spread of fleet orders so menus have more interesting data

Example:

```bash
python3 scripts/setup_ui_stress_game.py /tmp/nc-ui-stress --force
```

Use this when you want:

- busy fleet lists
- planet-management screens with more than one colony
- more meaningful general/intel screens
- a useful maintenance test bed

### `setup_player1_tui_stress_game.py`

Creates a larger twelve-player campaign aimed specifically at **player 1 TUI
coverage** without running a bot-played campaign.

It currently:

- creates a fresh engine-backed `sysop new-game` campaign
- uses a fixed default map seed for reproducible placement
- supports `--players`, `--seed`, and `--turn`
- names all twelve empires and assigns them varied tax rates
- gives player 1 a large owned-colony footprint plus a much larger fleet roster
- stages empty and loaded troop transports at Aurora Prime for load/unload testing
- seeds player 1 with active starbases, rich unread report blocks, and queued mail
- injects mixed foreign-world intel for player 1 so database/detail screens show
  unknown, partial, and full scout-quality rows
- can also seed an isolated localhost `nc-connect` returning-player fixture
  with a pre-claimed hosted seat

Example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force
```

Turn-4 example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --turn 4
```

Explicit-seed example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --players 12 --seed 1515
```

Use this when you want:

- a busy player 1 startup flow with unread reports and messages
- fleet, planet, database, and rankings tables that scroll immediately
- `INFO ABOUT A PLANET` to show both owned detail and varied foreign intel
- starbase, transport, build, and stardock screens populated on first launch

Returning-player localhost fixture example:

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force --hosted-claim-player 1 --hosted-nsec-file /path/to/player.nsec
```

That mode:

- keeps the normal stress game seeding
- claims one hosted seat for the supplied identity
- creates an isolated `nc-connect` wallet/cache/config tree under `/tmp`
- prints exact launch commands for the localhost host helper and GUI reconnect

### `setup_nc_helm_lab.py`

Seeds a dedicated `nc-helm` stress lab with one campaign for each supported map
size tier:

- `18x18` (`4` players)
- `27x27` (`9` players)
- `36x36` (`16` players)
- `45x45` (`25` players)

It is a thin wrapper over `nc-cli harness seed-nc-helm-lab`, but it is shaped
for repo-root use and prints the exact `nc-helm` launch command for each map.

Example:

```bash
python3 scripts/setup_nc_helm_lab.py --root /tmp/nc-helm-lab --force
```

Launch one map immediately after seeding:

```bash
python3 scripts/setup_nc_helm_lab.py --root /tmp/nc-helm-lab --force --launch map45-p25
```

Use this when you want:

- quick visual passes across all `nc-helm` map tiers
- reproducible dashboard testing at small, medium, and large galaxy scales
- a single manifest under `/tmp` with launch commands for each seeded map

### `start_local_gui_hosted_test.sh`

Legacy helper for the retired `nc-sysop` / `nc-connect` hosted path.

It starts a local `nc-sysop nostr serve` instance for a stress-test game so the
standalone `nc-connect` GUI can join it through the old invite-code path.

Example:

```bash
./scripts/start_local_gui_hosted_test.sh --dir /tmp/ec-player1-ui
```

It:

- verifies `/tmp/ec-player1-ui/ncgame.db`
- reports pending invite codes when the game still has unclaimed seats
- reports claimed seats when the game is already seeded for returning-player reconnects
- requires a relay already listening at `ws://localhost:8080`
- writes a temporary gate config and identity under `/tmp/ec-local-gate`
- defaults loopback localhost to the current user plus `~/.ssh/authorized_keys`
- still supports explicit `--ssh-user` / `--auth-keys-*` overrides
- prints raw invite codes like `victim-sickness@localhost:8080`
- prints claimed seat identities for returning-player fixture checks
- runs `nc-sysop nostr serve` in the foreground

Use this only when you are deliberately testing the old retired hosted path.
For the current relay-native localhost lab, use
`install_nc_host_user_service.sh` with `nc-host` and `nc-helm` instead.

For same-machine hosted play, the intended normal command is the plain example
above; `sudo` should not be necessary unless your SSH setup is unusual.

### `setup_classic_probe_game.py`

Creates a fresh four-player Rust-backed campaign aimed at classic `ECGAME`
playback and display checking.

It currently:

- creates a fresh `sysop new-game` directory
- names all four players and empires
- gives player 1 multiple owned worlds and a busier fleet roster
- seeds nearby hostile target worlds and incoming enemy fleets
- runs several Rust maint turns so `RESULTS.DAT` contains a busy maint-report
  backlog when player 1 logs in
- clears routed `MESSAGES.DAT` output back out of the classic probe directory,
  because original `ECGAME` can display the maint reports from `RESULTS.DAT`
  but the Rust-only routed `MESSAGES.DAT` format is not classic-compatible
- prepares the classic login alias for player 1
- launches original `ECGAME` through DOSBox-X unless `--no-launch` is used

Example:

```bash
python3 scripts/setup_classic_probe_game.py /tmp/nc-classic-probe --force
```

Dry-run example:

```bash
python3 scripts/setup_classic_probe_game.py /tmp/nc-classic-probe --force --no-launch
```

Use this when you want:

- the original DOS client, not `nc-game`
- a busy unread-report backlog for player 1
- multiple fleets and planets to inspect in classic menus
- a practical hybrid-loop smoke test after Rust maint changes

### `run_classic_report_probe.sh`

Thin wrapper around `setup_classic_probe_game.py` for the most common
"open classic ECGAME with pending reports" workflow.

If you do not pass a target directory, it uses:

```bash
/tmp/nc-classic-report-probe
```

Example:

```bash
./scripts/run_classic_report_probe.sh
```

Custom target example:

```bash
./scripts/run_classic_report_probe.sh /tmp/ec-report-format-probe
```

Pass-through example:

```bash
./scripts/run_classic_report_probe.sh --turns 6 --alias SYSOP
```

This wrapper always adds `--force`, so rerunning it refreshes the probe
directory before launch.

### `build_release_packages.py`

Builds the reproducible EC v1.5 DOS release zips under `releases/`.

It currently:

- builds one archive with the original packed binaries
- builds one archive with the curated runnable unlocked binaries
- copies the original `.DOC` manuals into each package
- seeds both packages with the preserved `fixtures/ecutil-init/v1.5` game
  directory
- generates the known-good local-console `CHAIN.TXT`
- refreshes `NC_UNLOCKED/` first when the unlocked variant is selected
- validates the generated archives when `--verify` is passed

Example:

```bash
python3 scripts/build_release_packages.py --verify
```

Classic-only example:

```bash
python3 scripts/build_release_packages.py --variant classic --verify
```

Use this when you want:

- a reproducible bundle to hand to emulator developers
- a small local package that opens with the repo's known-good `CHAIN.TXT`
- a clean split between original packed and curated runnable unlocked variants

### `build_release_bundle.py`

Builds a standalone release bundle under `releases/`.

It currently:

- builds either:
  - the internal combined private-beta bundle
  - a public `nc-connect` player archive
  - or a public `nc-sysop` BBS/sysop archive
- includes the matching public PDF manuals in the archive
- writes `README.md` and `BUILD-INFO.txt` into the bundle root
- can unpack and smoke-test the bundle when `--verify` is passed
- defaults to the current host Rust target, with explicit support for:
  - `x86_64-unknown-linux-gnu`
  - `aarch64-apple-darwin`
  - `x86_64-apple-darwin`
  - `x86_64-pc-windows-msvc`
  - `i686-pc-windows-msvc` for `--artifact sysop` only
  - `i686-win7-windows-msvc` for `--artifact sysop` only

Example:

```bash
python3 scripts/build_release_bundle.py --verify
```

Public player archive example:

```bash
python3 scripts/build_release_bundle.py --artifact nc-connect --verify
```

Public sysop archive example:

```bash
python3 scripts/build_release_bundle.py --artifact sysop --target x86_64-unknown-linux-gnu --verify
```

Explicit Apple Silicon example:

```bash
python3 scripts/build_release_bundle.py --artifact nc-connect --target aarch64-apple-darwin --verify
```

Use this when you want:

- a native archive without requiring a Rust toolchain
- either a public `nc-connect` player package, a public `nc-sysop` sysop
  package, or the internal combined bundle
- a quick way to hand players or testers the right manual with the binary

The combined bundle is an internal/private-beta helper. The `nc-connect`
artifact is the public player-facing archive. The `sysop` artifact is the
public BBS/sysop archive for Windows and Linux door hosts.

The release tooling supports public `nc-connect` archives for:

- `x86_64-pc-windows-msvc`
- `x86_64-unknown-linux-gnu`
- `aarch64-apple-darwin`
- `x86_64-apple-darwin`

The release tooling supports public `nc-sysop` archives for:

- `x86_64-pc-windows-msvc`
- `i686-pc-windows-msvc`
- `i686-win7-windows-msvc`
- `x86_64-unknown-linux-gnu`

The `i686-win7-windows-msvc` sysop archive uses `cargo +nightly` with
`-Z build-std=std,panic_abort` because Rust does not ship the standard library
for that legacy Win7 target in the normal stable distribution.

### `build_linux_playtest_bundle.py`

Compatibility wrapper around `build_release_bundle.py` that keeps the old
Linux x64 command working:

```bash
python3 scripts/build_linux_playtest_bundle.py --verify
```

### `publish_release_packages.py`

Builds the selected DOS release bundles and/or public Rust download archives,
verifies them, then uploads the generated assets to an existing GitHub Release
with `gh release upload --clobber`. When public Rust assets are part of the
run, it also refreshes the signed-download verification block at the top of the
release body.

Default example:

```bash
python3 scripts/publish_release_packages.py
```

Custom tag example:

```bash
python3 scripts/publish_release_packages.py --tag release-artifacts
```

Unlocked-only example:

```bash
python3 scripts/publish_release_packages.py --variant unlocked
```

Windows player archive example:

```bash
python3 scripts/publish_release_packages.py \
  --nc-connect-target x86_64-pc-windows-msvc \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Linux player archive example:

```bash
python3 scripts/publish_release_packages.py \
  --nc-connect-target x86_64-unknown-linux-gnu \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Linux sysop archive example:

```bash
python3 scripts/publish_release_packages.py \
  --sysop-target x86_64-unknown-linux-gnu \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Windows sysop archive example:

```bash
python3 scripts/publish_release_packages.py \
  --sysop-target x86_64-pc-windows-msvc \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Windows 32-bit sysop archive example:

```bash
python3 scripts/publish_release_packages.py \
  --sysop-target i686-pc-windows-msvc \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Windows 7 32-bit sysop archive example:

```bash
python3 scripts/publish_release_packages.py \
  --sysop-target i686-win7-windows-msvc \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Apple Silicon player archive example:

```bash
python3 scripts/publish_release_packages.py \
  --nc-connect-target aarch64-apple-darwin \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Signed public player release example:

```bash
python3 scripts/publish_release_packages.py \
  --nc-connect-target aarch64-apple-darwin \
  --gpg-key C3504EE1EE38410CE1C433BC372B8AAACB867F13
```

Use this when you want:

- the easiest release workflow for DOS bundles and public Rust download archives
- the generated release assets to stay untracked locally under `releases/`
- the public downloadable copies to live on GitHub Releases instead of `main`
- signed `SHA256SUMS.txt` / `SHA256SUMS.txt.asc` assets for the public
  Rust archives

When `--nc-connect-target` or `--sysop-target` is used,
`publish_release_packages.py` requires `--gpg-key` and signs the shared public
Rust checksum manifest for the selected target(s). It keeps that manifest
complete by reusing any other already-published public Rust archives from the
release. It also updates the GitHub release-body verification notice
automatically.

`publish_release_packages.sh` remains as a thin compatibility wrapper around
the Python entrypoint.

### `run_client.py`

Launches `nc-game` against a chosen campaign directory and player seat.

Example:

```bash
python3 scripts/run_client.py /tmp/nc-ui-stress --player 1
```

Release build example:

```bash
python3 scripts/run_client.py /tmp/nc-ui-stress --player 1 --release
```

By default `run_client.py` launches `nc-game` against the existing
`ncgame.db` runtime state and does not refresh from classic `.DAT` files.
This preserves joins, theme choices, and other in-client changes across
re-entry.

If you intentionally changed the `.DAT` files outside the Rust runtime and
need to resync the runtime DB before launch, opt in explicitly:

```bash
python3 scripts/run_client.py /tmp/nc-ui-stress --player 1 --refresh-from-dat
```

## Recommended Workflow

### Clean join/onboarding test

```bash
python3 scripts/new_test_game.py /tmp/ec-join-test --players 4 --force
python3 scripts/run_client.py /tmp/ec-join-test --player 1
```

### Rich UI / command-menu test

```bash
python3 scripts/setup_ui_stress_game.py /tmp/nc-ui-stress --force
python3 scripts/run_client.py /tmp/nc-ui-stress --player 1
```

### Player 1 TUI torture test

```bash
python3 scripts/setup_player1_tui_stress_game.py /tmp/ec-player1-ui --force
python3 scripts/run_client.py /tmp/ec-player1-ui --player 1
```

### Classic ECGAME playback test

```bash
python3 scripts/setup_classic_probe_game.py /tmp/nc-classic-probe --force
```

### Maintenance regression pass on a scripted game

```bash
cd rust
cargo run -q -p nc-cli -- maint-rust /tmp/nc-ui-stress 1
```

### Export a scripted game back to classic `.DAT`

```bash
cd rust
cargo run -q -p nc-cli -- db-export /tmp/nc-ui-stress /tmp/nc-ui-stress-exported
```

Then run the original oracle if needed:

```bash
python3 tools/ecmaint_oracle.py run /tmp/nc-ui-stress-exported
```

## Extending These Scripts

If you add new scripted setups, keep them aligned with the current project
boundary:

- prefer calling `nc-cli` commands instead of mutating SQLite directly in
  Python
- treat `nc-cli` as the stable setup surface
- keep the client/runtime SQLite-native
- use `db-export` only when you specifically need classic compatibility output

If a setup needs a new reusable state mutation, add a focused `nc-cli` command
first and keep the Python layer thin.
