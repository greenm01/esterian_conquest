# LLM Player Guide

This guide defines how to use an LLM or bot as a real EC player in a live
campaign.

The core rule is simple: the bot plays from the same information a human player
would have. It may use the manuals, its own reports and messages, and any
player-visible summaries explicitly given to it. It may not inspect hidden game
state.

## Required Reading Order

Before an LLM starts planning moves, give it these docs in this order:

1. [../manuals/ec_qstart.md](../manuals/ec_qstart.md)
2. [../manuals/ec_player.md](../manuals/ec_player.md)
3. [../player/turn-kdl.md](../player/turn-kdl.md)

Recommended supporting references for operators:

- [ec-reports.md](ec-reports.md)
  - report wording and narrative shapes
- [harness/README.md](harness/README.md)
  - runtime scenario replay and combat harness workflows
- [harness/campaign-play.md](harness/campaign-play.md)
  - reproducible conductor workflow for live multi-bot campaigns

## Allowed And Forbidden Inputs

Allowed inputs:

- the player manuals under [../manuals/](../manuals/)
- the bot's own reports, messages, and review screens
- player-visible planet, fleet, and database summaries
- the current year, player number, and open turn number
- a scenario brief that is explicitly provided to that player
- the bot's own notes from prior turns

Forbidden inputs:

- direct inspection of `ecgame.db`
- hidden state from other empires
- total-map dumps or developer summaries that reveal unexplored worlds
- harness/debug/oracle output not available to the player
- combat or maintenance metrics that reveal unseen outcomes
- another empire's submitted `turn.kdl`

If a helper script or operator has more information than a player would have,
that extra information must not be passed into the bot prompt.

## Standard Turn Workspace

Generated bot turns and notes must stay out of git.

Use this default workspace layout:

```text
.tmp/llm-turns/<game_id>/player-<n>/turn-0005.kdl
.tmp/llm-turns/<game_id>/player-<n>/notes-0005.md
.tmp/llm-turns/<game_id>/player-<n>/status-turn-0005.kdl
```

Guidelines:

- organize files first by game, then by player
- keep one `turn-<nnnn>.kdl` per open turn
- keep optional reasoning notes beside the KDL, not inside it
- treat `.tmp/llm-turns/` as disposable local output
- do not commit generated turn corpora, logs, or notes

If you need a purely external temp location, mirror the same layout under
`/tmp/ec-llm-turns/`, but `.tmp/llm-turns/` is the repo-local default.

In conductor-driven campaigns, the status file is how the coordinator and the
bot signal progress to each other. The normal flow is:

1. the conductor opens the turn and the status becomes `ready`
2. the bot or operator claims work on it
3. the bot writes `turn-<nnnn>.kdl`
4. the conductor scans and marks the file `validated` or `rejected`

The bundle `README.md` is refreshed alongside that status. If the conductor
rejects a turn, the current bundle will show both the rejection state and the
validation error, next to the legal action hints for the rerun.

Use the helper command when a bot starts work:

```bash
cd rust
cargo run -q -p ec-cli -- harness claim-turn --dir /tmp/ec-bot-campaign --player 2
```

The bot should not advance maintenance itself. The conductor does that only
after every active player's file is validated.

## If You Are Using A Coordinator With Sub-Agents

When an outer LLM coordinator is running the campaign, it should spawn one
player worker per empire for the current turn.

That coordinator should give each worker only:

- the manuals
- that player's own `bundle-turn-<nnnn>/`
- that player's prior notes and turn files
- that player's target `turn-<nnnn>.kdl` path

It should not give a player worker:

- another player's bundle
- another player's `turn.kdl`
- the campaign database
- maintenance output or hidden summaries

The outer coordinator then waits for all player workers, runs `scan-turn`,
fixes any rejected player files, and only then runs `apply-turn-batch`.

## What The Operator Should Provide

For each decision turn, the operator should give the bot:

