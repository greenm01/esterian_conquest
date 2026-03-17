#!/usr/bin/env python3
"""Mine existing file-I/O trace data for step-4 phase-boundary evidence.

Reads a fileio_trace_*.txt artifact and produces:
- Fleet write-pass timeline with per-pass record index sequences
- Pass-boundary detection (where the engine restarts the fleet iteration)
- Cross-file interleave detection within the fleet write block
- DATABASE.DAT slot correlation (offset/100 = planet record index)
- Cross-scenario comparison when multiple traces are available

Usage:
    python3 tools/analyze_fileio_trace.py                         # analyze all traces in artifacts/
    python3 tools/analyze_fileio_trace.py <trace_file>            # analyze a single trace
    python3 tools/analyze_fileio_trace.py --compare               # cross-scenario comparison
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-fileio-trace"

FLEET_RECORD_SIZE = 54
PLANET_RECORD_SIZE = 97
DATABASE_RECORD_SIZE = 100


@dataclass
class IOEvent:
    index: int
    kind: str
    name: str
    detail: str


@dataclass
class FleetWritePass:
    """A single pass through fleet records (one iteration of the fleet table)."""
    pass_number: int
    start_event: int
    end_event: int
    record_indices: list[int]
    interleaved_events: list[IOEvent]


@dataclass
class DatabaseWrite:
    event_index: int
    offset: int
    record_index: int  # offset / DATABASE_RECORD_SIZE
    nbytes: int


@dataclass
class TraceAnalysis:
    scenario: str
    total_events: int
    events: list[IOEvent]
    # Fleet analysis
    fleet_write_block_start: int
    fleet_write_block_end: int
    fleet_write_count: int
    fleet_write_passes: list[FleetWritePass]
    fleet_record_count: int  # number of distinct fleet records
    # Database analysis
    database_writes: list[DatabaseWrite]
    # Phase boundaries
    load_phase_end: int
    validation_phase_end: int
    simulation_phase_start: int
    flush_phase_start: int


def parse_trace(path: Path) -> tuple[str, list[IOEvent]]:
    """Parse a fileio_trace_*.txt file."""
    events: list[IOEvent] = []
    scenario = "unknown"

    for line in path.read_text().splitlines():
        if line.startswith("ECMAINT File I/O Trace"):
            m = re.search(r"scenario=(\S+)", line)
            if m:
                scenario = m.group(1)
            continue
        if line.startswith("Total events:"):
            continue
        if not line.strip():
            continue

        m = re.match(r"\[\s*(\d+)\]\s+(\w+)\s+(\S+)\s*(.*)", line)
        if m:
            events.append(IOEvent(
                index=int(m.group(1)),
                kind=m.group(2),
                name=m.group(3),
                detail=m.group(4).strip(),
            ))

    return scenario, events


def extract_seek_offset(events: list[IOEvent], write_idx: int, file_name: str) -> int | None:
    """Find the seek offset immediately before a write event for a given file."""
    for i in range(write_idx - 1, max(write_idx - 5, -1), -1):
        ev = events[i]
        if ev.name == file_name and ev.kind == "seek":
            m = re.match(r"offset=(\d+)\s+whence=0", ev.detail)
            if m:
                return int(m.group(1))
    return None


def analyze_fleet_writes(events: list[IOEvent]) -> tuple[int, int, list[FleetWritePass], int]:
    """Analyze the fleet write block: find passes, record indices, interleaves."""
    # Find the fleet write block boundaries
    fleet_writes = [(i, ev) for i, ev in enumerate(events) if ev.kind == "write" and ev.name == "FLEETS.DAT"]
    if not fleet_writes:
        return -1, -1, [], 0

    first_write_pos = fleet_writes[0][0]
    last_write_pos = fleet_writes[-1][0]

    # Extract record index for each fleet write using preceding seek offset
    write_records: list[tuple[int, int, int]] = []  # (list_pos, event_index, record_index)
    for list_pos, (ev_pos, ev) in enumerate(fleet_writes):
        offset = extract_seek_offset(events, ev_pos, "FLEETS.DAT")
        if offset is not None:
            record_idx = offset // FLEET_RECORD_SIZE
            write_records.append((list_pos, ev.index, record_idx))
        else:
            write_records.append((list_pos, ev.index, -1))

    # Detect distinct fleet record count
    distinct_records = sorted(set(r for _, _, r in write_records if r >= 0))
    fleet_record_count = len(distinct_records)

    # Detect pass boundaries: a new pass starts when we see a record index
    # that was already seen in the current pass
    passes: list[FleetWritePass] = []
    current_pass_records: list[int] = []
    current_pass_start = write_records[0][1] if write_records else 0
    seen_in_pass: set[int] = set()

    for list_pos, event_idx, record_idx in write_records:
        if record_idx in seen_in_pass and record_idx >= 0:
            # Start a new pass
            passes.append(FleetWritePass(
                pass_number=len(passes) + 1,
                start_event=current_pass_start,
                end_event=event_idx - 1,
                record_indices=list(current_pass_records),
                interleaved_events=[],
            ))
            current_pass_records = [record_idx]
            current_pass_start = event_idx
            seen_in_pass = {record_idx}
        else:
            current_pass_records.append(record_idx)
            if record_idx >= 0:
                seen_in_pass.add(record_idx)

    # Final pass
    if current_pass_records:
        passes.append(FleetWritePass(
            pass_number=len(passes) + 1,
            start_event=current_pass_start,
            end_event=write_records[-1][1],
            record_indices=list(current_pass_records),
            interleaved_events=[],
        ))

    # Find interleaved non-fleet events within the fleet write block
    first_ev = fleet_writes[0][1].index
    last_ev = fleet_writes[-1][1].index

    for ev in events:
        if first_ev <= ev.index <= last_ev and ev.name != "FLEETS.DAT":
            # Find which pass this belongs to
            for p in passes:
                if p.start_event <= ev.index <= p.end_event:
                    p.interleaved_events.append(ev)
                    break

    return first_ev, last_ev, passes, fleet_record_count


def analyze_database_writes(events: list[IOEvent]) -> list[DatabaseWrite]:
    """Extract DATABASE.DAT writes with record index correlation."""
    writes: list[DatabaseWrite] = []
    for i, ev in enumerate(events):
        if ev.kind == "write" and ev.name == "DATABASE.DAT":
            nbytes = int(ev.detail.split()[0]) if ev.detail else 0
            offset = extract_seek_offset(events, i, "DATABASE.DAT")
            if offset is not None:
                writes.append(DatabaseWrite(
                    event_index=ev.index,
                    offset=offset,
                    record_index=offset // DATABASE_RECORD_SIZE,
                    nbytes=nbytes,
                ))
    return writes


def find_phase_boundaries(events: list[IOEvent]) -> tuple[int, int, int, int]:
    """Identify coarse phase boundaries from file access patterns."""
    # Load phase ends when initial sequential reads complete
    # (first time we see a non-sequential file access pattern)
    load_end = 0
    validation_end = 0
    sim_start = 0
    flush_start = len(events) - 1

    # Find MOVE.TOK close — marks transition from validation to simulation
    for ev in events:
        if ev.kind == "close" and ev.name == "MOVE.TOK":
            sim_start = ev.index
            validation_end = ev.index - 1
            break

    # Find first write that's not FLEETS.DAT — marks transition to flush
    # Actually, find the final sequential PLAYER.DAT write block
    for ev in reversed(events):
        if ev.kind == "write" and ev.name == "PLAYER.DAT":
            flush_start = ev.index
            break

    # Load phase: from start to first re-open of a previously-seen file
    opened: set[str] = set()
    for ev in events:
        if ev.kind == "open":
            if ev.name in opened:
                load_end = ev.index - 1
                break
            opened.add(ev.name)

    return load_end, validation_end, sim_start, flush_start


def analyze_trace(path: Path) -> TraceAnalysis:
    """Full analysis of a single trace file."""
    scenario, events = parse_trace(path)
    fleet_start, fleet_end, passes, fleet_count = analyze_fleet_writes(events)
    db_writes = analyze_database_writes(events)
    load_end, val_end, sim_start, flush_start = find_phase_boundaries(events)

    return TraceAnalysis(
        scenario=scenario,
        total_events=len(events),
        events=events,
        fleet_write_block_start=fleet_start,
        fleet_write_block_end=fleet_end,
        fleet_write_count=sum(1 for ev in events if ev.kind == "write" and ev.name == "FLEETS.DAT"),
        fleet_write_passes=passes,
        fleet_record_count=fleet_count,
        database_writes=db_writes,
        load_phase_end=load_end,
        validation_phase_end=val_end,
        simulation_phase_start=sim_start,
        flush_phase_start=flush_start,
    )


def format_analysis(analysis: TraceAnalysis) -> str:
    """Format analysis results as readable text."""
    lines: list[str] = []
    lines.append("=" * 100)
    lines.append(f"FILE I/O TRACE ANALYSIS — scenario={analysis.scenario}")
    lines.append("=" * 100)
    lines.append("")

    # Phase boundaries
    lines.append("PHASE BOUNDARIES:")
    lines.append(f"  Load phase:       events 0 .. {analysis.load_phase_end}")
    lines.append(f"  Validation phase: events {analysis.load_phase_end + 1} .. {analysis.validation_phase_end}")
    lines.append(f"  Simulation phase: events {analysis.simulation_phase_start} .. {analysis.flush_phase_start - 1}")
    lines.append(f"  Flush phase:      events {analysis.flush_phase_start} .. {analysis.total_events - 1}")
    lines.append("")

    # Fleet write summary
    lines.append("FLEET WRITE BLOCK:")
    lines.append(f"  Event range:    {analysis.fleet_write_block_start} .. {analysis.fleet_write_block_end}")
    lines.append(f"  Total writes:   {analysis.fleet_write_count}")
    lines.append(f"  Fleet records:  {analysis.fleet_record_count}")
    lines.append(f"  Detected passes: {len(analysis.fleet_write_passes)}")
    if analysis.fleet_record_count > 0:
        expected_passes = analysis.fleet_write_count // analysis.fleet_record_count
        lines.append(f"  Expected passes: {analysis.fleet_write_count} / {analysis.fleet_record_count} = {expected_passes}")
    lines.append("")

    # Per-pass detail
    lines.append("FLEET WRITE PASSES (record index sequences):")
    lines.append("")
    for p in analysis.fleet_write_passes:
        record_str = ",".join(str(r) for r in p.record_indices)
        lines.append(f"  Pass {p.pass_number:3d}: events [{p.start_event:4d}..{p.end_event:4d}]  records=[{record_str}]")
        if p.interleaved_events:
            for ie in p.interleaved_events:
                lines.append(f"           INTERLEAVE: [{ie.index:4d}] {ie.kind:<6s} {ie.name:<20s} {ie.detail}")
    lines.append("")

    # Check pass consistency: do all passes visit the same record set in the same order?
    if len(analysis.fleet_write_passes) >= 2:
        first_order = analysis.fleet_write_passes[0].record_indices
        all_same = all(p.record_indices == first_order for p in analysis.fleet_write_passes)
        lines.append("FLEET PASS ORDER CONSISTENCY:")
        if all_same:
            lines.append(f"  All {len(analysis.fleet_write_passes)} passes visit records in the SAME order: [{','.join(str(r) for r in first_order)}]")
        else:
            lines.append(f"  Passes visit records in DIFFERENT orders!")
            distinct_orders: dict[str, list[int]] = {}
            for p in analysis.fleet_write_passes:
                key = ",".join(str(r) for r in p.record_indices)
                distinct_orders.setdefault(key, []).append(p.pass_number)
            for order, pass_nums in distinct_orders.items():
                lines.append(f"    Order [{order}]: passes {pass_nums}")
        lines.append("")

    # Database write analysis
    if analysis.database_writes:
        lines.append("DATABASE.DAT WRITES:")
        lines.append(f"  Total writes: {len(analysis.database_writes)}")
        lines.append(f"  Record indices (offset/100): {[dw.record_index for dw in analysis.database_writes]}")
        lines.append("")
        for dw in analysis.database_writes:
            lines.append(
                f"  [{dw.event_index:4d}] offset={dw.offset:5d}  "
                f"record_index={dw.record_index:3d} (planet)  "
                f"bytes={dw.nbytes}"
            )
        lines.append("")

    # Cross-file event timeline within the simulation phase
    lines.append("SIMULATION PHASE FILE ACCESS TIMELINE:")
    lines.append("(non-seek events only, grouped by file)")
    lines.append("")

    sim_events = [
        ev for ev in analysis.events
        if analysis.simulation_phase_start <= ev.index < analysis.flush_phase_start
        and ev.kind in ("read", "write", "open", "close", "create")
    ]

    current_file = ""
    current_kind = ""
    current_count = 0
    current_start = 0

    def flush_group():
        nonlocal current_file, current_kind, current_count, current_start
        if current_count > 0:
            if current_count == 1:
                lines.append(f"  [{current_start:4d}] {current_kind:<6s} {current_file}")
            else:
                lines.append(f"  [{current_start:4d}] {current_kind:<6s} {current_file}  x{current_count}")
        current_count = 0

    for ev in sim_events:
        if ev.name != current_file or ev.kind != current_kind:
            flush_group()
            current_file = ev.name
            current_kind = ev.kind
            current_start = ev.index
            current_count = 1
        else:
            current_count += 1
    flush_group()

    return "\n".join(lines) + "\n"


def format_comparison(analyses: list[TraceAnalysis]) -> str:
    """Cross-scenario comparison."""
    lines: list[str] = []
    lines.append("=" * 100)
    lines.append("CROSS-SCENARIO COMPARISON")
    lines.append("=" * 100)
    lines.append("")

    # Summary table
    headers = ["Scenario", "Events", "Fleet Writes", "Fleet Recs", "Passes", "DB Writes", "Sim Start"]
    widths = [20, 8, 13, 11, 8, 10, 10]
    header_line = "  ".join(h.ljust(w) for h, w in zip(headers, widths))
    lines.append(header_line)
    lines.append("-" * len(header_line))

    for a in analyses:
        row = [
            a.scenario,
            str(a.total_events),
            str(a.fleet_write_count),
            str(a.fleet_record_count),
            str(len(a.fleet_write_passes)),
            str(len(a.database_writes)),
            str(a.simulation_phase_start),
        ]
        lines.append("  ".join(r.ljust(w) for r, w in zip(row, widths)))

    lines.append("")

    # Fleet write-pass count comparison
    lines.append("FLEET WRITE PASS ANALYSIS:")
    for a in analyses:
        if a.fleet_record_count > 0:
            ratio = a.fleet_write_count / a.fleet_record_count
            lines.append(
                f"  {a.scenario:20s}: {a.fleet_write_count} writes / {a.fleet_record_count} records "
                f"= {ratio:.1f} passes"
            )
    lines.append("")

    # First-write ordering comparison
    lines.append("FIRST WRITE ORDERING (by file):")
    for a in analyses:
        first_writes: dict[str, int] = {}
        for ev in a.events:
            if ev.kind == "write" and ev.name not in first_writes:
                first_writes[ev.name] = ev.index
        ordered = sorted(first_writes.items(), key=lambda x: x[1])
        order_str = " -> ".join(f"{name}[{idx}]" for name, idx in ordered)
        lines.append(f"  {a.scenario:20s}: {order_str}")
    lines.append("")

    # Database write planet indices comparison
    lines.append("DATABASE.DAT PLANET INDICES:")
    for a in analyses:
        planet_indices = [dw.record_index for dw in a.database_writes]
        lines.append(f"  {a.scenario:20s}: {planet_indices}")
    lines.append("")

    # Fleet record order consistency across scenarios
    lines.append("FLEET RECORD VISIT ORDER (first pass):")
    for a in analyses:
        if a.fleet_write_passes:
            records = a.fleet_write_passes[0].record_indices
            lines.append(f"  {a.scenario:20s}: [{','.join(str(r) for r in records)}]")
    lines.append("")

    # Interleave presence comparison
    lines.append("INTERLEAVED EVENTS WITHIN FLEET WRITE BLOCK:")
    for a in analyses:
        total_interleaves = sum(len(p.interleaved_events) for p in a.fleet_write_passes)
        if total_interleaves > 0:
            lines.append(f"  {a.scenario:20s}: {total_interleaves} interleaved events")
            for p in a.fleet_write_passes:
                for ie in p.interleaved_events:
                    lines.append(f"    pass {p.pass_number}, [{ie.index:4d}] {ie.kind} {ie.name} {ie.detail}")
        else:
            lines.append(f"  {a.scenario:20s}: none")

    return "\n".join(lines) + "\n"


def main() -> int:
    args = sys.argv[1:]

    if args and args[0] == "--compare":
        # Cross-scenario comparison of all available traces
        traces = sorted(ARTIFACT_DIR.glob("fileio_trace_*.txt"))
        if not traces:
            print(f"No traces found in {ARTIFACT_DIR}")
            return 1
        analyses = [analyze_trace(t) for t in traces]
        result = format_comparison(analyses)
        print(result)
        out_path = ARTIFACT_DIR / "cross_scenario_comparison.txt"
        out_path.write_text(result)
        print(f"Written to: {out_path}")
        return 0

    if args:
        # Analyze a single trace file
        traces = [Path(a) for a in args]
    else:
        # Analyze all traces
        traces = sorted(ARTIFACT_DIR.glob("fileio_trace_*.txt"))

    if not traces:
        print(f"No traces found in {ARTIFACT_DIR}")
        return 1

    for trace_path in traces:
        analysis = analyze_trace(trace_path)
        result = format_analysis(analysis)
        print(result)

        out_name = trace_path.stem.replace("fileio_trace_", "analysis_") + ".txt"
        out_path = ARTIFACT_DIR / out_name
        out_path.write_text(result)
        print(f"Written to: {out_path}")
        print()

    return 0


if __name__ == "__main__":
    sys.exit(main())
