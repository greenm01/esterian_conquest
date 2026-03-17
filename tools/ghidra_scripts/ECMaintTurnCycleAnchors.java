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

public class ECMaintTurnCycleAnchors extends GhidraScript {

    private static final List<String> TARGETS = Arrays.asList(
        "2000:84a0", // widen into probable top-level driver before 861d
        "2000:861d", // external caller of restore/validate block
        "2000:87f4", // nearby later driver/report region
        "2000:8b15", // nearby common jump/loop tail
        "2000:8b3d", // common handoff target after report tail / failure path
        "3000:1abc", // pre-validate setup call
        "3000:1e88", // pre-validate setup/result call
        "2000:9e1e", // token wait helper
        "2000:6d9b", // Move.Tok restore entry
        "2000:5ee4", // integrity validator
        "2000:1da6", // first post-validate phase
        "2000:0c06", // second post-validate phase
        "2000:2db3", // late report/output plumbing candidate
        "2000:56be", // fourth post-validate phase
        "2000:7659", // conditional post-phase when 0x169a set
        "0000:0f0f", // caller into late summary function
        "0000:1092", // post-canonical summary/report function
        "0000:127a", // late 1..52 loop
        "3000:32e0"  // late flush helper
    );

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "turn-cycle-anchors.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            for (String target : TARGETS) {
                writeTarget(out, target);
            }
        }
        println("Wrote " + report.getAbsolutePath());
    }

    private void writeTarget(PrintWriter out, String addressText) {
        Address address = toAddr(addressText);
        Function function = getFunctionContaining(address);
        out.printf("Target %s%n", address);
        out.printf("- containing function: %s%n",
            function == null ? "<none>" : function.getEntryPoint() + " " + function.getName());
        out.println("- incoming references:");

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
        writeWindowAround(out, address, 40, 100);
        out.println();

        if (function != null) {
            out.println("- containing-function entry window:");
            writeWindow(out, function.getEntryPoint(), 120);
            out.println();
        }
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

    private void writeWindowAround(PrintWriter out, Address center, int backCount, int forwardCount) {
        Instruction ins = getInstructionContaining(center);
        if (ins == null) {
            ins = getInstructionAt(center);
        }
        if (ins == null) {
            out.printf("  %s <no instruction>%n", center);
            return;
        }

        for (int i = 0; i < backCount; i++) {
            Instruction prev = ins.getPrevious();
            if (prev == null) {
                break;
            }
            ins = prev;
        }
        writeWindow(out, ins.getAddress(), backCount + forwardCount);
    }
}
