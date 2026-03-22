# Preservation Approach

This repository is not trying to recover the original Pascal source code
verbatim. The goal is to preserve Esterian Conquest v1.5 as a working
historical artifact, reverse engineer its file formats and rules, and build
Rust tooling that can generate 100% compliant gamestate files accepted by the
original game and `ECMAINT`. That compliance target is the first concrete
milestone toward a faithful modern reimplementation in Rust. The original DOS
binaries and data remain the reference implementation throughout.

## Principles

### Manuals as spec, binaries as oracle

The shipped manuals define intended player-facing rules and mechanics. The
three original executables each serve a distinct role: `ECGAME.EXE` is the
player-facing command UI, `ECUTIL.EXE` is the sysop/configuration utility, and
`ECMAINT.EXE` is the yearly maintenance and simulation engine.

When semantics and implementation quirks diverge, prefer the manuals for
gameplay meaning and the binaries for file compatibility, accepted directory
structure, and proven cross-file invariants. Do not chase byte-perfect parity
if it would force Rust away from the original documented rules without adding
compatibility value. Document original logic bugs when they matter for oracle
work, but do not intentionally reproduce them in Rust unless they are required
for classic file safety or parser acceptance.

### Confirmed behavior over guessed structure

Only name fields after they are supported by diffs, screenshots, docs, or
repeated observation. Keep unknown bytes raw until they are mapped with
confidence. When the original UI exposes a higher-level semantic label, prefer
that player-facing term over a lower-level storage nickname. For example, Rust
may still carry internal economic field names like `factories` while RE is in
progress, but player/client surfaces should ultimately align to classic terms
like `Present Production`, `Potential Production`, and `Total Available Points`
once those semantics are confirmed.

### Meaningful fidelity over ghost fidelity

The project is not trying to preserve every hidden numeric quirk, scratch byte,
or ambiguous maintenance artifact from the original binaries. The fidelity
target is the combination of:

- player-facing rules and timing that materially affect decisions
- compatibility behavior required for classic file safety and oracle workflows
- stable, reusable semantics that improve the Rust engine

When an oracle thread bottoms out in hidden state, inconsistent low-level
numeric behavior, or probable implementation bugs that do not materially affect
the game, prefer a documented Rust rule over indefinite ghost-chasing. The
current Rust combat system is already the model for this: preserve the turn
placement and visible consequences that matter, but do not freeze the engine
waiting for perfect historical internals.

### Reverse engineering is now a closed phase

The heavy discovery phase is complete enough for normal development. The
original manuals, binaries, fixtures, and archived RE notes remain essential
provenance and compatibility resources, but they are no longer the repo's
day-to-day center of gravity.

From here forward:

- oracle and Ghidra work are targeted tools, not a standing project stream
- the default path is to build and refine the Rust engine/client against the
  already documented rule set
- deep RE should reopen only for a concrete compatibility regression, crash,
  unresolved file-safety issue, or player-visible gameplay mismatch
- unresolved low-level quirks that do not materially affect gameplay may stay
  documented without blocking the Rust engine

### Stable docs vs. lab notes

`docs/dev/archive/RE_NOTES.md` is the chronological investigation notebook and
is treated as archival material. `docs/` holds stable, reusable engineering
docs.

### Layered architecture

The Rust workspace is organized as a set of focused crates, each owning a
clear slice of the problem: low-level classic codecs, runtime state and shared
model, the engine/rules boundary, classic import/export compatibility, the
sysop/oracle CLI, the player-facing TUI client, and the scenario test harness.
The implementation stays data-oriented throughout, with explicit record/file
layouts, focused free functions or small impl blocks, and feature-oriented
submodules instead of monolithic source files. For the current crate inventory,
ownership boundaries, and module structure, see
[rust-architecture.md](rust-architecture.md).

### Fixtures to lock in behavior

Preserved fixtures cover the key states that matter for regression testing:
original shipped state, initialized state, post-maintenance state, and
targeted scenario snapshots for specific features.

