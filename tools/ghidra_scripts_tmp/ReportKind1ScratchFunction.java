import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportKind1ScratchFunction extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/kind1-scratch-function.txt";

    private static final String[][] RANGES = new String[][] {
        {"0000:02ed", "0000:03d5", "kind1_dispatch_path"},
        {"2000:c067", "2000:c0a8", "kind1_helper_region"}
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
            writeFunctionHeader(out, "0000:02c0", "summary_dispatcher_candidate");
            writeFunctionHeader(out, "2000:c067", "kind1_helper_candidate");
            for (String[] range : RANGES) {
                writeRange(out, range[0], range[1], range[2]);
            }
        }

        println("ReportKind1ScratchFunction> wrote " + outFile.getCanonicalPath());
    }

    private void writeFunctionHeader(PrintWriter out, String entryText, String label) throws Exception {
        Address entry = toAddr(entryText);
        Function function = getFunctionAt(entry);
        out.printf("%s %s%n", entry, label);
        out.printf("- function: %s%n", function == null ? "<none>" : function.getName());
        out.println();
    }

    private void writeRange(PrintWriter out, String startText, String endText, String label) throws Exception {
        Address start = toAddr(startText);
        Address end = toAddr(endText);
        out.printf("%s (%s .. %s)%n", label, start, end);

        Instruction instruction = ensureInstruction(start);
        while (instruction != null && instruction.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
            out.printf("- %s  %s", instruction.getAddress(), instruction);

            Function function = getFunctionContaining(instruction.getAddress());
            if (function != null && instruction.getAddress().equals(function.getEntryPoint())) {
                out.printf("    [function %s]", function.getName());
            }

            for (Reference ref : instruction.getReferencesFrom()) {
                if (ref.getToAddress() != null) {
                    out.printf("    [ref %s]", ref.getToAddress());
                }
            }

            for (int i = 0; i < instruction.getNumOperands(); i++) {
                for (Object object : instruction.getOpObjects(i)) {
                    if (object instanceof Scalar scalar) {
                        out.printf("    [scalar 0x%x]", scalar.getUnsignedValue());
                    } else if (object instanceof Address address) {
                        out.printf("    [addr %s]", address);
                    }
                }
            }

            out.println();
            instruction = nextInstruction(instruction, end);
        }
        out.println();
    }

    private Instruction ensureInstruction(Address address) throws Exception {
        Instruction instruction = getInstructionAt(address);
        if (instruction != null) {
            return instruction;
        }
        disassemble(address);
        instruction = getInstructionContaining(address);
        if (instruction != null) {
            return instruction;
        }
        Address cursor = address;
        for (int i = 0; i < 32 && instruction == null; i++) {
            cursor = cursor.add(1);
            disassemble(cursor);
            instruction = getInstructionContaining(cursor);
        }
        return instruction;
    }

    private Instruction nextInstruction(Instruction instruction, Address end) throws Exception {
        Instruction next = instruction.getNext();
        if (next != null) {
            return next;
        }
        Address cursor = instruction.getMaxAddress().add(1);
        while (cursor.compareTo(end) <= 0 && !monitor.isCancelled()) {
            disassemble(cursor);
            next = getInstructionContaining(cursor);
            if (next != null && next.getAddress().compareTo(instruction.getAddress()) > 0) {
                return next;
            }
            cursor = cursor.add(1);
        }
        return null;
    }
}