- the game id
- the player number
- the current year and open turn
- the bot's current bundle from `.tmp/llm-turns/<game_id>/player-<n>/bundle-turn-<nnnn>/`
- any visible fleet, planet, and database summaries for that player
- the target output path for the next `turn.kdl`
- the legal order surface from [../player/turn-kdl.md](../player/turn-kdl.md)

Do not pre-digest the situation with hidden-state summaries. If you provide a
human-written briefing, it must be composed only from player-visible facts.

Important boundary:

- treat the bundle as authoritative for what the bot is allowed to know
- current bundles include player-visible starmap/intel, owned assets, diplomacy, and incoming player mail
- current bundles now also include coordinator-generated legal action hints per fleet
- current bundles do not expose raw global review text from `RESULTS.DAT` or `MESSAGES.DAT`
- do not pass the bot `ecgame.db`, hidden empire summaries, or developer-only metrics
- if using sub-agents, do not pass one bot another bot's bundle or turn file

## The Bot's Turn Loop

Every turn, the bot should work in this order:

1. Read reports and messages first.
2. Separate confirmed facts from inference.
3. Update its mental map of nearby worlds, fleets, borders, and threats.
4. Choose a strategic posture for the current phase of the game.
5. Decide this turn's concrete objectives.
6. Write a legal `turn.kdl`.
7. Write a short notes file with assumptions and next-turn follow-ups.

The notes file is optional, but recommended for multi-turn agent play.

Before finalizing the turn, the bot must run this self-check:

- every planet-targeted order uses a target that appears in the bundle's legal action hints
- `scout_sector` and `scout_system` are used only by fleets that actually have scout ships
- `colonize` is used only by fleets that actually have ETAC ships
- `invade` and `blitz` are used only by fleets with loaded troop transports
- if a legal target is unclear, replace the risky order with `hold`, `move`, `seek_home`, or a message/diplomacy action

Submitting a syntactically valid but obviously illegal or nonsense order is a bot failure.

## How To Think About EC

EC is a four-phase 4X game, but the exact boundaries are not fixed by turn
number. The bot should infer the phase from the map and reports.

### 1. Landgrab

Primary goals:

- find nearby raw worlds
- expand with `ETAC` fleets
- build map knowledge quickly
- avoid crippling long-term growth with excessive taxes

Typical posture:

- use destroyers and scouts to expand visible information
- build more colonization capacity while raw worlds remain
- protect the homeworld enough that a fast raid does not end the game

### 2. Consolidation

Primary goals:

- convert colonies into useful production
- fortify important worlds
- define interior vs frontier systems

Typical posture:

- lower taxes on young colonies when growth matters more than immediate cash
- add armies, ground batteries, and starbases to worlds that must survive
- keep flexible combat fleets near likely contact zones

### 3. Conflict And Leverage

Primary goals:

- identify weak rivals and exposed planets
- shape borders with diplomacy, deterrence, and selective force
- gather enough intelligence to attack where success is likely

Typical posture:

- scout before committing invasion fleets
- blockade and guard worlds to control movement
- use diplomacy as a timing tool, not as proof of safety

### 4. Imperial Endgame

Primary goals:

- eliminate or cripple the remaining powers that still matter
- deny key production to strong rivals
- protect gains from immediate recapture

Typical posture:

- prioritize planets that change the balance of production
- keep enough armies and fleet cover to hold conquered worlds
- bombard when taking a world intact is unrealistic or too expensive

## Strategy Heuristics

These are the default planning rules the bot should follow unless reports or
local conditions strongly argue otherwise.

### Economy

- early taxes usually should not exceed roughly the mid-60% range
- young colonies often benefit from lower tax pressure so current production can grow
- starbases are strategic multipliers, not vanity builds
- production should match a purpose: colonize, scout, defend, invade, or hold

### Information

- unexplored space is dangerous; scout and view aggressively
- a printed or saved starmap equivalent is strategically valuable
- scout reports are worth more than guesses when planning invasions
- distinguish between "seen once", "currently confirmed", and "assumed unchanged"

### Defense

- planets need armies, not just ships
- ground batteries punish invasion attempts and protect against surface attack
- fleets and starbases protect approach lanes; armies hold the planet itself
- a newly taken world that is not guarded is often only temporarily yours

