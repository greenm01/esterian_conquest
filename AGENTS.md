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
- agents shall keep source files mean and lean
- agents shall keep binary layout explicit
- agents shall use plain record/file data plus focused functions
- agents shall avoid deep object hierarchies and abstraction-heavy designs
- agents shall not create giant monolithic source files
- agents shall organize code with focused subdirectories, submodules, and crates when that keeps features clearer and reuse cleaner
- if a file starts getting too large or unmanageable, agents shall stop and reassess the structure before growing that file further

In practice:

- `nc-data` shall stay organized around explicit file/record layouts
- `nc-cli` shall stay organized around command-family submodules
- shared parsing, path, and reporting helpers shall live in support modules
- batch/report commands shall reuse pure validators rather than duplicate checks
- larger features shall be split across focused modules instead of accumulating in oversized files
- when a file starts to feel crowded, agents shall pause and split or reorganize it instead of continuing to extend it by default

Pre-`v1.0` storage policy:

- agents shall not preserve SQLite schema-migration baggage for obsolete
  intermediate dev databases unless there is an active supported save base that
  actually requires it
- before `v1.0`, favor a clean forward schema over dead upgrade paths
- after `v1.0`, add migration support only for released, user-facing save
  formats with an explicit upgrade policy

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
4. [docs/dev/archive/RE_NOTES.md](docs/dev/archive/RE_NOTES.md)
   Read only the sections relevant to the current task; do not reload the full notebook unless needed.
5. [README.md](README.md)
   Check current user-facing commands/workflows before changing the CLI surface.

Before making gameplay or rules assumptions, first check the current Rust
manual sources:

- [docs/manuals/ec_player_manual.typ](docs/manuals/ec_player_manual.typ)
- [docs/manuals/nc_sysop_manual.typ](docs/manuals/nc_sysop_manual.typ)

These Typst manuals are the authoritative user-facing manuals for the modern
Rust edition.

If a rule, workflow, or terminology point is ambiguous or still uncovered
there, then check the shipped game docs in [original/v1.5](original/v1.5):

- [ECREADME.DOC](original/v1.5/ECREADME.DOC)
- [ECPLAYER.DOC](original/v1.5/ECPLAYER.DOC)
- [ECQSTART.DOC](original/v1.5/ECQSTART.DOC)
- [WHATSNEW.DOC](original/v1.5/WHATSNEW.DOC)

The preserved original docs are historical/provenance references and an
ambiguity fallback for intended classic behavior, startup conditions, turn
structure, and user-facing mechanics. Agents shall consult them when the Rust
manuals or stable docs are incomplete, then reconcile the result back into the
current manuals/docs rather than treating the originals as a higher-authority
product surface.

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

- `docs/dev/archive/RE_NOTES.md` is the chronological RE notebook (archival; prefer dedicated spec docs)
- `docs/` holds stable engineering guidance and milestone docs
- historical or bulky handoff detail shall be archived outside
  `docs/next-session.md`, for example in `docs/dev/archive/next-session-archive.md`
- the RE phase is complete for implementation; new findings are refinements only
- if new RE evidence arises (edge cases, expanded oracle testing), update:
  - `docs/dev/archive/RE_NOTES.md`
  - the relevant spec doc, if the finding changes implementation guidance
- prefer headless Ghidra scripts and reproducible artifacts over ad hoc manual notes
- do not treat guessed semantics as settled; keep unknown fields raw until supported
- agents shall treat `docs/manuals/ec_player_manual.typ` and
  `docs/manuals/nc_sysop_manual.typ` as the authoritative manual sources for
  the Rust edition
- agents shall check the shipped game docs in `original/v1.5/*.DOC` when the
  current manuals/docs are ambiguous or incomplete; binary RE and fixture
  diffs shall be reconciled with the original docs and then folded back into
  the current manuals/specs rather than leaving the originals as the only
  source
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

- first add or refine typed accessors in `nc-data`
- then expose the behavior through focused `nc-cli` commands
- then add regression tests using preserved fixtures or known-valid generated directories
- then update docs for the new capability

Prefer rule-shaped generators and validators over preserved-byte blob emission.

## Shell Notes

- the user runs **fish shell**; never give multi-line commands that rely on bash `\` continuation or assume bash syntax
- for `git add`, `gh release upload`, and similar commands with multiple file arguments, put all args on a single line — fish treats each newline as a new command and will try to execute filenames as commands

## Commit / Doc Workflow

- keep commits scoped to a real milestone or coherent gain
- update user-facing docs when commands, workflows, or project milestones change
- mention new commands in `README.md` and the relevant docs when they become part of the normal workflow
- in repository markdown/docs, use relative Markdown links for repo files; do
  not commit machine-local absolute paths, so links render correctly on GitHub
- do not parallelize ordering-dependent git operations; run steps like
  `git commit` and `git push` serially so later commands cannot race earlier
  state changes
- preserve unrelated local `.ghidra` project DB churn; do not revert it unless explicitly asked
- `docs/next-session.md` shall stay short and current; archive older detail
  instead of appending indefinitely

## Temp Files

- agents shall use `/tmp` for scratch probe directories, debug gamestates,
  temporary captures, and other disposable work products
- agents shall not create repo-local temp directories such as `tmp_*` or
  similar scratch folders in the working tree unless explicitly asked
- if a repo-local scratch directory is created accidentally during a session,
  agents shall move or delete it before finishing the task

## Current Priorities

1. advance the Rust-first engine and player/TUI experience on top of the stable
   runtime/model architecture
2. preserve classic compatibility at the explicit import/export/oracle boundary
   through focused regression tests and sweeps
3. replace scenario-specific constants with explicit validators/builders
4. use typed KDL for setup, turn submission, and harness/scenario authoring now
   that the runtime/state model exists, but keep Rust types authoritative and
   avoid freeform arbitrary-mutation DSLs
5. reopen deep oracle / Ghidra reverse engineering only for a concrete
   compatibility regression, crash, file-safety problem, or player-visible rule
   mismatch that materially blocks Rust development

## Avoid

- large catch-all `main.rs` growth or other giant monolithic source files
- copy-pasted scenario logic across commands
- duplicated logic that should be shared through focused helpers or modules
- mixing semantic RE guesses into typed APIs too early
- treating preserved fixture recreation as the end state; it is only the first milestone
