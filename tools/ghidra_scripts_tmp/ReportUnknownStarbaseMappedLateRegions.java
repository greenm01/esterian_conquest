import java.io.File;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.CodeUnit;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

public class ReportUnknownStarbaseMappedLateRegions extends GhidraScript {
    private static final long[] STARTS = {
        0x22f70L, // live 2895:2760
        0x22fbCL, // live 2895:27ac
        0x28630L, // live 2895:7e20
        0x2865bL, // live 2895:7e4b
    };

    @Override
    public void run() throws Exception {
        File outFile = new File(
            "./artifacts/ghidra/ecmaint-live/unknown-starbase-mapped-late-regions.txt");

        try (PrintWriter out = new PrintWriter(outFile)) {
            out.println("Mapped unknown-starbase late regions");
            out.println();

            for (long start : STARTS) {
                Address address = toAddr(start);
                Function function = getFunctionContaining(address);
                out.printf("== %s ==%n", address);
                if (function != null) {
                    out.printf("Function: %s @ %s%n", function.getName(), function.getEntryPoint());
                }
                else {
                    out.println("Function: <none>");
                }

                out.println("References:");
                Reference[] refs = getReferencesTo(address);
                boolean foundRef = false;
                for (Reference ref : refs) {
                    out.printf("  %s -> %s (%s)%n", ref.getFromAddress(), ref.getToAddress(), ref.getReferenceType());
                    foundRef = true;
                }
                if (!foundRef) {
                    out.println("  <none>");
                }

                out.println("Instructions:");
                Address cur = address.subtract(0x20);
                Address end = address.add(0x60);
                while (cur.compareTo(end) <= 0) {
                    CodeUnit cu = currentProgram.getListing().getCodeUnitAt(cur);
                    if (cu instanceof Instruction instr) {
                        out.printf("  %s: %s%n", instr.getAddress(), instr);
                        cur = instr.getMaxAddress().next();
                    }
                    else {
                        cur = cur.next();
                    }
                }
                out.println();
            }
        }
    }
}
