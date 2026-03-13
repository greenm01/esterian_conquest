import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseScalarScan extends GhidraScript {
    private static final String OUT_PATH =
        "artifacts/ghidra/ecmaint-live/unknown-starbase-scalar-scan.txt";
    private static final long[] TARGETS = {0x3504, 0x350c, 0x350d, 0x350e, 0x351b, 0x351d, 0x351f, 0x3521, 0x3525};

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.println("# Unknown Starbase Scalar Scan");
            out.println();
            out.println("- Focus: brute-force instruction scan for scalar/immediate uses of");
            out.println("  the late starbase scratch offsets.");
            out.println();

            for (long target : TARGETS) {
                out.printf("## 0x%04x%n%n", target);
                scanForScalar(out, target);
                out.println();
            }
        }

        println("ReportUnknownStarbaseScalarScan> wrote " + outFile.getCanonicalPath());
    }

    private void scanForScalar(PrintWriter out, long target) {
        boolean saw = false;
        InstructionIterator it = currentProgram.getListing().getInstructions(true);
        while (it.hasNext()) {
            Instruction inst = it.next();
            Object[] objs = inst.getOpObjects(0);
            if (matches(objs, target)) {
                out.printf("- %s  %s%n", inst.getAddress(), inst);
                saw = true;
            }
            for (int i = 1; i < inst.getNumOperands(); i++) {
                objs = inst.getOpObjects(i);
                if (matches(objs, target)) {
                    out.printf("- %s  %s%n", inst.getAddress(), inst);
                    saw = true;
                    break;
                }
            }
        }
        if (!saw) {
            out.println("- <no scalar matches>");
        }
    }

    private boolean matches(Object[] objs, long target) {
        if (objs == null) {
            return false;
        }
        for (Object obj : objs) {
            if (obj instanceof Scalar scalar) {
                if (scalar.getUnsignedValue() == target) {
                    return true;
                }
            }
        }
        return false;
    }
}
