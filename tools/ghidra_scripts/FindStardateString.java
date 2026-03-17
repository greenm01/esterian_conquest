//@category EsterianConquest

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;

public class FindStardateString extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "stardate-xrefs.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            
            Memory memory = currentProgram.getMemory();
            byte[] searchBytes = "Stardate: ".getBytes("ASCII");
            
            List<Address> matches = new ArrayList<>();
            Address current = memory.getMinAddress();
            while (current != null) {
                current = memory.findBytes(current, searchBytes, null, true, monitor);
                if (current != null) {
                    matches.add(current);
                    out.printf("Found 'Stardate: ' at %s%n", current.toString());
                    findScalarReferences(out, current);
                    current = current.add(1);
                }
            }
            
            if (matches.isEmpty()) {
                // Try without the space
                searchBytes = "Stardate".getBytes("ASCII");
                current = memory.getMinAddress();
                while (current != null) {
                    current = memory.findBytes(current, searchBytes, null, true, monitor);
                    if (current != null) {
                        matches.add(current);
                        out.printf("Found 'Stardate' at %s%n", current.toString());
                        findScalarReferences(out, current);
                        current = current.add(1);
                    }
                }
            }
            
            if (matches.isEmpty()) {
                out.println("Could not find Stardate string.");
            }
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void findScalarReferences(PrintWriter out, Address targetAddr) throws Exception {
        long offset = targetAddr.getOffset();
        // The address in ghidra is segment:offset
        long shortOffset = offset & 0xFFFF;
        
        out.printf("Searching for 16-bit scalar 0x%04x...%n", shortOffset);
        
        int hits = 0;
        InstructionIterator iter = currentProgram.getListing().getInstructions(true);
        while (iter.hasNext() && !monitor.isCancelled()) {
            Instruction ins = iter.next();
            if (containsScalar(ins, shortOffset)) {
                hits++;
                out.printf("  - %s  %s%n", ins.getAddress(), ins);
                dumpWindow(out, ins, 4, 8);
            }
        }
        if (hits == 0) {
            out.println("  - <no scalar hits for " + String.format("0x%04x", shortOffset) + ">");
        }
        out.println();
    }

    private boolean containsScalar(Instruction ins, long value) {
        for (int operand = 0; operand < ins.getNumOperands(); operand++) {
            Object[] objects = ins.getOpObjects(operand);
            for (Object object : objects) {
                if (object instanceof Scalar && ((Scalar) object).getUnsignedValue() == value) {
                    return true;
                }
            }
        }
        return false;
    }

    private void dumpWindow(PrintWriter out, Instruction center, int back, int forward) {
        Instruction start = center;
        for (int i = 0; i < back; i++) {
            Instruction prev = start.getPrevious();
            if (prev == null) {
                break;
            }
            start = prev;
        }
        Instruction ins = start;
        int emitted = 0;
        int total = back + forward + 1;
        while (ins != null && emitted < total && !monitor.isCancelled()) {
            out.printf("      %s  %s%n", ins.getAddress(), ins);
            ins = ins.getNext();
            emitted++;
        }
    }
}
