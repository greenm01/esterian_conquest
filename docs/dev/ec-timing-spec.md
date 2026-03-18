# ECMAINT Timing / Stardate Notes

This document captures the current stable timing-related findings for
`ECMAINT.EXE`.

It is intentionally narrower than a full maintenance-phase spec. The goal here
is to separate what is already supported by historical logs and static RE from
what still needs deeper report-writer recovery.

This includes both:

- the internal `1..52` timing model behind report emission
- the player-visible `Stardate` header contract that classic reports render

For the broader recovered phase ordering, see
[ec-turn-cycle-spec.md](/home/mag/dev/esterian_conquest/docs/dev/ec-turn-cycle-spec.md).
For phase placement in the Rust target engine, also see
[rust-turn-cycle-implementation.md](/home/mag/dev/esterian_conquest/docs/dev/rust-turn-cycle-implementation.md).

## Settled So Far

- player-facing `Stardate` values are not just yearly labels
- historical player logs show a `1..52` in-year component
- historical logs also show year rollover from `52/YYYY` to `1/YYYY+1`
- the `1..52` scale is not just narrative flavor text:
  - preserved fleet logs show mission outcomes landing at specific in-year
    stardates
  - one fleet can emit multiple ordered reports within the same year at
    different stardates as a mission progresses
- multiple report families use the same week/year form:
  - fleet reports
  - planet reports
  - starbase reports
  - Fleet Command Center reports

Examples from the shipped historical captures:

- `ec8.txt` reaches `Stardate: 52/3009`
- `ec9.txt` begins with `Stardate: 1/3010`
- `ec47.txt` reaches `Stardate: 52/3049`
- `ec48.txt` begins with `Stardate: 1/3052`

This is strong evidence for a real internal yearly tick scale rather than a
purely cosmetic year stamp. The leading semantic interpretation is now
"week-of-year", since `52` fits weeks much better than literal days or months.

## Report Header Contract

The classic player-visible report shape is now clear enough to treat as a
formatting requirement, not just an implementation detail.

Settled presentation rule:

- every classic player report family carries `Stardate: <week>/<year>` on the
  **first
  line** of the report
- that `Stardate` text is **right-justified** on the first line, after the
  source clause, rather than emitted as its own separate header line
- Rust should preserve that classic first-line shape for fleet, planet,
  starbase, and Fleet Command Center reports
- the week field is rendered in classic EC notation:
  - integer `1..52`
  - no zero padding
- the year field is rendered as a four-digit year
- this weekly first-line rule applies to player report entries, not to the
  separate rankings banner text
- preserved rankings headers use a separate year-only form:
  `Stardate: YYYY A.D.`

Examples from the shipped historical logs:

```text
 -> From your 1st Fleet, located in System(13,15)          Stardate: 32/3001
 -> From planet "you're my bitch" in System(23,5):          Stardate: 1/3021
```

Practical meaning:

- the surface format is `Stardate: <week>/<year>`
- the leading field behaves like a week-of-year tick, not a literal calendar
  month
- do not emit Rust report timestamps on a separate line above the report body
- do not left-align `Stardate:` directly after the source phrase with no
  spacing; preserve the classic padded first-line look
- the exact formatter routine is still not statically recovered, but the
  output contract is strong enough to implement directly

## Strongest Behavioral Evidence

The clearest preserved evidence comes from the shipped campaign text captures.

In `ec.txt` (`3001 A.D.`), the starting fleet review shows:

- `1st Fleet` colonize `(13,15)` with `Travel Time: 1 year`
- `2nd Fleet` colonize `(20,11)` with `Travel Time: 2 years`
- `3rd Fleet` view `(23,5)` with `Travel Time: 2 years`

Then `ec2.txt` later shows the resulting unread reports:

- `1st Fleet` colonization at `Stardate: 32/3001`
- `4th Fleet` starbase-guard arrival at `Stardate: 1/3002`
- `3rd Fleet` viewing mission report at `Stardate: 12/3002`
- `3rd Fleet` later move-complete report at `Stardate: 21/3002`
- `2nd Fleet` colonization at `Stardate: 25/3002`

That is strong evidence for a real intra-year scheduler:

- the same maintenance year can contain multiple ordered event times
- the same fleet can generate more than one report at different in-year ticks
- those ticks line up with mission progress, not just with a single decorative
  timestamp chosen after the fact

Practical conclusion:

- `ECMAINT` is using a real sub-year event timeline for movement/report
  sequencing
- the leading semantic interpretation remains week-of-year
- the open question is now the exact implementation of that weekly scheduler,
  not whether it exists

## Full Log Corpus Patterns

