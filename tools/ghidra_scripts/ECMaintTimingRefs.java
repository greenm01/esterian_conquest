//@category EsterianConquest

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.Arrays;
import java.util.List;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ECMaintTimingRefs extends GhidraScript {

    private static final List<String> TARGETS = Arrays.asList(
        "2000:945b",
        "2000:6fc6",
        "3000:189c",
        "3000:39dc"
    );

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "timing-refs.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            writeReferenceSections(out);
            writeConstantHits(out);
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void writeReferenceSections(PrintWriter out) {
        for (String target : TARGETS) {
            Address address = toAddr(target);
            out.printf("References to %s%n", address);
            ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(address);
            int count = 0;
            while (refs.hasNext() && !monitor.isCancelled()) {
                Reference ref = refs.next();
                Function caller = getFunctionContaining(ref.getFromAddress());
                out.printf(
                    "- %s (%s, %s)%n",
                    ref.getFromAddress(),
                    ref.getReferenceType(),
                    caller == null ? "<no-function>" : caller.getEntryPoint() + " " + caller.getName()
                );
                count++;
            }
            if (count == 0) {
                out.println("- <none>");
            }
            out.println();
        }
    }

    private void writeConstantHits(PrintWriter out) {
        long[] targets = {
            0x945bL,
            0x6fc6L,
            0x39dcL,
            0x189cL
        };
        out.println("Scalar constant hits");
        InstructionIterator it = currentProgram.getListing().getInstructions(true);
        while (it.hasNext() && !monitor.isCancelled()) {
            Instruction ins = it.next();
            for (int i = 0; i < ins.getNumOperands(); i++) {
                Object[] objects = ins.getOpObjects(i);
                for (Object object : objects) {
                    if (!(object instanceof Scalar)) {
                        continue;
                    }
                    long value = ((Scalar) object).getValue() & 0xffffL;
                    for (long target : targets) {
                        if (value == target) {
                            Function caller = getFunctionContaining(ins.getAddress());
                            out.printf(
                                "- 0x%04X at %s: %s (%s)%n",
                                target,
                                ins.getAddress(),
                                ins,
                                caller == null ? "<no-function>" : caller.getEntryPoint() + " " + caller.getName()
                            );
                        }
                    }
                }
            }
        }
    }
}
