# Starbase Economy Oracle Audit

This note records the current black-box state of the starbase economy follow-up.

Goal:

- verify whether classic uses a specific starbase growth bonus
- understand the "withstand tax burden better" wording from the manuals

## Manual Baseline

The original docs are clear on the semantic direction:

- taxes above roughly `65%` can harm production
- a starbase lets a planet spend up to `5x` current production on builds
- a starbase helps a planet grow faster
- a planet with a starbase "may endure a tax rate of `67%` to `70%`, but
  there is no guarantee"

That gives a strong player-facing spec, but not an exact internal formula.

## Controlled Probe

Current reproducible tooling:

- CLI initializer:
  [rust/ec-cli/src/commands/economy.rs](../../rust/ec-cli/src/commands/economy.rs)
  `economy-starbase-probe-init`
- Oracle runner:
  [tools/ecmaint_starbase_economy_audit.py](../../tools/ecmaint_starbase_economy_audit.py)

Probe shape:

- create a fresh runtime campaign
- activate a stable player-facing setup
- seed one harmless movement order so classic definitely runs a yearly pass
- create two identical owned probe colonies:
  - `Plain Colony`
  - `Base Colony`
- commission a friendly active starbase at `Base Colony`
- export through `ec-compat`
- run classic `ECMAINT /R`
- import and compare the two colony rows

## Current Result

What *is* confirmed from the current probe family:

- the commissioned starbase survives the oracle run as a real active base
- the post-oracle imported colony still shows the starbase tag
- the post-oracle imported colony still shows `5x` build-capacity semantics
  through the larger `build_capacity` value

What is **not** reliable enough yet:

- the exact current-production growth bonus
- the exact tax-burden threshold or penalty rule

The generated probe worlds still produce pathological post-oracle production
values that do not line up cleanly with the manuals. Representative sweep
results from the current generated baseline:

- `25%`: plain `50 -> 34`, starbase `50 -> 36`
- `50%`: plain `50 -> 45`, starbase `50 -> 37`
- `65%`: plain `50 -> 32`, starbase `50 -> 32`
- `67%`: plain `50 -> 32`, starbase `50 -> 36`
- `70%`: plain `50 -> 32`, starbase `50 -> 32`

Those outputs are too noisy and counterintuitive to promote into a canonical
classic formula. The likely conclusion is not "classic really behaves exactly
like this"; it is that the current generated colony-state probe is still not a
clean enough oracle baseline for exact economy recovery.

## Practical Conclusion

At the moment, the project can safely claim:

- classic/manuals clearly support `5x` starbase build capacity
- classic/manuals clearly support some separate starbase growth/tax tolerance
  benefit
- the exact classic numeric rule for that benefit remains unrecovered

So the Rust policy remains:

- keep starbase `5x` build capacity
- keep the documented Rust `+50%` growth bonus and `65 -> 70` safe-tax shift
  as canonical Rust behavior
- do **not** describe those exact numbers as oracle-verified classic formulas

## If Revisited

The next useful step is **not** another blind tax sweep on the same generated
baseline.

Do one of these first:

1. recover a more clearly accepted classic planet-state baseline before the
   economy mutation
2. or do a deeper static RE pass on the planet-side economy functions in the
   unwrapped `ECMAINTU.EXE` project

Until then, this thread is evidence of the limit of the current black-box
probe, not evidence for a new exact classic formula.
