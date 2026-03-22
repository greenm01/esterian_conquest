# Campaign Play

This is the reproducible conductor workflow for running a real multi-player
campaign with LLM or bot players.

The operator goal is simple:

- tell the harness to build a campaign
- let the conductor run turns until a requested open turn
- stop and inspect the live game in the TUI

The conductor is the only authority that advances the year. Bots act like
normal players: they read their own bundle, write `turn.kdl`, and wait for the
conductor to validate and apply it.

## Two Coordinator Layers

There are two distinct coordinator roles in this workflow.

1. `ec-cli harness` conductor
   - opens turns
   - writes bundles and status files
   - validates `turn.kdl`
   - applies the full player batch
   - runs maintenance

2. outer operator or LLM coordinator
   - notices that a turn is open
   - claims the player slots
   - spawns one player worker per empire
   - gives each worker only its own bundle and output path
   - waits for the `turn-<nnnn>.kdl` files
   - runs `scan-turn` and `apply-turn-batch`

The first coordinator is in-repo and authoritative over campaign state. The
second is the orchestration layer that actually sends out sub-agents.

## Happy Path

Build the campaign and advance it until turn 5 is open:

```bash
cd rust
cargo run -q -p ec-cli -- harness play-until --file /tmp/scenario.kdl --dir /tmp/ec-bot-campaign --game-id tui-polish --turn 5 --bundle-profile llm
```

If every active player already has a valid turn file for turns 1 through 4, the
command stops with turn 5 open.

Then inspect the live runtime campaign in the TUI:

```bash
cd rust
cargo run -q -p ec-client -- --dir /tmp/ec-bot-campaign --player 1
```

Use any player slot you want to inspect.

## Workspace Layout

The conductor writes all bot coordination files under the ignored local root:

```text
.tmp/llm-turns/<game_id>/campaign/manifest.kdl
.tmp/llm-turns/<game_id>/player-1/bundle-turn-0005/
.tmp/llm-turns/<game_id>/player-1/bundle-turn-0005/.llm/spatial.kdl   # llm profile only
.tmp/llm-turns/<game_id>/player-1/status-turn-0005.kdl
.tmp/llm-turns/<game_id>/player-1/turn-0005.kdl
.tmp/llm-turns/<game_id>/player-1/notes-0005.md
```

The workspace is durable enough to resume after interruption, but it is local
scratch output and should stay out of git.

## Turn State Machine

Per-player turn coordination is file-backed.

States:

- `ready`
  - conductor opened the turn and published the bundle
- `claimed`
  - bot/operator marked the turn in progress
- `submitted`
  - turn file exists and is pending validation
- `validated`
  - conductor accepted the file for the current year
- `rejected`
  - conductor rejected the file and wrote the error into the status file
  - the per-player bundle `README.md` is also refreshed with that rejection and
    the validation error, so a rerun worker sees it immediately
- `applied`
  - the full turn batch was applied and maintenance ran

Normal sequence:

1. conductor opens turn `N`
2. each player's `status-turn-<N>.kdl` becomes `ready`
3. bot or operator claims work with `harness claim-turn`
4. bot writes `turn-<N>.kdl`
5. conductor runs `harness scan-turn`
6. statuses become `validated` or `rejected`
7. when all active players are `validated`, conductor runs `harness apply-turn-batch`
8. maintenance advances once and turn `N+1` opens

If you are using LLM sub-agents, expand steps 3 through 6 like this:

1. outer coordinator claims player `1..N`
2. outer coordinator spawns one worker per player
3. each worker reads only its own `bundle-turn-<nnnn>/`
4. each worker writes only its own `turn-<nnnn>.kdl` and optional notes file
5. outer coordinator waits for all workers to finish
6. outer coordinator runs `scan-turn`
7. if any player is `rejected`, repair or rerun only that player
8. once all players are `validated`, outer coordinator runs `apply-turn-batch`

## Step-By-Step Commands

Initialize the campaign explicitly:

