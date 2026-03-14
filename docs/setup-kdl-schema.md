# Setup KDL Schema

This document defines the first KDL shape now supported for sysop/admin game
setup.

It is both:

- the current schema contract for `ec-cli sysop new-game --config ...`
- the base shape for future setup/admin expansion

## Goals

The first `setup.kdl` should cover the durable, declarative parts of `ECUTIL`:

- player count
- game year
- optional map-generation seed
- maintenance schedule
- setup/program options
- setup mode

It should not try to encode:

- low-level `.DAT` byte quirks
- maintenance phase logic
- combat mechanics
- arbitrary live-turn state mutation

## Ownership Boundary

The intended flow is:

1. sysop writes or edits `setup.kdl`
2. `ec-cli sysop` validates the file
3. Rust materializes a classic-compatible `.DAT` directory through
   `CoreGameData`

So:

- KDL stores setup intent
- Rust remains the compatibility and writeback authority

## First Schema

Top-level nodes:

- `game`
- `setup_options`
- `port_setup`

### `game`

Required properties:

- `player_count`
- `year`
- `setup_mode`

Optional properties:

- `seed`

Recommended meanings:

- `player_count`
  - current Rust-compatible range: `1..=25`
  - long-term manual tiers: `4`, `9`, `16`, `25`
- `year`
  - starting game year, normally `3000`
- `seed`
  - optional map-generation seed
  - if omitted, `ec-cli sysop new-game` may generate one at runtime
- `setup_mode`
  - `"canonical-four-player"`
  - `"builder-compatible"`

`setup_mode` exists so the first parser can distinguish:

- the classic 4-player start package
- the broader compatibility-oriented generated new-game path

### `setup_options`

Optional properties:

- `snoop`
- `local_timeout`
- `remote_timeout`
- `max_key_gap_minutes`
- `minimum_time_minutes`
- `purge_after_turns`
- `autopilot_after_turns`
- `maintenance_days`

Recommended shape:

- booleans for on/off fields
- integers for minutes/turns
- `maintenance_days` as child nodes or repeated values

### `port_setup`

Optional container for preserved `ECUTIL F5` modem/COM settings.

Repeated `com` nodes with:

- `port`
- `irq`
- `hardware_flow_control`

Example meanings:

- `port="com1"`
- `irq=4`
- `hardware_flow_control=#true`

Explicit homeworld placement is not part of the first KDL schema.

For the current Rust setup path:

- `player_count` is explicit in KDL
- the game engine supplies homeworld placement for the chosen player count
- `seed` controls reproducible generated placement when present
- `canonical-four-player` still requires `player_count = 4`, but placement
  remains engine-generated rather than KDL-authored

## Example

```kdl
game player_count=4 year=3000 setup_mode="builder-compatible" seed=1515

setup_options snoop=#true local_timeout=#false remote_timeout=#true max_key_gap_minutes=10 minimum_time_minutes=0 purge_after_turns=0 autopilot_after_turns=0

port_setup {
  com port="com1" irq=4 hardware_flow_control=#true
  com port="com2" irq=3 hardware_flow_control=#true
  com port="com3" irq=4 hardware_flow_control=#true
  com port="com4" irq=3 hardware_flow_control=#true
}

maintenance_days {
  day "sun"
  day "mon"
  day "tue"
  day "wed"
  day "thu"
  day "fri"
  day "sat"
}
```

## Validation Rules

First-version validation should enforce:

- `player_count` in the currently supported range
- `year` in the classic accepted range
- `seed` in unsigned integer range
- options values in sane ranges
- COM IRQ values in allowed range

It should not yet promise:

- full manual-tier support beyond the current record-model limits
- exact `ECUTIL` map RNG recreation
- full player metadata editing

## Adoption Plan

Recommended sequence:

1. keep this schema and the sample file in sync with the Rust parser
2. extend validation as more `ECUTIL` fields move into KDL
3. keep `ec-cli sysop new-game --config setup.kdl` materializing compatible directories
4. expand the schema for richer map-generation choices and larger manual player tiers
