import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportIPBMScratchFunction extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/ipbm-scratch-function.txt";

    @Override
    protected void run() throws Exception {
        File outFile = new File(currentProgram.getDomainFile().getProjectLocator().getLocation(), "../../" + OUTPUT_PATH);
        File parent = outFile.getCanonicalFile().getParentFile();
        if (!parent.exists() && !parent.mkdirs()) {
            throw new IllegalStateException("failed to create output dir " + parent);
        }

        Address entry = toAddr("0000:02c0");
        Function fn = getFunctionAt(entry);

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            out.printf("%s %s%n", entry, fn == null ? "<no-function>" : fn.getName());
            out.println();
            writeReferencesTo(out, entry);
            writeFunctionBody(out, entry);
        }

        println("ReportIPBMScratchFunction> wrote " + outFile.getCanonicalPath());
    }

    private void writeReferencesTo(PrintWriter out, Address entry) {
        out.println("References to function entry");
        var refs = currentProgram.getReferenceManager().getReferencesTo(entry);
        int count = 0;
        while (refs.hasNext() && !monitor.isCancelled()) {
            var ref = refs.next();
            Function caller = getFunctionContaining(ref.getFromAddress());
            out.printf("- %s", ref.getFromAddress());
            if (caller != null) {
                out.printf("  [caller %s %s]", caller.getEntryPoint(), caller.getName());
            }
            out.println();
            count++;
        }
        if (count == 0) {
            out.println("- <none>");
        }
        out.println();
    }

    private void writeFunctionBody(PrintWriter out, Address entry) throws Exception {
        out.println("Function body");
        Instruction inst = ensureInstruction(entry);
        int count = 0;
        while (inst != null && count < 900 && !monitor.isCancelled()) {
            out.printf("- %s  %s", inst.getAddress(), inst);
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
                        if (value >= 0x3538 && value <= 0x3557) {
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
