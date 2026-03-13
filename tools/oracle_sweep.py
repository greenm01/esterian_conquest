#!/usr/bin/env python3
"""
Oracle sweep script for Milestone 3 Phase 4.

Generates diverse gamestates using the Rust GameStateBuilder and validates
them against the original ECMAINT.EXE binary via the oracle harness.
"""

import subprocess
import tempfile
import shutil
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import List, Tuple
import random

# Repository root
REPO_ROOT = Path(__file__).parent.parent
RUST_DIR = REPO_ROOT / "rust"

@dataclass
class SweepResult:
    name: str
    player_count: int
    year: int
    passed_preflight: bool
    passed_oracle: bool
    errors: List[str]


def generate_gamestate(target_dir: Path, player_count: int, year: int, coords: List[Tuple[int, int]]) -> Tuple[bool, List[str]]:
    """Generate a gamestate using ec-cli generate-gamestate command."""
    coord_args = [f"{x}:{y}" for x, y in coords]
    cmd = [
        "cargo", "run", "-q", "-p", "ec-cli", "--",
        "generate-gamestate", str(target_dir),
        str(player_count), str(year)
    ] + coord_args
    
    result = subprocess.run(
        cmd,
        cwd=RUST_DIR,
        capture_output=True,
        text=True
    )
    
    # Check if generation succeeded
    if result.returncode != 0:
        return False, [f"Generation failed: {result.stderr}"]
    
    # Check preflight validation in output
    passed_preflight = "Preflight validation: OK" in result.stdout
    errors = []
    if not passed_preflight:
        # Extract error lines
        for line in result.stdout.split('\n'):
            if line.strip().startswith('-'):
                errors.append(line.strip()[2:])
    
    return passed_preflight, errors


def run_oracle(target_dir: Path) -> Tuple[bool, List[str]]:
    """Run ECMAINT oracle on the generated gamestate."""
    cmd = [
        "python3", "tools/ecmaint_oracle.py", "run", str(target_dir)
    ]
    
    result = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        capture_output=True,
        text=True
    )
    
    # Check for actual ECMAINT errors (not just word presence)
    stdout_lower = result.stdout.lower()
    errors = []
    
    # Look for actual error indicators, not just the word "error"
    # "ECMAINT oracle run complete" contains both words but isn't an error
    if "integrity" in stdout_lower and "abort" in stdout_lower:
        errors.append("ECMAINT integrity abort detected")
    if "failed" in stdout_lower:
        errors.append("ECMAINT failure detected")
    if result.stdout.count("ERROR") > 1:  # More than just log noise
        errors.append("Multiple ERROR entries detected")
    
    return len(errors) == 0, errors


def sweep_diverse_configurations(count: int = 10) -> List[SweepResult]:
    """Generate and test diverse gamestate configurations."""
    results = []
    
    # Predefined diverse configurations
    configurations = [
        # (player_count, year, homeworld_coords)
        (1, 3000, [(16, 13)]),
        (1, 3001, [(20, 20)]),
        (2, 3000, [(16, 13), (30, 6)]),
        (2, 3001, [(10, 10), (25, 25)]),
        (3, 3000, [(16, 13), (30, 6), (2, 25)]),
        (3, 3001, [(5, 5), (15, 15), (25, 25)]),
        (4, 3000, [(16, 13), (30, 6), (2, 25), (26, 26)]),
        (4, 3001, [(8, 8), (16, 16), (24, 24), (32, 32)]),
        (4, 3050, [(16, 13), (30, 6), (2, 25), (26, 26)]),
        (2, 3100, [(0, 0), (31, 31)]),
    ]
    
    for i, (player_count, year, coords) in enumerate(configurations[:count]):
        name = f"config_{player_count}p_{year}"
        
        with tempfile.TemporaryDirectory() as tmpdir:
            target = Path(tmpdir) / name
            
            # Generate gamestate
            passed_preflight, gen_errors = generate_gamestate(
                target, player_count, year, coords
            )
            
            if not passed_preflight:
                results.append(SweepResult(
                    name=name,
                    player_count=player_count,
                    year=year,
                    passed_preflight=False,
                    passed_oracle=False,
                    errors=gen_errors
                ))
                print(f"✗ {name}: FAILED preflight - {gen_errors}")
                continue
            
            # Run oracle
            passed_oracle, oracle_errors = run_oracle(target)
            
            results.append(SweepResult(
                name=name,
                player_count=player_count,
                year=year,
                passed_preflight=True,
                passed_oracle=passed_oracle,
                errors=oracle_errors
            ))
            
            if passed_oracle:
                print(f"✓ {name}: PASSED")
            else:
                print(f"✗ {name}: FAILED oracle - {oracle_errors}")
    
    return results


def main():
    print("=" * 60)
    print("Milestone 3 Phase 4: Oracle Sweep")
    print("=" * 60)
    print()
    
    # Run sweep
    results = sweep_diverse_configurations(count=10)
    
    # Summary
    print()
    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    
    total = len(results)
    passed = sum(1 for r in results if r.passed_oracle)
    failed = total - passed
    
    print(f"Total configurations tested: {total}")
    print(f"Passed: {passed}")
    print(f"Failed: {failed}")
    print(f"Success rate: {passed/total*100:.1f}%")
    print()
    
    if failed > 0:
        print("FAILED CONFIGURATIONS:")
        for r in results:
            if not r.passed_oracle:
                print(f"  - {r.name}: {r.errors}")
        print()
        return 1
    else:
        print("✓ All configurations passed ECMAINT oracle validation!")
        return 0


if __name__ == "__main__":
    sys.exit(main())
