#!/usr/bin/env python3
"""Bootstrap and advance a four-player human harness campaign for TUI inspection.

Coordinator quick-start
=======================

If you are an LLM coordinator, this file is the workflow entrypoint. You do not
need to re-read the harness docs to use the normal four-player human-TUI flow.

What this script owns:
- writing a simple 4-player scenario into `.tmp/campaigns/<game_id>/scenario.kdl`
- initializing or reopening the harness campaign
- claiming players 1..4 for the current open turn
- scanning and applying a fully validated turn batch
- reopening and reclaiming the next turn after maintenance
- printing the live `nc-client` inspect commands and player bundle paths

What the outer LLM coordinator owns:
- read only one player's bundle at a time
- spawn one worker per player for the current turn
- each worker reads only:
  - its `bundle-turn-XXXX/README.md`
  - its local bundle files like `starmap.txt` / `starmap.csv`
  - its own prior notes / turn files if needed
- each worker writes only:
  - `player-N/turn-XXXX.kdl`
  - optional `player-N/notes-XXXX.md`
- once all workers finish, rerun this script in `advance` mode
- if `advance` reports blocking players, rerun only the rejected/missing players

Normal loop:
1. `python3 tools/harness/four_player_human_campaign.py bootstrap --reset`
2. Coordinate player workers for turn 1.
3. `python3 tools/harness/four_player_human_campaign.py advance`
4. Coordinate player workers for turn 2.
5. `python3 tools/harness/four_player_human_campaign.py advance`
6. Coordinate player workers for turn 3.
7. `python3 tools/harness/four_player_human_campaign.py advance`
8. Inspect the live game in the TUI using the printed `nc-client` command.

The script always uses the harness `human` bundle profile. This keeps the player
surface human-visible and avoids hidden `.llm/spatial.kdl` files for this
specific TUI-inspection workflow.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path


SCENARIO_TEMPLATE = """scenario player_count=4 year=3000 baseline="builder-compatible" seed={seed} label="Human TUI Campaign"

