import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportIPBMScratchRefs extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/ipbm-scratch-refs.txt";

    private static final String[][] TARGETS = new String[][] {
        {"2000:3538", "ipbm_scratch_base"},
        {"2000:353a", "ipbm_bypass_override_candidate"},
        {"2000:353b", "ipbm_follow_on_count_or_gate"},
        {"2000:3541", "ipbm_summary_byte_1"},
        {"2000:3542", "ipbm_summary_byte_2"},
        {"2000:354f", "ipbm_summary_word_a"},
        {"2000:3551", "ipbm_summary_word_b"},
        {"2000:3553", "ipbm_summary_word_c"}
    };

    @Override
    protected void run() throws Exception {
        File outFile = new File(currentProgram.getDomainFile().getProjectLocator().getLocation(), "../../" + OUTPUT_PATH);
        File parent = outFile.getCanonicalFile().getParentFile();
        if (!parent.exists() && !parent.mkdirs()) {
            throw new IllegalStateException("failed to create output dir " + parent);
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            for (String[] target : TARGETS) {
                writeTarget(out, target[0], target[1]);
            }
        }

        println("ReportIPBMScratchRefs> wrote " + outFile.getCanonicalPath());
    }

    private void writeTarget(PrintWriter out, String addrText, String label) throws Exception {
        Address addr = toAddr(addrText);
        out.printf("%s %s%n", addr, label);
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(addr);
        int count = 0;
        while (refs.hasNext() && !monitor.isCancelled()) {
            Reference ref = refs.next();
            Address from = ref.getFromAddress();
            Instruction inst = getInstructionContaining(from);
            Function fn = getFunctionContaining(from);
            out.printf("- ref from %s", from);
            if (inst != null) {
                out.printf("  %s", inst);
            }
            if (fn != null) {
                out.printf("  [function %s %s]", fn.getEntryPoint(), fn.getName());
            }
            out.println();
            writeNearby(out, from);
            count++;
        }
        if (count == 0) {
            out.println("- <none>");
        }
        out.println();
    }

    private void writeNearby(PrintWriter out, Address center) throws Exception {
        Instruction inst = getInstructionContaining(center);
        if (inst == null) {
            disassemble(center);
            inst = getInstructionContaining(center);
        }
        if (inst == null) {
            return;
        }

        Instruction start = inst;
        for (int i = 0; i < 3; i++) {
            Instruction prev = start.getPrevious();
            if (prev == null) {
                break;
            }
            start = prev;
        }

        Instruction cursor = start;
        int emitted = 0;
        while (cursor != null && emitted < 8 && !monitor.isCancelled()) {
            out.printf("  - %s  %s%n", cursor.getAddress(), cursor);
            cursor = cursor.getNext();
            emitted++;
        }
    }
}