### Engine outputs over UI playback

`ECMAINT` writes the underlying state and generated report data. `ECGAME` is
still useful, but mainly as a viewer/validation layer for those outputs. When
possible, decode changes in `.DAT` files first and use live report viewing
second. Historical text captures remain reference evidence when live playback
is unavailable or flaky.

For the Rust `ECGAME` clone, the original pre-menu player flow should be
preserved too. Startup is not just a splash or logo -- the full
pre-command-center path should be modeled explicitly: the EC ASCII splash, EC
intro text, first-time onboarding or joined-player review flow, and then the
first command center menu. Startup presentation is game-owned and should live
in the Rust client, with the splash and intro kept stable inside the `80x20`
client model instead of delegated to a sysop config path. When the original
game asks for homeworld naming or new-colony naming before returning to the
menus, that is part of the same login/entry pipeline. The classic sequence
should be preserved while modernizing friction where useful: ANSI/CP437 by
default, cleaner prompt handling, safer input validation, and no fake
monochrome-first experience in the default Rust client.

### How EC was recovered

The Rust engine was not built from guesswork. The current model and docs came
from repeated cross-checking between the original manuals, the original DOS
binaries, preserved fixtures, and controlled Rust-generated scenarios.

| Tool / source | What it was used for | Why it mattered |
| --- | --- | --- |
| Original EC manuals in [`original/v1.5/*.DOC`](../../original/v1.5) | Canonical guide for player-facing rules, setup constraints, turn structure, and terminology | Kept the Rust clone grounded in intended game behavior instead of raw binary quirks alone |
| Ghidra disassembly and headless scripts | Static recovery of file layouts, maint flow, scheduler logic, and helper call structure | Turned opaque Pascal-era code paths into stable Rust-facing specs |
| DOSBox-X debugger, INT 21 tracing, and memory dumps | Dynamic tracing of `ECGAME` / `ECMAINT` behavior, file I/O order, token handling, and live state changes | Proved phase ordering, runtime transitions, and report/output boundaries that static RE alone could not settle |
| Controlled gamestate file diffs | Compared Rust-generated or hand-shaped directories against classic `.DAT` outputs before and after maintenance | Exposed real cross-file invariants and kept the Rust side honest at the compatibility boundary |
| Report and log analysis | Studied `RESULTS.DAT`, `MESSAGES.DAT`, shipped `ec*.txt` logs, and preserved output captures | Recovered player-visible timing, report cadence, `Stardate` behavior, and event sequencing |
| Rust-generated scenarios and oracle sweeps | Created narrow test cases, ran the original binaries as oracle, and promoted repeated outcomes into shared rules | Turned reverse engineering into reusable implementation guidance instead of one-off notes |

For the full RE directory, provenance, and evidence entrypoint, see
[reverse_engineering/README.md](../reverse_engineering/README.md).

### Escalating RE depth

Start with Rust-generated scenarios, preserved fixtures, and black-box
`ECMAINT` acceptance testing. Promote repeated deterministic pass/fail
patterns into shared Rust rules first. Escalate to deep static/dynamic RE only
when all three conditions hold: a path is blocking broader compliant gamestate
generation, black-box testing has plateaued, and the expected rule is reusable
rather than one-off trivia. When deep RE is required, stop once the rule is
explicit enough to promote into Rust -- do not keep digging only to satisfy
curiosity. The Guard Starbase / `unknown starbase` investigation is the
template for a justified deep-dive blocker, not the default workflow for every
mechanic.

### Controlled oracle loops for new mechanics

