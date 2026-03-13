import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbasePayloadProducers extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-payload-producers.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Payload Producers");
            out.println();
            out.println("- Focus: where the late starbase report fields in `3502` are written");
            out.println("  before `0000:3fcf..41a0` consumes them.");
            out.println();
            out.println("## Kind-1 Loader Consumption");
            out.println();
            dumpRange(out, "0000:0307", "0000:03cf");
            out.println();
            out.println("Interpretation:");
            out.println("- `350d` / `350e` are the first two decoded tag bytes from the");
            out.println("  shared kind-1 summary `+0x06` decoder");
            out.println("- `351b..351f` is the later 3-word payload group consumed by the");
            out.println("  same kind-1 dispatch path");
            out.println("- `350c` is the decoded selector/control byte copied out by the");
            out.println("  kind-1 loader and later checked by the late starbase predicate");
            out.println();
            out.println("## Common Post-Kind Writeback");
            out.println();
            dumpRange(out, "0000:0c6c", "0000:0cd4");
            out.println();
            out.println("Interpretation:");
            out.println("- the common post-kind pipeline writes the late starbase report");
            out.println("  payload fields directly:");
            out.println("  - `350d` and `350e` from the first two canonicalized tuples");
            out.println("  - `351b..351f` from the later 3-word payload group");
            out.println("- this confirms the late starbase report block is consuming the");
            out.println("  shared kind-1 canonicalized summary payload, not ad hoc local data");
            out.println();
            out.println("## Remaining Late-Path-Only Fields");
            out.println();
            out.println("- focused scalar scans still did not recover clean producer refs for:");
            out.println("  - `3521`");
            out.println("  - `3525`");
            out.println("- practical consequence:");
            out.println("  - `350d`, `350e`, and `351b..351f` are now traced back to the shared");
            out.println("    kind-1 canonicalization pipeline");
            out.println("  - `3521` and `3525` remain late-path scratch/state fields whose");
            out.println("    caller-side meaning still needs targeted recovery");
        }

        println("ReportUnknownStarbasePayloadProducers> wrote " + outFile.getCanonicalPath());
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
