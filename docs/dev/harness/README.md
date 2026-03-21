# Harness KDL

`ec-cli harness` now supports two related but separate workflows:

- real-game/runtime scenario setup for TUI playtesting and repros
- combat scenario and sweep execution for combat stress testing and metrics

The harness is Rust-first:

- KDL expresses stable scenario intent
- Rust validates, builds runtime state, and executes turns/combat
- SQLite/runtime state is the primary output
- classic `.DAT` export is optional via `--export-classic`

## Commands

```bash
cd rust
cargo run -q -p ec-cli -- harness check-scenario --file /tmp/scenario.kdl
cargo run -q -p ec-cli -- harness run-scenario --file /tmp/scenario.kdl --dir /tmp/ec-scenario
cargo run -q -p ec-cli -- harness check-combat --file /tmp/combat-scenario.kdl
cargo run -q -p ec-cli -- harness run-combat --file /tmp/combat-scenario.kdl
cargo run -q -p ec-cli -- harness run-sweep --file /tmp/combat-sweep.kdl
```

## `scenario.kdl`

Use this when you want a coherent runtime campaign snapshot for playtesting.

```kdl
scenario player_count=4 year=3000 baseline="builder-compatible" seed=1515 label="Turn 3 Playtest"

house record=1 handle="SYSOP" empire="Aurora" homeworld="Aurora Prime" tax=30
house record=2 handle="RIVAL" empire="Helios Crown"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

planet record=1 {
  name "Aurora Prime"
  production potential=140 present=120 stored=80 economy_marker=30
  defenses armies=14 batteries=6
  stardock slot=1 kind="destroyer" count=3
  commission slot=1
}

fleet record=1 {
  coords x=10 y=10
  ships bb=1 ca=2 dd=3 sc=0 tt=1 armies=1 etac=0
  roe value=8
  order kind="hold" speed=0 x=10 y=10
}

turn-file path="player1-turn.kdl"
queued-mail from=1 to=2 year=3000 subject="Border" body="Hold line."
results-block player=1 "Command summary\nTurn 3 ready."
messages-block player=1 "Incoming traffic\nStand by."
```

Supported top-level nodes:

- `house`
  - set handle, empire name, homeworld name, and tax for a player record
- `relation`
  - stored diplomacy between houses
- `planet`
  - override planet coords, owner, name, production, defenses, stardock slots, and optional `commission` actions
- `fleet`
  - override an existing or commissioned fleet record's location, composition, ROE, speed, invasion armies, and order
- `turn-file`
  - apply an existing `turn.kdl` after scenario setup
- `queued-mail`
  - seed Rust queued outbox mail
- `results-block` / `messages-block`
  - seed startup review text for the referenced player

Notes:

- `baseline` can be `"builder-compatible"` or `"joinable-new-game"`
- the harness does not support arbitrary raw field mutation
- fleet ownership reassignment is not supported in the first version; use the baseline owner-linked fleet records or commission new fleets from stardock

## `combat-scenario.kdl`

Use this when you want a combat-focused scenario executed immediately through Rust maintenance.

```kdl
combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="Skirmish"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

fleet record=1 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}

fleet record=5 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}
```

This uses the same child-node vocabulary as `scenario.kdl`, but adds:

- `turns`
  - how many Rust maintenance turns to execute

## `combat-sweep.kdl`

Use this when you want many generated combat cases from one base combat scenario.

```kdl
combat-sweep scenario="combat.kdl" turns=1 seed=99 max_cases=8

fleet-ship fleet=1 kind="bb" 1 2 3
fleet-roe fleet=5 6 10
planet-stat planet=14 field="armies" 0 5 10
relation-variation from=1 to=2 "neutral" "enemy"
```

Supported sweep dimensions:

- `fleet-ship`
  - vary `bb`, `ca`, `dd`, `sc`, `tt`, `armies`, or `etac`
- `fleet-roe`
  - vary fleet ROE values
- `planet-stat`
  - vary `armies` or `batteries`
- `relation-variation`
  - vary diplomacy between `neutral` and `enemy`

Current sweep behavior:

- expansion is deterministic
- `max_cases` caps how many generated cases execute
- output focuses on event counts and elapsed time metrics
- the base scenario must already define the fleets/planets referenced by the sweep dimensions
