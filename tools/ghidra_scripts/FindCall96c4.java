
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class FindCall96c4 extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address addr = toAddr("2000:96c4");
        println("Callers of " + addr + ":");
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(addr);
        while (refs.hasNext()) {
            Reference ref = refs.next();
            println("  " + ref.getFromAddress());
        }
    }
}