The shipped `ec*.txt` report logs also show stable ordering rules across the
whole corpus.

Current aggregate sweep:

- `47` log files with `735` timestamped events
- every file is strictly nondecreasing by `(year, week)`
- `5` files span multiple report years, which shows unread reports can persist
  into later login sessions

The reusable aggregate report is generated by:

```bash
python3 tools/analyze_ec_report_logs.py
```

Current output:

- `artifacts/ec-report-log-analysis.txt`

High-signal sequencing patterns:

- same-week bundles are common for one source
  - especially `sensor contact -> identification -> interception`
  - these appear to be emitted as one weekly batch
- same-week ordering is stable rather than arbitrary
  - the aggregate corpus now shows `38` direct `sensor contact ->
    identification` pairs
  - it also shows repeated longer chains including
    `sensor contact -> identification -> interception`
- multi-week sequences from the same source are also common
  - examples include `extended orbit` followed by later contact/update reports
  - this strongly supports mission progress advancing across multiple in-year
    ticks
- adjacent report timing is concentrated at zero or one week of separation
  - current corpus counts:
    - `350` adjacent transitions with week-gap `0`
    - `67` adjacent transitions with week-gap `1`
  - this fits a real ordered weekly event stream much better than post-hoc
    decorative timestamps
- Fleet Command Center reports are interleaved into the same weekly order
  - they usually read like administrative loss summaries after combat or
    interception outcomes
  - they are not a separate out-of-band yearly appendix
- targeted recurring transitions also show immediate follow-on consequences in
  that same ordered stream:
  - `identified -> fleet-lost` same week: `4x`
  - `fleet-lost -> join-retarget` same week: `2x`
  - `fleet-lost -> planet-bombarded` same week: `4x`
  - `intercepted -> planet-bombarded` next week: `3x`

Practical conclusion:

- report ordering is structured, not arbitrary
- week values are participating in event batching and progression
- the corpus supports a real sub-year scheduler with same-week and cross-week
  report phases

## Current Static Anchors

Static timing-focused analysis currently has these anchors:

- `2000:6fc6`
  - string cluster containing:
    `Today is ' - maintenance is not scheduled to run.`
  - this is the clearest current anchor for the maintenance schedule/date gate
- `2000:945b`
  - currently labeled `ecmaint_emit_timestamp_message_helper`
  - current evidence says this helper belongs to the token/schedule path, not
    the player report `Stardate` path
- `3000:39dc`
  - current time-query helper candidate from earlier token-anchor work
  - still not semantically decoded
- `3000:189c`
  - ranking-generation string cluster:
    `Enabling player-ranking text file generation...`
  - useful as a rankings-output anchor, but still not tied to a decoded
    stardate formatter path

The reusable headless extraction for this work is:

```bash
tools/run_ghidra_script.sh ecmaint-live ECMaintTimingFlow.java
```

Its current output is written to:

- `artifacts/ghidra/ecmaint-live/timing-flow.txt`

The follow-up direct-reference sweep is:

```bash
tools/run_ghidra_script.sh ecmaint-live ECMaintTimingRefs.java
```

Its current output is written to:

- `artifacts/ghidra/ecmaint-live/timing-refs.txt`

## Important Correction

The first timing-focused pass changed one earlier assumption:

- `2000:945b` has only four direct call sites in the current live dump
- all four sit in the token/schedule region around `0x97xx..0x9exx`
- none of those call sites are currently tied to player report generation

Practical interpretation:

- `2000:945b` is currently best treated as a current-date/status formatter used
  during maintenance scheduling/token handling
- do not treat it as the recovered player-report `Stardate` emitter yet
- `2000:6fc6` and `3000:189c` currently show no direct xrefs in the live dump,
  so the schedule/rankings strings are probably reached through indirect
  string-table handling rather than simple immediate references
- the startup `main.tok` / `Creating main work file...` / `Merging joint
  fleets...` cluster at `2000:841b..855a` also currently has no direct scalar
  xrefs in the live dump
  - current best interpretation is the same: the outer startup driver is likely
    reaching those messages through an indirect string/pointer path rather than
    inline `MOV DI, imm16` references

## Already Closed

- `ECMAINT` uses a real internal `1..52` yearly timeline rather than a
  decorative year-only timestamp
- the leading `Stardate` field in classic player reports is week-of-year, not
  month-of-year
- classic player report entries render `Stardate: <week>/<year>` as a
  right-justified first-line fragment
- preserved rankings headers use a separate year-only `Stardate: YYYY A.D.`
  banner form
- the weekly scheduler has an explicit late `1..52` loop with decode,
  timing-window derivation, and accept/reject testing
