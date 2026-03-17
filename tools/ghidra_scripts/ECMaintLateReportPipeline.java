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
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class ECMaintLateReportPipeline extends GhidraScript {

    private static final List<String> TARGETS = Arrays.asList(
        "0000:02c0",
        "1000:a26e",
        "1000:0b51",
        "2000:c057"
    );

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "late-report-pipeline.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            writeMasterLoop(out);
            for (String target : TARGETS) {
                writeTarget(out, target);
            }
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void writeMasterLoop(PrintWriter out) {
        Address start = toAddr("0000:12ef");
        out.println("Master loop window (0000:12ef..1369)");
        writeWindow(out, start, 40);
        out.println();
    }

    private void writeTarget(PrintWriter out, String addressText) {
        Address address = toAddr(addressText);
        Function function = getFunctionContaining(address);
        out.printf("Target %s%n", address);
        out.printf("- containing function: %s%n",
            function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());
        out.println("- callers:");
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(address);
        int count = 0;
        while (refs.hasNext() && !monitor.isCancelled()) {
            Reference ref = refs.next();
            Function caller = getFunctionContaining(ref.getFromAddress());
            out.printf("  - %s (%s, %s)%n",
                ref.getFromAddress(),
                ref.getReferenceType(),
                caller == null ? "<no-function>" : caller.getEntryPoint() + " " + caller.getName());
            count++;
        }
        if (count == 0) {
            out.println("  - <none>");
        }
        out.println("- local window:");
        writeWindow(out, address, 60);
        out.println();
    }

    private void writeWindow(PrintWriter out, Address start, int instructionCount) {
        Instruction ins = getInstructionContaining(start);
        if (ins == null) {
            ins = getInstructionAt(start);
        }
        if (ins == null) {
            out.printf("  %s <no instruction>%n", start);
            return;
        }
        int emitted = 0;
        while (ins != null && emitted < instructionCount && !monitor.isCancelled()) {
            out.printf("  %s  %s%n", ins.getAddress(), ins);
            ins = ins.getNext();
            emitted++;
        }
    }
}
