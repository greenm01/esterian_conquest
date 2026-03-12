import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseRefs extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/unknown-starbase-refs.txt";

    @Override
    protected void run() throws Exception {
        File outFile = new File(currentProgram.getDomainFile().getProjectLocator().getLocation(), "../../" + OUTPUT_PATH);
        File parent = outFile.getCanonicalFile().getParentFile();
        if (!parent.exists() && !parent.mkdirs()) {
            throw new IllegalStateException("failed to create output dir " + parent);
        }

        Address target = toAddr("2000:3f89");

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            out.printf("Target string: %s%n%n", target);
            ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(target);
            while (refs.hasNext()) {
                Reference ref = refs.next();
                Address from = ref.getFromAddress();
                out.printf("Reference %s -> %s%n", from, ref.getToAddress());
                Function fn = getFunctionContaining(from);
                out.printf("- function: %s%n", fn == null ? "<none>" : fn.getName());
                dumpContext(out, from);
                out.println();
            }
        }

        println("ReportUnknownStarbaseRefs> wrote " + outFile.getCanonicalPath());
    }

    private void dumpContext(PrintWriter out, Address center) throws Exception {
        Address start = center.subtract(16);
        Address end = center.add(24);
        Instruction instruction = getInstructionContaining(start);
        if (instruction == null) {
            disassemble(start);
            instruction = getInstructionContaining(start);
        }
        while (instruction != null && instruction.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
            out.printf("  %s  %s", instruction.getAddress(), instruction);
            if (instruction.getAddress().equals(center)) {
                out.print("    <xref>");
            }
            out.println();
            instruction = instruction.getNext();
            if (instruction == null) {
                break;
            }
        }
    }
}
