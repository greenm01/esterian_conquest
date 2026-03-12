import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;

public class ReportKind2HelperCallers extends GhidraScript {
    private static final String OUT_PATH = "artifacts/ghidra/ecmaint-live/kind2-helper-callers.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            reportTarget(out, "2000:c067");
            out.println();
            reportTarget(out, "2000:c09a");
        }

        println("ReportKind2HelperCallers> wrote " + outFile.getCanonicalPath());
    }

    private void reportTarget(PrintWriter out, String targetStr) throws Exception {
        Address target = toAddr(targetStr);
        out.printf("# Callers of %s%n%n", targetStr);

        List<Address> fromAddrs = new ArrayList<>();
        for (Reference ref : getReferencesTo(target)) {
            fromAddrs.add(ref.getFromAddress());
        }
        fromAddrs.sort(Comparator.naturalOrder());

        if (fromAddrs.isEmpty()) {
            out.println("<no references>");
            return;
        }

        for (Address from : fromAddrs) {
            out.printf("## Call from %s%n%n", from);
            dumpWindow(out, from, 6, 4);
            out.println();
        }
    }

    private void dumpWindow(PrintWriter out, Address center, int before, int after) {
        CodeUnit cu = currentProgram.getListing().getCodeUnitContaining(center);
        if (!(cu instanceof Instruction inst)) {
            out.println("<no instruction at center>");
            return;
        }

        List<Instruction> window = new ArrayList<>();
        Instruction curr = inst;
        for (int i = 0; i < before; i++) {
            curr = curr.getPrevious();
            if (curr == null) {
                break;
            }
            window.add(0, curr);
        }
        window.add(inst);
        curr = inst;
        for (int i = 0; i < after; i++) {
            curr = curr.getNext();
            if (curr == null) {
                break;
            }
            window.add(curr);
        }

        for (Instruction item : window) {
            out.printf("%s  %-28s ; bytes=%s%n",
                item.getAddress(),
                item.toString(),
                instructionBytesHex(item)
            );
        }
    }

    private String instructionBytesHex(Instruction inst) {
        try {
            return bytesHex(inst.getBytes());
        } catch (Exception e) {
            return "<bytes unavailable>";
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
