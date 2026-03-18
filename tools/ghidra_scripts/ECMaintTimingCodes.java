//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;

/**
 * Extract all MOV byte ptr [...], immediate assignments within 0000:02c0..1092
 * that write small constants (0-8) into stack-relative or ES-relative locations.
 * These are the timing code assignments in the 02c0 report decoder.
 *
 * Also scan 1000:dddb..e31b for the same pattern (durable event producers).
 */
public class ECMaintTimingCodes extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create: " + outputDir);
        }

        File report = new File(outputDir, "timing-code-assignments.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.println("ECMAINT Timing Code Assignments");
            out.println("================================");
            out.println();

            // Region 1: 0000:02c0..1092 (02c0 report decoder)
            out.println("Region: 0000:02c0..1092 (report decoder)");
            out.println("-----------------------------------------");
            scanRegion(out, "0000:02c0", "0000:1092");

            out.println();

            // Region 2: 1000:dddb..e31b (kind-1 durable producer)
            out.println("Region: 1000:dddb..e31b (kind-1 producer)");
            out.println("------------------------------------------");
            scanRegion(out, "1000:dddb", "1000:e31b");

            out.println();

            // Region 3: 1000:e31b..e700 (kind-2 durable producer)
            out.println("Region: 1000:e31b..e700 (kind-2 producer)");
            out.println("------------------------------------------");
            scanRegion(out, "1000:e31b", "1000:e700");
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void scanRegion(PrintWriter out, String startAddr, String endAddr) {
        Address start = toAddr(startAddr);
        Address end = toAddr(endAddr);
        Instruction ins = getInstructionAt(start);
        if (ins == null) {
            ins = getInstructionAfter(start);
        }

        while (ins != null && ins.getAddress().compareTo(end) <= 0 && !monitor.isCancelled()) {
            String mnem = ins.getMnemonicString();

            // Look for MOV byte ptr [something], immediate where immediate is 0-8
            if (mnem.equals("MOV") && ins.getNumOperands() == 2) {
                // Check if operand 1 is a small immediate
                Object[] op1 = ins.getOpObjects(1);
                if (op1.length == 1 && op1[0] instanceof Scalar) {
                    long val = ((Scalar) op1[0]).getUnsignedValue();
                    if (val >= 0 && val <= 8) {
                        String repr = ins.toString();
                        // Filter: only byte-ptr stores (not register moves)
                        if (repr.contains("byte ptr") || repr.contains("BYTE PTR")) {
                            out.printf("  %s  %s  [value=%d]%n",
                                ins.getAddress(), repr, val);
                        }
                    }
                }
            }

            // Also look for CMP with small constants (branching on kind/code values)
            if (mnem.equals("CMP") && ins.getNumOperands() == 2) {
                Object[] op1 = ins.getOpObjects(1);
                if (op1.length == 1 && op1[0] instanceof Scalar) {
                    long val = ((Scalar) op1[0]).getUnsignedValue();
                    if (val >= 1 && val <= 8) {
                        String repr = ins.toString();
                        out.printf("  %s  %s%n", ins.getAddress(), repr);
                    }
                }
            }

            ins = ins.getNext();
        }
    }
}