- timing-window constants for the recovered late scheduler path are bounded
- `CONQUEST.DAT[0x03..0x09]` is the maintenance-schedule block consulted by the
  outer schedule gate
- for implementation depth, the schedule gate is now best treated as:
  - current day-of-week query
  - `CONQUEST.DAT[0x03..0x09]` raw schedule bytes
  - no additional `CONQUEST.DAT` control field is currently required by the
    recovered model
- `2000:945b` is best treated as a schedule/status timestamp helper, not the
  player-report `Stardate` formatter
- player-report generation sits in the late output tail reached from
  `2000:8652 -> 2000:1da6 -> 2000:0c06 -> 2000:56be`
- rankings generation sits in the optional late output branch
  `2000:8665 -> 2000:7659`
- no recovered dedicated core `.DAT` field stores the weekly tick:
  - the recovered scheduler computes week placement in runtime/scratch state
  - the visible week then persists in emitted text outputs rather than as a
    named durable campaign-state field

## Implementation-Relevant Open Questions

- what exact semantic families feed the remaining non-durable local timing
  codes `1` and `2`
- how the internal `1..52` tick is assigned to specific
  movement/combat/report events within a yearly maintenance run

These are the remaining timing questions that still matter directly for the
Rust clone, because they affect visible `Stardate` values and weekly report
ordering.

## Low-Value Remaining RE Trivia

- which exact helper(s) inside the late player-report tail append the
  `Stardate: <week>/<year>` header fragment
- which exact helper(s) inside the optional rankings branch emit the year-only
  `Stardate: YYYY A.D.` banner text

These are still mildly interesting RE targets, but they do not block Rust
implementation now that the output contract and phase placement are already
recovered.

## Next RE Targets

- relate remaining local timing codes `1` and `2` back to concrete report or
  event families
- tighten the mapping from concrete event families to same-week vs later-week
  placement inside the `1..52` scheduler

## Strongest Late-Scheduler Static Path

The current strongest static timing seam is no longer just `1000:a26e` by
itself.

Recovered late weekly chain:

- `0000:127A..1361`
  - explicit outer `1..52` loop
- `0000:1333 -> 0000:02c0`
  - decodes active kind-`1` summary entries through `2000:c067`
  - seeds large stack-resident local timing state
- `0000:1339 -> 1000:a26e`
  - this is a mid-function entry inside `1000:9fa1`
  - it walks a local `0x0a`-byte code table and derives two timing-window
    families
- `1000:c102 -> 1000:9fa1 -> 1000:9c0e`
  - the same timing worker family is then consumed by `1000:c102`
  - `c102` calls `9c0e` twice with selector args `2` then `1`
  - current best reading is that `c102/9c0e` score the current week candidate
    against the computed timing windows and set a rejection flag when the slot
    is outside the acceptable range

The timing-code mapping still recovered in `a26e` remains:

- code `1` -> `+2`
- code `2` -> `+7`
- code `3` -> `+0x15`
- code `8` -> `+0x1e`
- codes `4..7` -> `+0`

Practical interpretation:

- the late weekly side now looks like an explicit placement mechanism with
  decode, window-derivation, and accept/reject testing
- this is stronger than the earlier "maybe offset shaping" read
- the preserved `0000:02c0` dispatch now bounds code `7` more tightly:
  - it is assigned only in the kind-`3` branch
  - the archived summary-dispatch RE already identifies kind `3` as the
    `IPBM` summary family
- whole-image timing-entry scans now also bound code `8` more tightly:
  - consumer-side helpers still compare against code `8`
  - no preserved ES-side writer feeds timing-entry code `8`
  - the only newly found `[-0x0a]` writes are SS-local scratch writes,
    not entry-table writes
- so code `8` is best treated as an unfed consumer-side case in the
  preserved image, not a reachable timing class
- the remaining semantic unknown is therefore narrower:
  - what exact semantic classes feed the remaining non-durable local timing
    codes `1` and `2`
  - not whether the late scheduler has real weekly selection logic

## Working Model

Current best model:

- each player round is still one year, matching the manuals
- within that yearly maintenance pass, `ECMAINT` appears to model a 52-step
  internal timeline
- the leading semantic interpretation of that timeline is week-of-year
- reports are timestamped with event ticks inside that yearly timeline, not
  just "the date maintenance ran"
- those timestamps are rendered in classic player reports as a right-justified
  first-line `Stardate: <week>/<year>` header fragment
- this timeline is mechanically relevant to mission/report sequencing, not
  just decorative report narration

That model is well supported by the shipped logs, but the exact report-writer
implementation path is still open.
