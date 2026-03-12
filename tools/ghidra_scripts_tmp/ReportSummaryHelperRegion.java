import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportSummaryHelperRegion extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/summary-helper-region.txt";

    @Override
    protected void run() throws Exception {
        File outFile = new File(currentProgram.getDomainFile().getProjectLocator().getLocation(), "../../" + OUTPUT_PATH);
        File parent = outFile.getCanonicalFile().getParentFile();
        if (!parent.exists() && !parent.mkdirs()) {
            throw new IllegalStateException("failed to create output dir " + parent);
        }

        Address start = toAddr("2000:bfc0");
        Address end = toAddr("2000:c140");

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            out.printf("Region: %s .. %s%n%n", start, end);
            Instruction inst = ensureInstruction(start);
            while (inst != null && inst.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
                out.printf("- %s  %s", inst.getAddress(), inst);
                for (Reference ref : inst.getReferencesFrom()) {
                    if (ref.getToAddress() != null) {
                        out.printf("    [ref %s]", ref.getToAddress());
                    }
                }
                for (int i = 0; i < inst.getNumOperands(); i++) {
                    Object[] objects = inst.getOpObjects(i);
                    for (Object object : objects) {
                        if (object instanceof Scalar scalar) {
                            long value = scalar.getUnsignedValue();
                            if (value >= 0x3500 && value <= 0x3580) {
                                out.printf("    [scratch 0x%x]", value);
                            } else if (value <= 0x20) {
                                out.printf("    [scalar 0x%x]", value);
                            }
                        }
                    }
                }
                out.println();
                inst = nextInstruction(inst, end);
            }
        }

        println("ReportSummaryHelperRegion> wrote " + outFile.getCanonicalPath());
    }

    private Instruction ensureInstruction(Address address) throws Exception {
        Instruction inst = getInstructionAt(address);
        if (inst != null) {
            return inst;
        }
        disassemble(address);
        inst = getInstructionContaining(address);
        if (inst != null) {
            return inst;
        }
        Address cursor = address;
        for (int i = 0; i < 64 && inst == null; i++) {
            cursor = cursor.add(1);
            disassemble(cursor);
            inst = getInstructionContaining(cursor);
        }
        return inst;
    }

    private Instruction nextInstruction(Instruction inst, Address end) throws Exception {
        Instruction next = inst.getNext();
        if (next != null) {
            return next;
        }
        Address cursor = inst.getMaxAddress().add(1);
        while (cursor.compareTo(end) <= 0 && !monitor.isCancelled()) {
            disassemble(cursor);
            next = getInstructionContaining(cursor);
            if (next != null && next.getAddress().compareTo(inst.getAddress()) > 0) {
                return next;
            }
            cursor = cursor.add(1);
        }
        return null;
    }
}