The default workflow for investigating a new mechanic is to initialize a
controlled directory in Rust or from a preserved baseline, submit one tightly
scoped order family, run `ECMAINT` as the oracle, and diff the resulting
`.DAT`, `MESSAGES.DAT`, `RESULTS.DAT`, and `ERRORS.TXT` files. Only repeated
deterministic effects get promoted into `CoreGameData`. Deep RE is reserved
for after this loop stops yielding reusable rules. DOSBox file-I/O traces
should be treated as coarse phase-boundary evidence only -- they can prove
broad clustering like "heavy fleet-state mutation first, derived-file rebuild
later," but they are not proof of exact movement, economy, or combat ordering
inside a yearly simulation pass.

### Setup and map generation as gameplay semantics

The manuals explicitly define galaxy size by player count and total solar
system count. The Rust builder is useful infrastructure, but it is not
automatically the same thing as a faithful EC game initializer. Setup should be
refined as a manual-driven subsystem covering map dimensions, star count,
homeworld/start rules, and initial fleets and empire payloads.

Default sysop/admin setup should preserve the original pre-join distinction:
joinable new games are not the same thing as post-join campaign baselines.
`ECGAME` onboarding must see inactive player slots and `Not Named Yet`
homeworld seeds. Once a player actually joins a fresh slot, the opening
homeworld should immediately expose the documented starting spendable
production implied by the manuals rather than forcing an extra first-turn
maintenance wait. Automated maint/oracle sweeps may still use a separate
explicit post-join-compatible baseline when that is the thing being tested.
Exact reproduction of the original hidden map RNG is not required to be
faithful; adherence to the documented setup rules is.

### Recovered mechanics vs. canonical routing policy

Movement execution rules should follow recovered deterministic behavior where
known. Route selection and threat-aware navigation may be improved canonically
in Rust when the manuals do not define a detailed routing algorithm. Smart
pathfinding should be documented as a Rust policy layer, not implied to be a
recovered original mechanic.

The same policy applies to hidden movement state. If classic shows that some
diagonal trips complete more conservatively than naive rounded movement, Rust
should preserve that player-facing outcome where it matters. But unresolved
scratch-byte layouts or obscure completion artifacts are not automatically a
fidelity target by themselves.

### Seeded Rust combat inside the oracle's timing framework

The project does not need to reproduce the original opaque combat RNG. Seeded,
reproducible Rust combat remains the canonical replacement. However, combat
outcomes still need to be folded into the oracle-backed maintenance structure:
canonical middle turn order, intra-year `1..52` weekly timing, follow-on
consequences such as retreats, aborts, retargets, bombardment, invasion
resolution, and Fleet Command Center summaries, and late report/output emission
sequencing.

The practical rule is: do not spend RE effort trying to clone Pascal-era
randomness, but do spend RE effort recovering where combat happens in the turn,
when its consequences land on the weekly timeline, and how those consequences
are routed into reports and derived files. Rust should therefore converge on
seeded reproducible combat resolution, oracle-faithful phase placement, and
oracle-faithful weekly event/report scheduling.

### Declarative sysop config over endless setup flags

`ECUTIL`-style setup/admin data is mostly declarative and should eventually
live in KDL rather than only in one-off command flags. The long-term source of
truth for new-game/setup presets should be machine-readable config covering
player count, year, maintenance schedule, sysop options, optional
map-generation seed, and setup mode / starting-state presets. CLI and future
TUI surfaces should act as frontends over that config and the shared Rust
model, not as the only place where setup can be expressed.

### Storage additive to the compatibility boundary

The immediate engineering target remains a full-game-capable Rust maintenance
engine with classic `.DAT` fidelity. Modern storage is now an active adjacent
milestone because the client is starting to need history and richer intel
semantics that the classic files do not encode well. KDL should remain focused
on authored setup/config/scenario input, and turn limits and other Rust-only
campaign policy should stay secondary to the current runtime/export boundary
until that split settles.

