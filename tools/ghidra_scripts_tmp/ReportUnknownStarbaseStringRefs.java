import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;

public class ReportUnknownStarbaseStringRefs extends GhidraScript {
    private static final String OUT_PATH = "artifacts/ghidra/ecmaint-live/unknown-starbase-string-refs.txt";

    @Override
    public void run() throws Exception {
        File outFile = new File(OUT_PATH);
        File parent = outFile.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }

        try (PrintWriter out = new PrintWriter(new FileWriter(outFile))) {
            Address addr = toAddr("0000:3f89");
            out.println("# Unknown Starbase String Refs");
            out.println();
            out.println("- Raw string location from `strings -td /tmp/ecmaint-debug/MEMDUMP.BIN`:");
            out.println("  `Fleet assigned to an unknown starbase.` at `0000:3f89`");
            out.println();

            ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(addr);
            while (refs.hasNext()) {
                Reference ref = refs.next();
                Address from = ref.getFromAddress();
                Function fn = getFunctionContaining(from);
                String fnName = fn == null ? "<no function>" : fn.getEntryPoint() + " " + fn.getName();
                out.printf("- %s -> %s  [%s]  in %s%n", from, ref.getToAddress(), ref.getReferenceType(), fnName);
            }
        }

        println("ReportUnknownStarbaseStringRefs> wrote " + outFile.getCanonicalPath());
    }
}
