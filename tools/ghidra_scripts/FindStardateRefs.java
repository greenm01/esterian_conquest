//@category EsterianConquest

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.mem.Memory;

import java.io.File;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;

public class FindStardateRefs extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File outputDir = args.length >= 1 ? new File(args[0]) : new File("artifacts/ghidra/ecmaint-live");
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("failed to create output directory: " + outputDir);
        }

        File report = new File(outputDir, "stardate-ghidra-xrefs.txt");
        try (PrintWriter out = new PrintWriter(new FileWriter(report))) {
            out.printf("Program: %s%n%n", currentProgram.getName());
            
            Memory memory = currentProgram.getMemory();
            byte[] searchBytes = "Stardate".getBytes("ASCII");
            
            Address current = memory.getMinAddress();
            while (current != null) {
                current = memory.findBytes(current, searchBytes, null, true, monitor);
                if (current != null) {
                    out.printf("Found 'Stardate' at %s%n", current.toString());
                    
                    ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(current);
                    int count = 0;
                    while (refs.hasNext() && !monitor.isCancelled()) {
                        Reference ref = refs.next();
                        out.printf("  - referenced from: %s%n", ref.getFromAddress());
                        count++;
                    }
                    if (count == 0) {
                        out.println("  - <no Ghidra references found to exact address>");
                        
                        // In BP, strings have a length byte preceding them, so references might point to current - 1
                        Address lengthByteAddr = current.subtract(1);
                        ReferenceIterator bpRefs = currentProgram.getReferenceManager().getReferencesTo(lengthByteAddr);
                        int bpCount = 0;
                        while (bpRefs.hasNext() && !monitor.isCancelled()) {
                            Reference ref = bpRefs.next();
                            out.printf("  - referenced (via length byte) from: %s%n", ref.getFromAddress());
                            bpCount++;
                        }
                        if (bpCount == 0) {
                            out.println("  - <no Ghidra references found to length byte address either>");
                        }
                    }
                    out.println();
                    current = current.add(1);
                }
            }
        }
        println("Wrote " + report.getAbsolutePath());
    }
}
