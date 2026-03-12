
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class FindRefs extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address addr = toAddr("2000:841b");
        println("References to " + addr + ":");
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(addr);
        while (refs.hasNext()) {
            Reference ref = refs.next();
            println("  " + ref.getFromAddress() + " (" + ref.getReferenceType() + ")");
        }
    }
}
