import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseSelectorReturn extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-selector-return.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Selector Return");
            out.println();
            out.println("- Focus: local list layout and return block inside `1000:d166..d4fe`.");
            out.println("- Goal: explain how the helper's candidate list rooted at `FECC`");
            out.println("  yields the winning entry consumed by `0000:3fcf..41a0`.");
            out.println();
            out.println("## Local Slot Layout");
            out.println();
            out.println("- candidate count: `[BP + 0xFBD6]`");
            out.println("- candidate list base: `[BP + 0xFECC]`");
            out.println("- first candidate slot: `[BP + 0xFECE] = FECC + 2`");
            out.println("- ranking tuples: `[BP + 0xFBD8]`, `[BP + 0xFBDA]`, `[BP + 0xFBDC]`");
            out.println();
            out.println("## Candidate Insert Block");
            out.println();
            dumpRange(out, "1000:d205", "1000:d217");
            out.println();
            out.println("Interpretation:");
            out.println("- the helper increments candidate count first");
            out.println("- then stores the matched entry index at `FECC + count * 2`");
            out.println("- because the list is 1-based, the first real candidate lands at `FECE`");
            out.println();
            out.println("## Candidate Sort / First-Slot Swap Block");
            out.println();
            dumpRange(out, "1000:d46c", "1000:d4a2");
            out.println();
            out.println("Interpretation:");
            out.println("- sort/swaps move entry indexes within the same `FECC` list");
            out.println("- the first sorted winner is normalized back into the first slot");
            out.println("- after sorting, `FECE` is the winning candidate slot used by return");
            out.println();
            out.println("## Return Block");
            out.println();
            dumpRange(out, "1000:d4bc", "1000:d4fe");
            out.println();
            out.println("Interpretation:");
            out.println("- if candidate count is zero, helper returns `AL = 0`");
            out.println("- otherwise it returns `AL = 1`");
            out.println("- the selected entry bytes `0x00` and `0x01` are read from the");
            out.println("  winning candidate slot at `FECE`");
            out.println("- this means the helper's stable side effect is the winning");
            out.println("  selected-entry pair, while the direct register return is only a");
            out.println("  boolean success gate");
        }

        println("ReportUnknownStarbaseSelectorReturn> wrote " + outFile.getCanonicalPath());
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
