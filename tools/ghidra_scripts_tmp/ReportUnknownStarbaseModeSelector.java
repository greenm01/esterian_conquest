import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseModeSelector extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-mode-selector.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Mode Selector");
            out.println();
            out.println("- Focus: later uses of `3521` after the starbase-specific");
            out.println("  resolution/report loops.");
            out.println();

            out.println("## 0000:cce7..cd39");
            out.println();
            dumpRange(out, "0000:cce7", "0000:cd39");
            out.println();
            out.println("Interpretation:");
            out.println("- reads `3521` and selects one of several small constant tables");
            out.println("  written to `0x630..0x633`");
            out.println("- current best hypothesis: late report-layout / variant selector");
            out.println("  rather than a summary payload field");
            out.println();

            out.println("## 0000:f800..fa00");
            out.println();
            dumpRange(out, "0000:f800", "0000:fa00");
            out.println();
            out.println("Interpretation:");
            out.println("- includes the two later reads of `3521` at `f812` and `f8f2`");
            out.println("- should be read as the downstream consumer of the mode byte after");
            out.println("  the late starbase report paths");
        }

        println("ReportUnknownStarbaseModeSelector> wrote " + outFile.getCanonicalPath());
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
