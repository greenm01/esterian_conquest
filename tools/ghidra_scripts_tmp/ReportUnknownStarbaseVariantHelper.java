import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.mem.Memory;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseVariantHelper extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-variant-helper.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Variant Helper");
            out.println();
            out.println("- Focus: `0x3000:44b7` and the nearby `CS:6766` data passed");
            out.println("  with `3521` by the later starbase path.");
            out.println();

            out.println("## 3000:44b7..4560");
            out.println();
            dumpRange(out, "3000:44b7", "3000:4560");
            out.println();

            out.println("## 3000:6766 Bytes");
            out.println();
            dumpBytes(out, "3000:6766", 96);
            out.println();

            out.println("Interpretation:");
            out.println("- goal is to determine whether `3521` selects table rows,");
            out.println("  threshold bands, or another variant class through this helper");
        }

        println("ReportUnknownStarbaseVariantHelper> wrote " + outFile.getCanonicalPath());
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

    private void dumpBytes(PrintWriter out, String addrStr, int len) throws Exception {
        Address addr = toAddr(addrStr);
        Memory mem = currentProgram.getMemory();
        byte[] bytes = new byte[len];
        mem.getBytes(addr, bytes);
        for (int i = 0; i < bytes.length; i += 16) {
            int chunk = Math.min(16, bytes.length - i);
            byte[] slice = new byte[chunk];
            System.arraycopy(bytes, i, slice, 0, chunk);
            out.printf("%s  %s%n", addr.add(i), bytesHex(slice));
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
