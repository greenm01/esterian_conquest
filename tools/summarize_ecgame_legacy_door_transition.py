import json
from pathlib import Path


SOURCE = Path("artifacts/ecgame-startup/legacy-door-locals.json")
OUTPUT = Path("artifacts/ecgame-startup/legacy-door-transition.txt")


def parse_frame(sample: dict[str, str]) -> dict[int, int]:
    data = bytes.fromhex(sample["frame_hex"])
    return {
        off - 0x20: int.from_bytes(data[off : off + 2], "little")
        for off in range(0, min(len(data), 0x38), 2)
    }


def main() -> None:
    samples = json.loads(SOURCE.read_text(encoding="utf-8"))
    by_ax = {sample["ax"]: sample for sample in samples}

    key_order = ["3F10", "3FFF", "3F1A"]
    lines: list[str] = []

    for key in key_order:
        sample = by_ax[key]
        frame = parse_frame(sample)
        lines.append(f"{key}")
        lines.append(
            "  regs:"
            f" SI={sample['si']} DI={sample['di']} BP={sample['bp']} SP={sample['sp']}"
            f" CS={sample['cs']} DS={sample['ds']} SS={sample['ss']}"
        )
        for off in (0x00, 0x02, 0x04, 0x06, 0x08, 0x0A, 0x0C, 0x0E, 0x10, 0x12, 0x14, 0x16):
            rel = f"+0x{off:02X}"
            lines.append(f"  [BP{rel}] = 0x{frame.get(off, 0):04X}")
        lines.append("")

    lines.extend(
        [
            "Observed handoff:",
            "- Stable loop phase ends at `3F10` with `[BP+0x0A] = [BP+0x0C] = 0x0011`.",
            "- The next `3FFF` stop uses a different frame shape (`BP=F6A8`, `SP=F68A`, `SI=F8B8`).",
            "- At that handoff, the old loop counter slots are replaced by pointer/data words including `0x403C`, `0x44A1`, and inline `COM` bytes.",
            "- The later `3F1A` stop uses a third frame shape (`BP=F6AE`, `SP=F692`) and no longer carries the `0x0011` loop-limit pair.",
            "- Practical interpretation: `3F10` completes the fixed field-window parser loop, then control transfers into a follow-on phase that repacks parser state before the later `0x1C` exit path.",
        ]
    )

    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(OUTPUT.read_text(encoding="utf-8"))


if __name__ == "__main__":
    main()
