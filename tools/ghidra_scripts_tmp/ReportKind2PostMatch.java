import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportKind2PostMatch extends GhidraScript {
    private static final String OUT_PATH = "artifacts/ghidra/ecmaint-live/kind2-post-match.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Kind-2 Post-Match Report");
            out.println();
            out.println("- Focus: `0000:06AE..0800`, immediately after the kind-2 matcher loop.");
            out.println("- Goal: recover the handoff after a successful/failed kind-1/kind-2 pair");
            out.println("  so the remaining `unknown starbase` discriminator can be targeted.");
            out.println();

            dumpRange(out, "0000:06AE", "0000:0800");
        }

        println("ReportKind2PostMatch> wrote " + outFile.getCanonicalPath());
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