SQLite now sits at the runtime center of the Rust stack without replacing the
classic compliance boundary. The intended shape is: `CoreGameData` remains the
canonical in-memory snapshot model, `ecgame.db` is the first-class persisted
source of truth for active campaigns, and classic `.DAT` files remain explicit
import/export projections and oracle artifacts. `ec-client` and normal Rust
maintenance/mutator paths should operate on SQLite runtime state, not on live
`.DAT` mutation paths. `ec-cli db-import` / `db-export` plus explicit classic
materialization helpers are the compatibility bridge for classic directories.
Read-only inspection/report commands should not create `ecgame.db` as a side
effect. Runtime/client views and classic export may use different projections
over the same canonical facts; do not maintain two unrelated intel systems.

Unresolved or partially decoded classic outputs may still be preserved in
compatibility-oriented SQLite tables while the Rust-native model matures.
SQLite must be bundled/self-hosted in the compiled Rust application; sysops and
players should not need a separate SQLite installation.

### Manual-defined economy semantics when replay probing stalls

The manuals clearly define the important economy tradeoff: empire-wide tax
generates yearly production points, newly colonized planets start below
maximum production, lower taxes improve development speed, and starbases
accelerate planetary growth. If the original `ECMAINT` replay path is awkward
to mutate for a narrow economy experiment, prefer a documented canonical Rust
growth rule over indefinite harness fighting. That rule should remain simple,
explicit, monotonic with respect to tax pressure and growth, and auditable in
tests. Original-binary evidence should still refine the rule when available,
but the project does not need to freeze on economy progress waiting for perfect
replay coverage. The current canonical Rust economy rule is documented in
[docs/economics.md](economics.md).

### Diplomacy and hostility as separate concepts

`enemy` is a stored diplomatic relation set by players in `ECGAME`. `hostile`
is the broader maintenance/combat state that determines whether a contact may
escalate into battle. A contact can become hostile because one side has
declared the other an enemy, one side attacks first, one side enters another
empire's defended solar system, or one side enters or leaves a blockaded world.
Rust should model the distinction in docs and code rather than collapsing both
concepts into one permanent shortcut. Where classic `PLAYER.DAT` diplomacy
bytes are known, they are authoritative.

### Surrender as campaign state

The manuals describe surrender and acknowledgement of an emperor as the
political victory condition. The documented `ECGAME` General Command menu does
not include a surrender or resign action, and a live `ECGAME` menu check
confirms that absence. Rust should therefore not invent a surrender UI command
unless stronger evidence appears. Instead, the Rust model should separate
mechanical defeat (destruction of armies, fleets, and planets; fleet defection
after loss of all planets) from political victory (recognition of one empire as
emperor; effective surrender or submission of the remaining empires).

### Compatible gamestate even when behavior is canonicalized

The Rust engine is now far enough along that it should prefer
**classic-compatible save directories** over brittle attempts to mimic every
hidden stochastic or processing-order quirk of the original binaries. For
unresolved or stochastic mechanics, a documented canonical Rust rule is
acceptable if the resulting `.DAT` files remain loadable and sane in original
`ECGAME`, the resulting directories remain structurally acceptable to the
original maintenance/tooling workflow, the rule is faithful to the player
manuals and observed gameplay spirit, and the rule is deterministic, auditable,
and regression-testable.

In practice this means file compatibility remains strict, deterministic
mechanics should still match exactly where practical, and non-deterministic or
under-recovered mechanics may reasonably diverge when the divergence is
explicit, compatible, and more reproducible than the original hidden behavior.

### Own the mechanics; do not reproduce the original RNG stream

`ECMAINT` uses an internal RNG for combat resolution (fleet battles,
bombardment ship losses) and rogue/autopilot AI decisions. The original RNG
output is not reproducible without full emulation of its internal state;
attempting to match it byte-for-byte is intractable and would produce a brittle
clone, not a faithful reimplementation. Instead, the project implements **its
own seeded and reproducible versions** of every mechanic: the original binary
and preserved fixtures are used to understand the *structure* of changes (what
fields change, in what range, under what conditions), and the project defines
its own canonical rules for the *magnitude* of random effects (bombardment ship
losses, battle attrition rates, AI economy choices). Those rules are documented
here and in `docs/dev/archive/RE_NOTES.md` so they are auditable and tunable
independently of the original binary.

