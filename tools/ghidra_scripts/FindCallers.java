
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.ReferenceManager;

public class FindCallers extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address addr = toAddr("2000:6d9b");
        
        ReferenceManager refMgr = currentProgram.getReferenceManager();
        ReferenceIterator refs = refMgr.getReferencesTo(addr);
        
        println("Callers of " + addr + ":");
        while (refs.hasNext()) {
            Reference ref = refs.next();
            println("  - " + ref.getFromAddress() + " (" + ref.getReferenceType() + ")");
        }
    }
}
