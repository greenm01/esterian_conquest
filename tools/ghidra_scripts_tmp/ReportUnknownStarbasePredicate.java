import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbasePredicate extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-predicate.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Predicate");
            out.println();
            out.println("- Focus: late starbase-specific region `0000:3fcf..41a0`.");
            out.println("- Goal: summarize the success predicate and the failure/report");
            out.println("  payload fields after the selector helper `0x1000:d183`.");
            out.println();
            out.println("## Selector Call And Gate");
            out.println();
            dumpRange(out, "0000:3fd9", "0000:3ff4");
            out.println();
            out.println("Interpretation:");
            out.println("- source scratch block is `DS:3502`");
            out.println("- `0x1000:d183` selects a candidate and returns selected-entry side");
            out.println("  effects through its local list / output bytes");
            out.println("- caller stores `AX` into local `[BP-0x28]` and rejects zero");
            out.println("- later code treats `[BP-0x28]` as the located summary slot");
            out.println();
            out.println("## Success Predicate");
            out.println();
            dumpRange(out, "0000:3ff4", "0000:40a4");
            out.println();
            out.println("Interpretation:");
            out.println("- current active summary index comes from caller arg `[BP+0x04]`");
            out.println("- located candidate summary index comes from local `[BP-0x28]`");
            out.println("- success requires:");
            out.println("  - located summary is active (`+0x03 != 0`)");
            out.println("  - current `+0x01 == located +0x01`");
            out.println("  - current `+0x02 == located +0x02`");
            out.println("  - current `+0x05 == located +0x05`");
            out.println("  - `byte ptr [0x350c] > 0`");
            out.println("- on success the routine sets local success flag `[BP-1] = 1`");
            out.println();
            out.println("## Failure / Report Payload");
            out.println();
            dumpRange(out, "0000:40a7", "0000:4184");
            out.println();
            out.println("Interpretation:");
            out.println("- failure/report path works from the same kind-1 scratch block `3502`");
            out.println("- it formats output using scratch fields:");
            out.println("  - `3525`");
            out.println("  - `351b..351f`");
            out.println("  - `350d`");
            out.println("  - `350e`");
            out.println("  - `3504`");
            out.println("- the branch at `40f7..410c` selects between two nearby CS-local");
            out.println("  string variants based on whether `351b..351f` is zero");
            out.println("- both failure/report exits clear `350c` and `3521`");
            out.println();
            out.println("## Early No-Match Exit");
            out.println();
            dumpRange(out, "0000:4186", "0000:41a0");
            out.println();
            out.println("Interpretation:");
            out.println("- if the selector stage yields no candidate, the routine jumps here");
            out.println("- this path emits one CS-local message through `0x3000:159b`");
            out.println("- then also clears `350c` and `3521` before returning");
        }

        println("ReportUnknownStarbasePredicate> wrote " + outFile.getCanonicalPath());
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
