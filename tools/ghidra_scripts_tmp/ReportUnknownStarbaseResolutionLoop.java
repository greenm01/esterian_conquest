import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseResolutionLoop extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-resolution-loop.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Resolution Loop");
            out.println();
            out.println("- Focus: later starbase-specific block `0000:42d8..456e`.");
            out.println("- Goal: summarize how the post-predicate path scans summaries and");
            out.println("  uses `3525` / `3521` in later starbase resolution/reporting.");
            out.println();

            out.println("## Main Scan / Structural Match");
            out.println();
            dumpRange(out, "0000:42d8", "0000:4428");
            out.println();
            out.println("Interpretation:");
            out.println("- clears `350c` up front and iterates active summaries through `2f72`");
            out.println("- requires summary identity on:");
            out.println("  - `+0x00 == [3504]`");
            out.println("  - `+0x01 == [350d]`");
            out.println("  - `+0x02 == [350e]`");
            out.println("  - `+0x05 == f(351b..351f)` via `0x3000:489d`");
            out.println("- rejects direct `+0x0A == [3502]` matches before the deeper path");
            out.println("- decodes candidate summary `+0x06` with `0x2000:c067`");
            out.println("- deeper acceptance requires:");
            out.println("  - decoded kind byte `== 4`");
            out.println("  - decoded local word `+0x23 == [3525]`");
            out.println("  - decoded local flag byte `+0x0a == 0`");
            out.println();

            out.println("## Success / Report Split");
            out.println();
            dumpRange(out, "0000:4428", "0000:456e");
            out.println();
            out.println("Interpretation:");
            out.println("- on structural success the block calls `0x2000:b9a7` and branches");
            out.println("  into two nearby CS-local report families");
            out.println("- both branches format a report around the same local candidate data");
            out.println("- a later fallback path at `451b..456e` emits an additional message,");
            out.println("  re-runs `0x1000:d183`, copies the selected entry back through");
            out.println("  `0x2000:c151`, rewrites `351b..351f`, then finalizes through");
            out.println("  `0x2000:c100`, `0x2000:c02a`, and `0x2000:c2f0`");
            out.println("- `3521` is explicitly cleared at `44f5`, alongside `350c` at `44fa`");
            out.println("- practical consequence:");
            out.println("  - `42d8..456e` is a later starbase resolution/report loop, not a");
            out.println("    generic logger");
            out.println("  - `3525` participates directly in the structural accept path");
            out.println("  - `3521` behaves like a late report/control mode byte that is reset");
            out.println("    when this later report flow completes");
        }

        println("ReportUnknownStarbaseResolutionLoop> wrote " + outFile.getCanonicalPath());
    }

    private void dumpRange(PrintWriter out, String startStr, String endStr) throws Exception {
        Address start = toAddr(startStr);
        Address end = toAddr(endStr);
        Address curr = start;
        while (curr != null && curr.compareTo(end) <= 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst == null) {
                disassemble(curr);
                inst = getInstructionAt(curr);
            }
            if (inst == null) {
                out.printf("%s  <no instruction>%n", curr);
                curr = curr.add(1);
                continue;
            }
            out.printf("%s  %-32s ; bytes=%s%n",
                curr,
                inst.toString(),
                bytesHex(inst.getBytes()));
            curr = inst.getMaxAddress().add(1);
        }
    }

    private String bytesHex(byte[] bytes) {
        if (bytes == null || bytes.length == 0) {
            return "";
        }
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < bytes.length; i++) {
            if (i != 0) {
                sb.append(' ');
            }
            sb.append(String.format("%02x", bytes[i] & 0xff));
        }
        return sb.toString();
    }
}