The acceptance criterion for these mechanics is internal consistency and
gameplay plausibility, not byte-exact fixture match. The shared campaign seed
belongs to the engine/runtime. The Rust client may derive cosmetic-only
presentation choices from that persisted campaign seed, but those choices must
not feed back into gameplay state or engine RNG ordering. Byte-exact fixture
match remains the acceptance criterion only for fully deterministic mechanics
(movement, year advancement, build queues, economy totals, cross-file linking).
The original post-maint fixtures are still used to understand field ranges and
change patterns; they are not used as a bit-level target for stochastic
mechanics. Once these canonical mechanics stabilize, prefer moving their stable
constants into machine-readable KDL config rather than burying them inline
forever in Rust code.

### Near-term acceptance rule

A format or mechanic is not "done" until Rust can emit the relevant state and
the original binaries accept it without integrity failures or unexpected
normalization. The original `ECMAINT` oracle is therefore a compatibility and
structure oracle first, not a universal semantics oracle. Bit-perfect
post-maint parity is worth pursuing only when it supports the manuals and the
mechanic is deterministic enough for that target to be meaningful.

For stochastic mechanics, "done" means correct field structure, plausible
magnitudes, and a documented canonical rule -- not byte-exact match to any
single oracle run. For manual-driven mechanics whose original binary behavior
is ambiguous, opaque, or stochastic, strict adherence to the manuals is a
better target than reproducing one hidden implementation artifact.

## What Counts As Success

In the short term, success means decoding the important on-disk formats,
reproducing `ECUTIL` behavior faithfully, understanding `ECMAINT` as a
deterministic state transformer, defining the cross-file invariants required
for original-engine acceptance, and generating fully compliant gamestate files
from Rust.

In the long term, the goal is to reimplement the real turn engine in Rust,
build a usable player client and admin client, and support classic-compatible
saves with reproducible results. The original player-facing ANSI presentation
should be preserved well enough to reuse or faithfully recreate the important
opening, menu, and report screens in the Rust client. Eventually the project
should support both classic `.DAT` directory interchange with the DOS binaries
through explicit import/export workflows and the richer SQLite-backed
runtime/history layer already used by the Rust client and maintenance paths,
including per-campaign `ecgame.db` persistence with history, analytics, and
richer player-facing intel views.

## Milestone Ladder

**1. Known accepted scenarios.** Rust can emit preserved accepted pre-maint
scenarios from decoded fields. The original binaries and preserved fixtures are
the acceptance oracle. Current examples include `fleet-order`, `planet-build`,
and `guard-starbase`.

**2. Parameterized scenario generation.** Replace scenario-specific constants
with explicit field builders and validators. Move from "recreate this one
accepted shape" toward "generate families of accepted shapes within known-safe
constraints."

**3. General compliant gamestate generation.** Rust can write a full arbitrary
gamestate directory that `ECMAINT` accepts without integrity failures. This
requires the remaining cross-file linkage rules, especially the starbase/fleet
summary-pairing semantics in `ECMAINT`.

**4. Full Rust maintenance replacement.** Reimplement `ECMAINT` behavior in
Rust with reproducible outputs. Preserve compatibility with original save
directories and reports. Seeded CRT combat is now implemented as a canonical
Rust replacement for the original RNG-driven combat paths, so combat acceptance
is structural and rule-based, not byte-exact to any one oracle run.

**5. Scenario DSL / KDL layer.** Add a human-editable scenario/order format
only after the internal Rust gamestate and order model stabilizes. KDL is
treated as a serialization layer over the compliant generator, not as the next
reverse-engineering target. KDL is still a good long-term fit for stable
machine-readable data: combat/entity constants, setup and baseline presets, and
oracle scenario definitions. Rust remains the authority for maintenance
sequencing and classic save-file compatibility; config should feed stable data
tables, not replace the engine. Future storage layers should follow the same
rule: they may sit beside the classic `.DAT` flow but not replace the
compatibility boundary. The long-term goal is to describe scenarios, describe
per-turn player orders, emit gamestate files, run original `ECMAINT`, and
iterate over a whole game from scripted inputs.

