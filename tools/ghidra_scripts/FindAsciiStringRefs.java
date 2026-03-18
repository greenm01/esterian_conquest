//@category EsterianConquest

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class FindAsciiStringRefs extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 2) {
            throw new IllegalArgumentException(
                "usage: FindAsciiStringRefs.java <ascii-text> <output_dir>");
        }

        String needle = args[0];
        File outputDir = new File(args[1]);
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        String safeName = needle.toLowerCase()
            .replaceAll("[^a-z0-9]+", "-")
            .replaceAll("^-+", "")
            .replaceAll("-+$", "");
        if (safeName.isEmpty()) {
            safeName = "string";
        }

        File report = new File(outputDir, "string-probe-" + safeName + ".txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n", currentProgram.getName());
            out.printf("Needle: %s%n%n", needle);
            findNeedle(out, needle);
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void findNeedle(PrintWriter out, String needle) throws Exception {
        Memory memory = currentProgram.getMemory();
        byte[] searchBytes = needle.getBytes("ASCII");
        Address current = memory.getMinAddress();
        int matchCount = 0;

        while (current != null && !monitor.isCancelled()) {
            current = memory.findBytes(current, searchBytes, null, true, monitor);
            if (current == null) {
                break;
            }
            matchCount++;
            out.printf("Match %d at %s%n", matchCount, current);
            out.printf("- preview: %s%n", readAscii(current));

            ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(current);
            int refCount = 0;
            out.println("- Ghidra refs:");
            while (refs.hasNext() && !monitor.isCancelled()) {
                Reference ref = refs.next();
                out.printf("  - %s (%s)%n", ref.getFromAddress(), ref.getReferenceType());
                refCount++;
            }
            if (refCount == 0) {
                out.println("  - <none>");
            }

            long shortOffset = current.getOffset() & 0xffff;
            out.printf("- scalar search for 0x%04x:%n", shortOffset);
            int scalarHits = 0;
            InstructionIterator iter = currentProgram.getListing().getInstructions(true);
            while (iter.hasNext() && !monitor.isCancelled()) {
                Instruction ins = iter.next();
                if (!containsScalar(ins, shortOffset)) {
                    continue;
                }
                scalarHits++;
                out.printf("  - %s  %s%n", ins.getAddress(), ins);
                dumpWindow(out, ins, 4, 8);
            }
            if (scalarHits == 0) {
                out.println("  - <none>");
            }

            out.println();
            current = current.add(1);
        }

        if (matchCount == 0) {
            out.println("<no matches>");
        }
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

    private String readAscii(Address address) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 160; i++) {
            byte b;
            try {
                b = getByte(address.add(i));
            } catch (Exception e) {
                break;
            }
            int value = b & 0xff;
            if (value == 0) {
                break;
            }
            if (value < 32 || value > 126) {
                if (sb.length() == 0) {
                    return "<non-ascii>";
                }
                break;
            }
            sb.append((char) value);
        }
        return sb.toString();
    }
}
