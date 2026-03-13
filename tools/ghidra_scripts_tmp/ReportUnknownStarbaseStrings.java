import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.Memory;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseStrings extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-strings.txt";
    private static final String[] TARGETS = {
        "0000:3f89",
        "0000:41a1",
        "0000:41e0",
        "0000:0a93",
        "0000:0abc",
        "0000:0ae6",
        "0000:0aed",
        "0000:0af4",
        "0000:0af6",
        "0000:0af8",
        "0000:0b17"
    };

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Strings");
            out.println();
            out.println("- Focus: nearby raw string fragments and CS-local text anchors around");
            out.println("  the late starbase report path.");
            out.println();
            for (String target : TARGETS) {
                out.printf("## %s%n%n", target);
                out.println(renderBytes(toAddr(target), 160));
                out.println();
            }
        }

        println("ReportUnknownStarbaseStrings> wrote " + outFile.getCanonicalPath());
    }

    private String renderBytes(Address start, int length) throws Exception {
        Memory memory = currentProgram.getMemory();
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < length; i++) {
            Address addr = start.add(i);
            byte b = memory.getByte(addr);
            int v = b & 0xff;
            if (v >= 32 && v < 127) {
                sb.append((char) v);
            }
            else {
                sb.append(String.format("<%02x>", v));
            }
        }
        return sb.toString();
    }
}
