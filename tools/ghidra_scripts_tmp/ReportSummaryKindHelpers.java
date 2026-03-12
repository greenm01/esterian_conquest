import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportSummaryKindHelpers extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/summary-kind-helpers.txt";

    private static final String[][] TARGETS = new String[][]{
        {"2000:c067", "kind1_helper"},
        {"2000:c09a", "kind2_helper"},
        {"2000:c0cd", "kind3_helper"}
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
                writeFunction(out, target[0], target[1]);
            }
        }

        println("ReportSummaryKindHelpers> wrote " + outFile.getCanonicalPath());
    }

    private void writeFunction(PrintWriter out, String entryText, String label) throws Exception {
        Address entry = toAddr(entryText);
        Function fn = getFunctionAt(entry);
        out.printf("%s %s%n", entry, label);
        out.printf("- function: %s%n", fn == null ? "<none>" : fn.getName());
        out.println("- body:");

        Instruction inst = ensureInstruction(entry);
        int count = 0;
        while (inst != null && count < 120 && !monitor.isCancelled()) {
            out.printf("  - %s  %s", inst.getAddress(), inst);
            Reference[] refs = inst.getReferencesFrom();
            for (Reference ref : refs) {
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
                        }
                    }
                }
            }
            out.println();
            if ("RET".equals(inst.getMnemonicString()) || "RETF".equals(inst.getMnemonicString())) {
                break;
            }
            inst = nextInstruction(inst);
            count++;
        }
        out.println();
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

    private Instruction nextInstruction(Instruction inst) throws Exception {
        Instruction next = inst.getNext();
        if (next != null) {
            return next;
        }
        Address cursor = inst.getMaxAddress().add(1);
        for (int i = 0; i < 64 && next == null; i++) {
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
