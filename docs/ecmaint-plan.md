# ECMAINT Reverse-Engineering Plan

`ECMAINT` is the highest-value target for recovering the actual game engine.

The bundled documentation says that maintenance resolves:

- fleet movement
- battles
- planetary attacks
- yearly updates
- player messaging/report output

## Working Hypothesis

`ECMAINT` is a deterministic transform from:

- `PLAYER.DAT`
- `PLANETS.DAT`
- `FLEETS.DAT`
- `CONQUEST.DAT`
- related support files

into:

- updated game-state files
- regenerated database/report/message files

That means we can study it even without a faithful player UI, as long as we can produce valid pre-maintenance states.

## Immediate Goals

1. Identify the observable maintenance phases

- input validation
- movement resolution
- combat resolution
- build completion
- derived database/report generation

2. Build targeted scenario fixtures

- one order change, then maintenance
- one build queue, then maintenance
- one battle-producing setup, then maintenance

3. Correlate changed bytes with documented mechanics

- movement and ETA
- bombardment
- invasion
- ownership changes
- losses and surviving fleets

## Practical Method

### Phase 1: Controlled File-Transform Analysis

Treat `ECMAINT` as a black-box transformer.

Workflow:

1. choose a clean baseline fixture
2. introduce one known state change
3. run `ECMAINT`
4. diff every changed file
5. record offsets, records, and messages produced

Files of particular interest:

- `PLAYER.DAT`
- `PLANETS.DAT`
- `FLEETS.DAT`
- `CONQUEST.DAT`
- `DATABASE.DAT`
- `MESSAGES.DAT`
- `RESULTS.DAT`

### Phase 2: Static Binary Mapping

For each observed phase:

- identify the file I/O callers
- separate runtime/helper code from application code
- label likely maintenance pass routines

### Phase 3: Rust Conformance Model

As each rule becomes clear:

- encode the relevant fields in `ec-data`
- add scenario fixtures
- add tests that assert observed outcomes
- eventually move the behavior into `ec-core`

## What We Need To Learn

High priority:

- movement order resolution
- combat resolution order
- damage/loss formulas
- build completion timing
- rogue empire behavior
- database/message generation rules

Medium priority:

- summary counters in `CONQUEST.DAT`
- exact message formatting triggers
- ranking/report generation

## Why This Matters

If we understand `ECMAINT`, we understand the real rules of the game.

That is the core requirement for a faithful preservation-oriented Rust reimplementation.
