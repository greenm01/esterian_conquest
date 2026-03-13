import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseScratchRefs extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-scratch-refs.txt";
    private static final String[] TARGETS = {"0000:3504", "0000:350c", "0000:350d", "0000:350e", "0000:351b", "0000:351d", "0000:351f", "0000:3521", "0000:3525"};

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Scratch Refs");
            out.println();
            out.println("- Focus: code references to the late starbase scratch fields used by");
            out.println("  `0000:3fcf..41a0`.");
            out.println();
            for (String target : TARGETS) {
                out.printf("## %s%n%n", target);
                dumpRefs(out, toAddr(target));
                out.println();
            }
        }

        println("ReportUnknownStarbaseScratchRefs> wrote " + outFile.getCanonicalPath());
    }

    private void dumpRefs(PrintWriter out, Address target) {
        Reference[] refs = getReferencesTo(target);
        boolean saw = false;
        for (Reference ref : refs) {
            Address from = ref.getFromAddress();
            Instruction inst = getInstructionAt(from);
            if (inst == null) {
                disassemble(from);
                inst = getInstructionAt(from);
            }
            if (inst == null) {
                out.printf("- %s <no instruction>%n", from);
                saw = true;
                continue;
            }
            out.printf("- %s  %s%n", from, inst);
            saw = true;
        }
        if (!saw) {
            out.println("- <no references>");
        }
    }
}
