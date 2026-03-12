//@category EsterianConquest

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.ReferenceManager;

public class FindCall9e1e extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address addr = toAddr("2000:9e1e");
        println("Callers of " + addr + ":");
        ReferenceManager refMgr = currentProgram.getReferenceManager();
        ReferenceIterator refs = refMgr.getReferencesTo(addr);
        
        while (refs.hasNext()) {
            Reference ref = refs.next();
            println("  - " + ref.getFromAddress() + " (" + ref.getReferenceType() + ")");
        }
    }
}