### Fleet Design

- specialized fleets are efficient, but mixed fleets are more flexible
- slow ships cap fleet speed, especially `ETAC`s and starbases
- troop transports need loaded armies to matter
- commission ships with a specific mission in mind instead of accumulating idle hulls

### Assault Choice

- `BLITZ` is the fast capture option when army superiority is overwhelming
- `INVADE` is the safer capture option against defended worlds because it first strips ground batteries
- `BOMBARD` is the denial option when capture is unlikely or not yet worth the cost

### Diplomacy

- diplomacy can buy time or isolate a rival
- neutral does not mean safe
- declare enemies deliberately so combat fleets will actually engage
- use player `message` traffic to coordinate, bluff, threaten, or betray within normal game rules

## Doctrine And Persona

The conductor may assign a doctrine to each player from a 12-type EC-native
pool:

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

The pool is shuffled per campaign, so the same player slot does not always get
the same style.

Treat doctrine as a planning bias:

- it can shape risk appetite, diplomacy style, and build priorities
- it does not authorize cheating
- it does not replace the bot's obligation to respond to the actual reports and map situation

Good doctrine use:

- a `landgrabber` should value scouting and colonization in the opening
- a `fortifier` should spend more aggressively on defenses and holding worlds
- a `schemer` should use diplomacy and messages more actively

Bad doctrine use:

- pretending to know hidden enemy worlds
- ignoring immediate threats because the persona says so
- acting randomly for flavor instead of trying to win

## Reading Reports Without Cheating

Reports are evidence, not omniscience.

The bot should classify each report detail as one of:

- confirmed current fact
- confirmed past event
- likely inference
- unresolved uncertainty

Examples:

- a scout report confirming armies and ground batteries on a planet is strong evidence for that turn, not a permanent truth
- an old ownership report may be stale if several turns have passed without new contact
- a fleet sighting confirms location at that report's `Stardate`, not necessarily the current moment

When uncertain, the bot should prefer actions that gather information cheaply.

## Recommended Bot Output Shape

For operator review, ask the bot to produce two artifacts:

1. a short situation summary
2. the authoritative `turn.kdl`

Suggested reasoning headings:

- `Current Situation`
- `Turn Objectives`
- `Risks And Unknowns`
- `Next-Turn Watchlist`

Then require one fenced `kdl` block or one file write containing the final turn.

## Example Workflow

Example local output path:

```text
.tmp/llm-turns/phase-sapling-awful/player-2/turn-0005.kdl
```

Example valid turn file:

```kdl
turn player=2 year=3004

tax rate=42

diplomacy to=3 relation="enemy"

planet record=6 {
  build points=5 kind="destroyer"
}

fleet record=5 {
  roe value=6
  order speed=6 kind="view" x=9 y=12
}
```

Example operator flow in a conductor-led campaign:

1. Build or reopen a live campaign scenario.
2. Open the current player bundle and status file for player 2.
3. Claim the turn:

```bash
cd rust
cargo run -q -p ec-cli -- harness claim-turn --dir /tmp/ec-campaign --player 2
```

4. Prompt the bot with the manuals, the visible bundle, and the target path above.
5. Save the bot's output KDL at that path.
6. Let the conductor validate it:

```bash
cd rust
cargo run -q -p ec-cli -- harness scan-turn --dir /tmp/ec-campaign
```

7. If every player is validated, let the conductor advance the year:

```bash
cd rust
cargo run -q -p ec-cli -- harness apply-turn-batch --dir /tmp/ec-campaign
```

For one-off single-player experiments outside the conductor flow, direct
`submit-turn` is still available.

## Relationship To Other LLM Docs

- [llm-test-harness.md](llm-test-harness.md)
  - black-box DOS/BBS automation against the original game
- this guide
  - live Rust-campaign play using strict player-visible information and `turn.kdl`

Use the DOS/BBS harness when reverse-engineering original behavior. Use this
guide when you want a bot to act as a normal player in a Rust-driven campaign.
