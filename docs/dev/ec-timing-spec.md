# ECMAINT Timing / Stardate Notes

This document captures the current stable timing-related findings for
`ECMAINT.EXE`.

It is intentionally narrower than a full maintenance-phase spec. The goal here
is to separate what is already supported by historical logs and static RE from
what still needs deeper report-writer recovery.

## Settled So Far

- player-facing `Stardate` values are not just yearly labels
- historical player logs show a `1..52` in-year component
- historical logs also show year rollover from `52/YYYY` to `1/YYYY+1`
- multiple report families use the same day/year form:
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

## Open Questions

- where the real report/rankings `Stardate: D/YYYY` text is formatted
- whether the day value is persisted on disk or only carried in scratch/runtime
  state during maintenance
- which `CONQUEST.DAT` fields feed the maintenance schedule gate
- how the internal `1..52` tick is assigned to specific movement/combat/report
  events within a yearly maintenance run

## Working Model

Current best model:

- each player round is still one year, matching the manuals
- within that yearly maintenance pass, `ECMAINT` appears to model a 52-step
  internal timeline
- the leading semantic interpretation of that timeline is week-of-year
- reports are timestamped with event ticks inside that yearly timeline, not
  just "the date maintenance ran"

That model is well supported by the shipped logs, but the exact report-writer
implementation path is still open.
