# Setup KDL Schema

This document defines the KDL shape supported for sysop/admin game setup.

It is both:

- the current schema contract for `ec-cli sysop new-game --config ...`
- the base shape for future setup/admin expansion

## Goals

The `setup.kdl` file should cover the durable, declarative parts of `ECUTIL`:

- player count
- optional map-generation seed
- maintenance schedule
- setup/program options

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

## Schema

Top-level nodes:

- `game`
- `setup_options`
- `port_setup`

### `game`

Required properties:

- `player_count`

Optional properties:

- `seed`

Recommended meanings:

- `player_count`
  - current Rust-compatible range: `1..=25`
  - long-term manual tiers: `4`, `9`, `16`, `25`
- `seed`
  - optional map-generation seed
  - if omitted, `ec-cli sysop new-game` may generate one at runtime

All new games start at year 3000. The `year` and `setup_mode` properties are
accepted for backward compatibility with existing dev configs but are silently
ignored.

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

Explicit homeworld placement is not part of the KDL schema.

For the current Rust setup path:

- `player_count` is explicit in KDL
- the game engine supplies homeworld placement for the chosen player count
- `seed` controls reproducible generated placement when present

## Example

```kdl
game player_count=4 seed=1515

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

Current validation enforces:

- `player_count` in the currently supported range
- `seed` in unsigned integer range
- options values in sane ranges
- COM IRQ values in allowed range

It does not yet promise:

- full manual-tier support beyond the current record-model limits
- exact `ECUTIL` map RNG recreation
- full player metadata editing

## Adoption Plan

Recommended sequence:

1. keep this schema and the sample file in sync with the Rust parser
2. extend validation as more `ECUTIL` fields move into KDL
3. keep `ec-cli sysop new-game --config setup.kdl` materializing compatible directories
4. expand the schema for richer map-generation choices and larger manual player tiers
