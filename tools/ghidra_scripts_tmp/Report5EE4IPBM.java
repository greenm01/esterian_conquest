import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class Report5EE4IPBM extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/5ee4-ipbm.txt";

    private static final String[][] DATA_TARGETS = new String[][] {
        {"2000:31F8", "ipbm_record_stream_candidate"},
        {"2000:3278", "player_record_stream"},
        {"2000:2F78", "planet_record_stream"},
        {"2000:3178", "fleet_record_stream"},
        {"2000:2FF8", "base_record_stream"}
    };

    private static final String[][] CODE_RANGES = new String[][] {
        {"2000:675A", "2000:68E8", "player48_ipbm_branch"},
        {"2000:68E9", "2000:69B8", "post_ipbm_summary_branch"},
        {"2000:5EE4", "2000:6068", "integrity_entry_prefix"}
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
            writeFunctionHeader(out, "2000:5EE4", "ecmaint_validate_primary_state");
            writeDataTargets(out);
            for (String[] range : CODE_RANGES) {
                writeRange(out, range[0], range[1], range[2]);
            }
        }

        println("Report5EE4IPBM> wrote " + outFile.getCanonicalPath());
    }

    private void writeFunctionHeader(PrintWriter out, String entryText, String label) throws Exception {
        Address entry = toAddr(entryText);
        Function function = getFunctionAt(entry);
        out.printf("%s %s%n", entry, label);
        out.printf("- function: %s%n", function == null ? "<none>" : function.getName());
        out.println();
    }

    private void writeDataTargets(PrintWriter out) throws Exception {
        out.println("Data targets");
        for (String[] target : DATA_TARGETS) {
            Address addr = toAddr(target[0]);
            Data data = getDataAt(addr);
            out.printf("%s %s%n", addr, target[1]);
            out.printf("- defined data: %s%n", data == null ? "<none>" : data);
            out.printf("- bytes:");
            for (int i = 0; i < 16; i++) {
                out.printf(" %02x", getByte(addr.add(i)) & 0xff);
            }
            out.println();
            out.println();
        }
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

            Reference[] refs = instruction.getReferencesFrom();
            for (Reference ref : refs) {
                if (ref.getToAddress() != null) {
                    out.printf("    [ref %s]", ref.getToAddress());
                }
            }

            for (int i = 0; i < instruction.getNumOperands(); i++) {
                Object[] objects = instruction.getOpObjects(i);
                for (Object object : objects) {
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
