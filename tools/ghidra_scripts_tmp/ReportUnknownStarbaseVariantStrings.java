import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.Memory;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseVariantStrings extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-variant-strings.txt";

    private static final String[] ADDRS = {
        "0000:0d13",
        "0000:0d30",
        "0000:0d4b",
        "0000:0d53",
        "0000:0d68",
        "0000:0d85",
        "0000:0db3",
        "0000:0dc6",
        "0000:0e1a"
    };

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Variant Strings");
            out.println();
            out.println("- Focus: CS-local counted strings used by the late starbase");
            out.println("  report paths in `3fcf..41a0` and `42d8..456e`.");
            out.println();

            for (String s : ADDRS) {
                dumpCountedString(out, s);
                out.println();
            }

            out.println("Interpretation:");
            out.println("- `0xd30` / `0xd4b` belong to the `b9a7 != 0` branch");
            out.println("- `0xd53` / `0xd68` / `0xd85` / `0xdb3` / `0xdc6` belong to the");
            out.println("  `b9a7 == 0` branch");
            out.println("- `0xe1a` is the later fallback message before re-running");
            out.println("  `0x1000:d183`");
        }

        println("ReportUnknownStarbaseVariantStrings> wrote " + outFile.getCanonicalPath());
    }

    private void dumpCountedString(PrintWriter out, String addrStr) throws Exception {
        Address addr = toAddr(addrStr);
        Memory mem = currentProgram.getMemory();
        int len = mem.getByte(addr) & 0xff;
        byte[] bytes = new byte[len];
        mem.getBytes(addr.add(1), bytes);
        out.printf("## %s%n%n", addr);
        out.printf("- length: %d%n", len);
        out.printf("- text: %s%n", render(bytes));
        out.printf("- bytes: %s%n", bytesHex(bytes));
    }

    private String render(byte[] bytes) {
        StringBuilder sb = new StringBuilder();
        for (byte b : bytes) {
            int c = b & 0xff;
            if (c >= 32 && c <= 126) {
                sb.append((char) c);
            } else {
                sb.append(String.format("<%02x>", c));
            }
        }
        return sb.toString();
    }

    private String bytesHex(byte[] bytes) {
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
