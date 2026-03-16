# AGENTS.md

This file is the agent-facing operating guide for this repository.

## Project Goal

The first concrete milestone is:

- generate 100% compliant gamestate files from Rust
- validate them against the original binaries, especially `ECMAINT.EXE`
- use that compliant generator as the bridge toward a full Rust reimplementation

Treat the original DOS binaries and preserved fixtures as the acceptance oracle.

## Architecture Rules

Enforce these as standing requirements:

- agents shall use data-oriented design
- agents shall keep logic DRY
- agents shall keep binary layout explicit
- agents shall use plain record/file data plus focused functions
- agents shall avoid deep object hierarchies and abstraction-heavy designs
- agents shall avoid monolithic source files when feature-oriented submodules are clearer

In practice:

- `ec-data` shall stay organized around explicit file/record layouts
- `ec-cli` shall stay organized around command-family submodules
- shared parsing, path, and reporting helpers shall live in support modules
- batch/report commands shall reuse pure validators rather than duplicate checks

See [docs/rust-architecture.md](docs/dev/rust-architecture.md) for the fuller rationale.

## Startup Reading Order

At the start of a development or agent session, read these in order:

1. [docs/next-session.md](docs/dev/next-session.md)
   Use this as the current handoff and restart point.
   It shall remain a compact restart brief, not a running notebook.
   If it starts accumulating detailed history, move that detail into
   archive docs and keep only the current state, current goal, biggest
   blockers, and immediate next steps.
2. [docs/approach.md](docs/dev/approach.md)
   Reconfirm the project goal, milestone ladder, and acceptance criteria.
3. [docs/rust-architecture.md](docs/dev/rust-architecture.md)
   Reconfirm the DOD/DRY/module-organization rules before editing Rust code.
4. [RE_NOTES.md](RE_NOTES.md)
   Read only the sections relevant to the current task; do not reload the full notebook unless needed.
5. [README.md](README.md)
   Check current user-facing commands/workflows before changing the CLI surface.

Before making gameplay or rules assumptions, also check the shipped game docs in
[original/v1.5](original/v1.5):

- [ECREADME.DOC](original/v1.5/ECREADME.DOC)
- [ECPLAYER.DOC](original/v1.5/ECPLAYER.DOC)
- [ECQSTART.DOC](original/v1.5/ECQSTART.DOC)
- [WHATSNEW.DOC](original/v1.5/WHATSNEW.DOC)

These docs shall be treated as a primary source for intended game behavior,
startup conditions, turn structure, and user-facing mechanics. Agents shall
check them before turning an observed pattern into a semantic claim.

If the task is Ghidra-heavy, also check:

- [docs/ghidra-workflow.md](docs/dev/ghidra-workflow.md)

If the task is DOSBox-heavy, also check:

- [docs/dosbox-workflow.md](docs/dev/dosbox-workflow.md)

## Testing Rules

- keep tests in crate `tests/` directories, not inline `#[cfg(test)]` modules, unless there is a strong local reason not to
- prefer regression tests that lock in preserved fixture behavior
- run `cargo test` from `rust/` after meaningful Rust changes
- do not claim compliance unless the relevant Rust tests are green

## Reverse-Engineering Rules

- `RE_NOTES.md` is the chronological RE notebook
- `docs/` holds stable engineering guidance and milestone docs
- historical or bulky handoff detail shall be archived outside
  `docs/next-session.md`, for example in `docs/next-session-archive.md`
- when a significant RE finding lands, update both:
  - `RE_NOTES.md`
  - the relevant stable doc, usually `docs/next-session.md` and/or `docs/approach.md`
- prefer headless Ghidra scripts and reproducible artifacts over ad hoc manual notes
- do not treat guessed semantics as settled; keep unknown fields raw until supported
- agents shall check the shipped game docs in `original/v1.5/*.DOC` before
  making gameplay/rules assumptions; binary RE and fixture diffs shall be
  reconciled with the original docs rather than replacing them
- agents shall use escalating RE depth:
  - start with Rust-generated scenarios, preserved fixtures, and black-box
    oracle testing against the original binaries
  - for new mechanics, default to:
    - initialize or materialize a controlled directory
    - submit one narrow order family or field mutation
    - run the original binary oracle
    - diff `.DAT` and report outputs
  - promote repeated deterministic pass/fail patterns into shared Rust rules first
  - escalate to deep static/dynamic RE only when:
    - the path blocks broader compliant gamestate generation
    - black-box testing has plateaued
    - the expected rule is reusable rather than one-off trivia
- agents shall stop a deep RE thread once the rule is explicit enough to
  promote into Rust; they shall not continue a rabbit hole only for extra
  historical detail
- agents shall not default to maximum-depth RE for every mechanic; the recent
  Guard Starbase / `unknown starbase` path is the blocker-escalation template,
  not the baseline workflow

## Rust Workflow

When extending Rust support:

- first add or refine typed accessors in `ec-data`
- then expose the behavior through focused `ec-cli` commands
- then add regression tests using preserved fixtures or known-valid generated directories
- then update docs for the new capability

Prefer rule-shaped generators and validators over preserved-byte blob emission.

## Commit / Doc Workflow

- keep commits scoped to a real milestone or coherent gain
- update user-facing docs when commands, workflows, or project milestones change
- mention new commands in `README.md` and the relevant docs when they become part of the normal workflow
- preserve unrelated local `.ghidra` project DB churn; do not revert it unless explicitly asked
- `docs/next-session.md` shall stay short and current; archive older detail
  instead of appending indefinitely

## Current Priorities

1. expand Rust from known accepted scenarios toward general compliant gamestate generation
2. keep recovering `ECMAINT` cross-file linkage and integrity rules
3. replace scenario-specific constants with explicit validators/builders
4. defer any KDL scenario DSL until the internal Rust state/order model stabilizes

## Avoid

- large catch-all `main.rs` growth
- copy-pasted scenario logic across commands
- mixing semantic RE guesses into typed APIs too early
- treating preserved fixture recreation as the end state; it is only the first milestone
