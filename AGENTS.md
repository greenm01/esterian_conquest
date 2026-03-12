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

See [docs/rust-architecture.md](/home/mag/dev/esterian_conquest/docs/rust-architecture.md) for the fuller rationale.

## Startup Reading Order

At the start of a development or agent session, read these in order:

1. [docs/next-session.md](/home/mag/dev/esterian_conquest/docs/next-session.md)
   Use this as the current handoff and restart point.
   It shall remain a compact restart brief, not a running notebook.
   If it starts accumulating detailed history, move that detail into
   archive docs and keep only the current state, current goal, biggest
   blockers, and immediate next steps.
2. [docs/approach.md](/home/mag/dev/esterian_conquest/docs/approach.md)
   Reconfirm the project goal, milestone ladder, and acceptance criteria.
3. [docs/rust-architecture.md](/home/mag/dev/esterian_conquest/docs/rust-architecture.md)
   Reconfirm the DOD/DRY/module-organization rules before editing Rust code.
4. [RE_NOTES.md](/home/mag/dev/esterian_conquest/RE_NOTES.md)
   Read only the sections relevant to the current task; do not reload the full notebook unless needed.
5. [README.md](/home/mag/dev/esterian_conquest/README.md)
   Check current user-facing commands/workflows before changing the CLI surface.

If the task is Ghidra-heavy, also check:

- [docs/ghidra-workflow.md](/home/mag/dev/esterian_conquest/docs/ghidra-workflow.md)

If the task is DOSBox-heavy, also check:

- [docs/dosbox-workflow.md](/home/mag/dev/esterian_conquest/docs/dosbox-workflow.md)

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
