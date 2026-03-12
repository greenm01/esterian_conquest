import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.ReferenceManager;

public class FindCallersTarget extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] targets = {"2000:9e1e", "2000:9cb0", "2000:96c4", "2000:9d48"};
        ReferenceManager refMgr = currentProgram.getReferenceManager();
        
        for (String targetStr : targets) {
            Address addr = toAddr(targetStr);
            println("Callers of " + addr + ":");
            ReferenceIterator refs = refMgr.getReferencesTo(addr);
            while (refs.hasNext()) {
                Reference ref = refs.next();
                println("  - " + ref.getFromAddress() + " (" + ref.getReferenceType() + ")");
            }
            println("");
        }
    }
}
