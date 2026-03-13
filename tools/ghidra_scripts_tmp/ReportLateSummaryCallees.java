import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportLateSummaryCallees extends GhidraScript {
    private static final String OUT_PATH = "artifacts/ghidra/ecmaint-live/late-summary-callees.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Late Summary Callees");
            out.println();
            out.println("- Focus: the segment-`1000` callees used by the later active-summary");
            out.println("  consumer at `0000:1302..1361`.");
            out.println("- Goal: identify which callee is more likely to drive starbase");
            out.println("  resolution or the later `unknown starbase` path.");
            out.println();

            out.println("## Window Around 1000:0b51");
            out.println();
            dumpRange(out, "1000:0b20", "1000:0be0");
            out.println();
            out.println("## Window Around 1000:a26e");
            out.println();
            dumpRange(out, "1000:a230", "1000:a2f0");
        }

        println("ReportLateSummaryCallees> wrote " + outFile.getCanonicalPath());
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
            out.printf("%s  %-28s ; bytes=%s%n",
                curr,
                inst.toString(),
                bytesHex(inst.getBytes())
            );
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
