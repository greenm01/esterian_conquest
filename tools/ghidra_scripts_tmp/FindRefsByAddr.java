import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class FindRefsByAddr extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address[] addrs = {
            currentProgram.getAddressFactory().getAddress("00029430"),
            currentProgram.getAddressFactory().getAddress("0002997c"),
            currentProgram.getAddressFactory().getAddress("000296c4"),
            currentProgram.getAddressFactory().getAddress("00041634") // DS:16A4
        };
        
        for (Address addr : addrs) {
            println("References to " + addr + ":");
            if (addr == null) continue;
            ReferenceIterator iter = currentProgram.getReferenceManager().getReferencesTo(addr);
            while (iter.hasNext()) {
                Reference ref = iter.next();
                println("  " + ref.getFromAddress() + " -> " + ref.getToAddress());
            }
        }
    }
}
