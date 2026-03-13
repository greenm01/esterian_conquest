import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseLateRanges extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-late-ranges.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Late Ranges");
            out.println();
            out.println("- Focus: hot ranges surfaced by the scalar scan for `3521` and `3525`.");
            out.println();
            out.println("## 0000:42d0..4570");
            out.println();
            dumpRange(out, "0000:42d0", "0000:4570");
            out.println();
            out.println("## 0000:cce0..cd10");
            out.println();
            dumpRange(out, "0000:cce0", "0000:cd10");
            out.println();
            out.println("## 0000:f7e0..fa10");
            out.println();
            dumpRange(out, "0000:f7e0", "0000:fa10");
        }

        println("ReportUnknownStarbaseLateRanges> wrote " + outFile.getCanonicalPath());
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