house record=1 handle="P1" empire="Aurora"
house record=2 handle="P2" empire="Helios"
house record=3 handle="P3" empire="Vesper"
house record=4 handle="P4" empire="Nadir"
"""


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def rust_dir() -> Path:
    return repo_root() / "rust"


def run_nc_cli(args: list[str], check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cargo", "run", "-q", "-p", "nc-cli", "--", *args],
        cwd=rust_dir(),
        check=check,
        text=True,
        capture_output=True,
    )


def default_paths(game_id: str) -> tuple[Path, Path, Path]:
    base = repo_root() / ".tmp" / "campaigns" / game_id
    scenario_path = base / "scenario.kdl"
    campaign_dir = base / "game"
    workspace_root = repo_root() / ".tmp" / "llm-turns" / game_id
    return scenario_path, campaign_dir, workspace_root


def write_scenario(path: Path, seed: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(SCENARIO_TEMPLATE.format(seed=seed), encoding="utf-8")


def manifest_path(campaign_dir: Path) -> Path:
    return campaign_dir / "nc-bot-campaign.kdl"


def claim_all_players(campaign_dir: Path, players: int) -> None:
    for player in range(1, players + 1):
        result = run_nc_cli(
            [
                "harness",
                "claim-turn",
                "--dir",
                str(campaign_dir),
                "--player",
                str(player),
            ],
            check=False,
        )
        if result.returncode == 0:
            sys.stdout.write(result.stdout)
            continue
        stderr = (result.stderr or "").strip()
        if "already validated" in stderr or "already closed" in stderr:
            sys.stdout.write(f"Player {player}: {stderr}\n")
            continue
        raise SystemExit(
            f"claim-turn failed for player {player}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )


def print_summary(game_id: str, scenario_path: Path, campaign_dir: Path, workspace_root: Path) -> None:
    print("\nCampaign ready.")
    print(f"  game_id={game_id}")
    print(f"  scenario={scenario_path}")
    print(f"  campaign_dir={campaign_dir}")
    print(f"  workspace_root={workspace_root}")
    print("\nPlayer bundles:")
    for player in range(1, 5):
        bundle_dir = workspace_root / f"player-{player}" / "bundle-turn-0001"
        turn_file = workspace_root / f"player-{player}" / "turn-0001.kdl"
        print(f"  player {player}:")
        print(f"    bundle={bundle_dir}")
        print(f"    turn_file={turn_file}")
    print("\nInspect in the TUI:")
    for player in range(1, 5):
        print(f"  cd rust && cargo run -q -p nc-client -- --dir {campaign_dir} --player {player}")


def bootstrap(args: argparse.Namespace) -> None:
    # Bootstrap is the "start or resume this campaign turn" command.
    # It is safe to rerun: if the manifest already exists, it reopens the
    # current turn instead of recreating the campaign.
    scenario_path, campaign_dir, workspace_root = default_paths(args.game_id)
    if args.reset:
        shutil.rmtree(scenario_path.parent, ignore_errors=True)
        shutil.rmtree(workspace_root, ignore_errors=True)

    if not scenario_path.exists() or args.rewrite_scenario:
        write_scenario(scenario_path, args.seed)

    if manifest_path(campaign_dir).exists():
        result = run_nc_cli(["harness", "open-turn", "--dir", str(campaign_dir)])
    else:
        result = run_nc_cli(
            [
                "harness",
                "init-campaign",
                "--file",
                str(scenario_path),
                "--dir",
                str(campaign_dir),
                "--game-id",
                args.game_id,
                "--bundle-profile",
                "human",
            ]
        )
    sys.stdout.write(result.stdout)
    claim_all_players(campaign_dir, 4)
    print_summary(args.game_id, scenario_path, campaign_dir, workspace_root)


def advance(args: argparse.Namespace) -> None:
    # Advance is the "all player turn files should already exist" command.
    # It validates the current batch, applies maintenance if the batch is
    # complete, then immediately opens and claims the next turn.
    _, campaign_dir, _ = default_paths(args.game_id)
    scan = run_nc_cli(["harness", "scan-turn", "--dir", str(campaign_dir)], check=False)
    sys.stdout.write(scan.stdout)
    if scan.returncode != 0:
        raise SystemExit(scan.stderr or "scan-turn failed")
    if "blocking_players=" in scan.stdout and "blocking_players=none" not in scan.stdout:
        raise SystemExit("scan-turn reported blocking players; fix rejected or missing turn files first")

    applied = run_nc_cli(["harness", "apply-turn-batch", "--dir", str(campaign_dir)])
    sys.stdout.write(applied.stdout)

    opened = run_nc_cli(["harness", "open-turn", "--dir", str(campaign_dir)])
    sys.stdout.write(opened.stdout)
    claim_all_players(campaign_dir, 4)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Bootstrap and advance a four-player human harness campaign for TUI inspection."
    )
    parser.add_argument(
        "--game-id",
        default="human-tui-four-player",
        help="Stable harness game id and workspace key.",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=1515,
        help="Scenario seed written into the generated scenario.kdl.",
    )
    subparsers = parser.add_subparsers(dest="cmd", required=False)

    bootstrap_parser = subparsers.add_parser("bootstrap", help="Create/open the campaign and claim players 1..4.")
    bootstrap_parser.add_argument(
        "--reset",
        action="store_true",
        help="Delete the existing local campaign/workspace before bootstrapping.",
    )
    bootstrap_parser.add_argument(
        "--rewrite-scenario",
        action="store_true",
        help="Rewrite the generated scenario.kdl even if it already exists.",
    )
    bootstrap_parser.set_defaults(func=bootstrap)

    advance_parser = subparsers.add_parser(
        "advance", help="Scan the current turn, apply it when fully validated, then open and claim the next turn."
    )
    advance_parser.set_defaults(func=advance)

    parser.set_defaults(func=bootstrap, reset=False, rewrite_scenario=False)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    args.func(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
