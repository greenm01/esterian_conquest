import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.scalar.Scalar;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportIPBMScratchScalarUses extends GhidraScript {

    private static final String OUTPUT_PATH = "artifacts/ghidra/ecmaint-live/ipbm-scratch-uses.txt";

    @Override
    protected void run() throws Exception {
        File outFile = new File(currentProgram.getDomainFile().getProjectLocator().getLocation(), "../../" + OUTPUT_PATH);
        File parent = outFile.getCanonicalFile().getParentFile();
        if (!parent.exists() && !parent.mkdirs()) {
            throw new IllegalStateException("failed to create output dir " + parent);
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            out.println("Instructions with scalar operands in [0x3538, 0x3553]");

            Instruction inst = getFirstInstruction();
            int count = 0;
            while (inst != null && !monitor.isCancelled()) {
                if (usesTargetScalar(inst)) {
                    Function fn = getFunctionContaining(inst.getAddress());
                    out.printf("- %s  %s", inst.getAddress(), inst);
                    if (fn != null) {
                        out.printf("  [function %s %s]", fn.getEntryPoint(), fn.getName());
                    }
                    out.println();
                    writeNearby(out, inst);
                    count++;
                }
                inst = inst.getNext();
            }
            out.printf("%nMatch count: %d%n", count);
        }

        println("ReportIPBMScratchScalarUses> wrote " + outFile.getCanonicalPath());
    }

    private boolean usesTargetScalar(Instruction inst) {
        for (int i = 0; i < inst.getNumOperands(); i++) {
            Object[] objects = inst.getOpObjects(i);
            for (Object object : objects) {
                if (object instanceof Scalar scalar) {
                    long value = scalar.getUnsignedValue();
                    if (value >= 0x3538 && value <= 0x3553) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    private void writeNearby(PrintWriter out, Instruction center) {
        Instruction start = center;
        for (int i = 0; i < 3; i++) {
            Instruction prev = start.getPrevious();
            if (prev == null) {
                break;
            }
            start = prev;
        }

        Instruction cursor = start;
        int emitted = 0;
        while (cursor != null && emitted < 9) {
            out.printf("  - %s  %s%n", cursor.getAddress(), cursor);
            cursor = cursor.getNext();
            emitted++;
        }
    }
}