**6. ANSI / UI preservation layer.** Capture and preserve the original
`ECGAME` ANSI output and screens where practical, treating those captures as
reference assets for the Rust client. Prefer exact stream capture when possible
and rendered-screen capture as a fallback. This is not the immediate RE
priority, but it is an explicit preservation goal and should be folded into the
Rust clone once the local `ECGAME` harness is reliable enough.

## RE Workflow

The default loop is: generate or mutate a controlled scenario in Rust, run the
original binary (`ECMAINT`, `ECGAME`, or `ECUTIL`) as the oracle, diff the
resulting `.DAT` files and reports, promote only strong repeated patterns into
`CoreGameData`, and escalate to deep RE only if the rule still blocks
generalization.

## Event And Report Direction

Maintenance-side player-visible consequences should be modeled as typed events
first, and rendered into classic report files second. Report formatting should
not be embedded ad hoc inside mechanic code paths; which crate owns which
report artifact is an architecture concern documented in
[rust-architecture.md](rust-architecture.md). The same
event/report pipeline should eventually cover fleet encounters and retreats,
bombardment/invasion/blitz and starbase defense, colonization
success/failure, scout reconnaissance and contact discovery, and mission
completion/denial outcomes.

### Event modeling policy

The typed maintenance event surface should continue broadening, with all events
pushed through a single report-generation pass. Scout arrival reports should
use the generic mission-outcome backbone first, with richer planet-intel
reconnaissance reports added later. `ScoutSolarSystem` should reuse the
existing `PlanetIntelEvent` / `DATABASE.DAT` refresh path where the current
maintenance model already supports it, and `ViewWorld` should use that same
intel-refresh path rather than creating a separate report-only branch.

When combat forces a fleet off its standing orders, the system should emit a
typed mission `Aborted` outcome from the battle phase instead of hiding that
consequence inside fleet-byte mutations alone. Scout-style hostile contact
detection should be emitted from the battle/contact grouping phase, because
that is where maint has the cleanest simultaneous view of who met whom before
attrition rewrites the board. That contact event family should be
mission-aware so scout, join, rendezvous, and guard/blockade reports can share
one detection path without copy-pasted reporting logic.

### Recipient scoping and loss reporting

Prefer recipient-scoped maintenance events over omniscient report summaries.
Bombardment, fleet battle, scouting/contact, merge, colonization, and mission
outcome reporting should be modeled from the acting or affected empire's point
of view rather than as a global debug narration. Destructive combat
consequences should become first-class events too: fleets and starbases that
are wiped out should emit explicit command-center loss reports rather than
being inferred indirectly from missing units. Where richer specialized report
events exist, prefer them over duplicate generic mission-resolution text;
invade/blitz should not generate two parallel attacker-side reports for the
same assault. Every fleet encounter should eventually emit an intel/contact
event even if no battle occurs; combat is only one possible consequence of
contact.

### Classic report file compatibility

`RESULTS.DAT` is the canonical maint report target. Classic `MESSAGES.DAT`
uses a different on-disk format from `RESULTS.DAT`-style 84-byte chunks and
also carries player-to-player mail with maintenance-gated recipient visibility.
Until the classic mail format is fully recovered, Rust should preserve existing
`MESSAGES.DAT` payloads unchanged and keep Rust-originated queued mail in
SQLite/runtime state rather than writing it into classic mail files.

For the concrete oracle runbooks (black-box loop, replay validation, deep RE
escalation), see
[reverse_engineering/README.md](../reverse_engineering/README.md#oracle-runbooks).