```bash
cd rust
cargo run -q -p ec-cli -- harness init-campaign --file /tmp/scenario.kdl --dir /tmp/ec-bot-campaign --game-id tui-polish --bundle-profile llm
```

Re-open or refresh bundles for the current open turn:

```bash
cd rust
cargo run -q -p ec-cli -- harness open-turn --dir /tmp/ec-bot-campaign
```

Mark a player's turn as in progress:

```bash
cd rust
cargo run -q -p ec-cli -- harness claim-turn --dir /tmp/ec-bot-campaign --player 2
```

Scan all current player workspaces and validate any submitted files:

```bash
cd rust
cargo run -q -p ec-cli -- harness scan-turn --dir /tmp/ec-bot-campaign
```

Apply a fully validated year batch:

```bash
cd rust
cargo run -q -p ec-cli -- harness apply-turn-batch --dir /tmp/ec-bot-campaign
```

Resume the conductor loop after more turns arrive:

```bash
cd rust
cargo run -q -p ec-cli -- harness play-until --file /tmp/scenario.kdl --dir /tmp/ec-bot-campaign --game-id tui-polish --turn 5 --bundle-profile llm
```

If the campaign manifest already exists, `play-until` resumes from the current
open turn instead of starting over.

## LLM Coordinator Pattern

This is the intended multi-agent pattern when you want the coordinator to send
out player workers directly.

1. run `harness init-campaign` once
2. when turn `N` opens, run `harness claim-turn --player <record>` for each active player
3. spawn one sub-agent per player
4. constrain each sub-agent to:
   - the manuals
   - that player's `bundle-turn-<nnnn>/`
   - that player's prior notes and turn files only
   - that player's output `turn-<nnnn>.kdl`
5. wait for all workers
6. run `harness scan-turn`
7. repair or rerun only rejected players
8. run `harness apply-turn-batch`
9. repeat until the requested open turn

Do not let player workers:

- read other players' bundles
- inspect `ecgame.db`
- apply maintenance
- edit another player's turn file

## What Bots Receive

Each player bundle contains only player-safe information:

- current game id, player, turn, year, and doctrine
- owned planets and fleets
- economy and stardock summaries
- diplomacy state as known to that player
- player-visible starmap exports
- coordinator-generated legal action hints per fleet
- hidden `.llm/spatial.kdl` route/target summaries when the campaign uses `--bundle-profile llm`
- incoming player mail from the immediately completed turn
- review pending flags

This is the current fog-of-war boundary. The conductor does not currently dump
raw global report bytes into bot bundles, because that would make it too easy
for smart bots to infer hidden state.

The legal action hints are there to reduce invalid bot turns. A player worker
should treat them as authoritative for what order families and visible targets
are safe to submit from the current bundle. In `llm` profile campaigns, bots
should prefer `.llm/spatial.kdl` as their machine-readable spatial surface and
use `README.md` as the human-readable companion.

## Messaging And Doctrine

Bots may use normal `turn.kdl` diplomacy and `message` commands to:

- coordinate with neighbors
- bluff or threaten
- recruit temporary allies
- declare enemies so fleets will actually engage

The conductor assigns each player a doctrine from a 12-type EC-native pool:

- `landgrabber`
- `surveyor`
- `shipwright`
- `fortifier`
- `raider`
- `blockader`
- `invader`
- `bombardier`
- `marshal`
- `schemer`
- `zealot`
- `kingmaker`

The pool is shuffled per campaign from the scenario seed plus `game_id`, so
player 1 is not always the same style. Treat doctrine as planning flavor, not
as hidden information or a rules override. Bots still have to act only on
visible state.

## Recovery

If `play-until` stops early, it means the campaign is blocked on missing or
invalid player turns. Check the current status files under
`.tmp/llm-turns/<game_id>/player-<n>/`:

- if a file is `ready`, the player has not started
- if it is `claimed`, the bot is working
- if it is `rejected`, fix the `turn.kdl` and rerun `scan-turn`

Once the missing players are `validated`, rerun `play-until` and the conductor
continues from the persisted campaign state.
