import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Instruction;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportKind2DecodedFieldUses extends GhidraScript {
    private static final String OUT_PATH = "artifacts/ghidra/ecmaint-live/kind2-decoded-field-uses.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Kind-2 Decoded Field Uses");
            out.println();
            out.println("- Focus: immediate post-call field consumption after the known summary `+0x06`");
            out.println("  decoders populate `3502`, `3558`, or local buffers.");
            out.println();

            dumpWindow(out, "0000:0307", 0, 28);
            out.println();
            dumpWindow(out, "0000:03fe", 0, 40);
            out.println();
            dumpWindow(out, "0000:0681", 0, 20);
            out.println();
            dumpWindow(out, "1000:50c2", 0, 20);
        }

        println("ReportKind2DecodedFieldUses> wrote " + outFile.getCanonicalPath());
    }

    private void dumpWindow(PrintWriter out, String centerStr, int before, int after) {
        Address center = toAddr(centerStr);
        out.printf("## Window Around %s%n%n", centerStr);
        CodeUnit cu = currentProgram.getListing().getCodeUnitContaining(center);
        if (!(cu instanceof Instruction inst)) {
            out.println("<no instruction at center>");
            return;
        }

        Instruction curr = inst;
        for (int i = 0; i < before; i++) {
            curr = curr.getPrevious();
            if (curr == null) {
                break;
            }
        }

        int remaining = before + after + 1;
        while (curr != null && remaining-- > 0) {
            out.printf("%s  %-28s ; bytes=%s%n",
                curr.getAddress(),
                curr.toString(),
                instructionBytesHex(curr)
            );
            curr = curr.getNext();
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
