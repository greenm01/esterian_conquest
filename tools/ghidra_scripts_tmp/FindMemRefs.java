import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;

public class FindMemRefs extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address addr = currentProgram.getAddressFactory().getAddress("0000:16A4");
        if (addr == null) {
            println("FindMemRefs> Address not found");
            return;
        }
        
        println("FindMemRefs> References to 16a4:");
        for (Reference ref : getReferencesTo(addr)) {
            println(String.format("FindMemRefs> %s: %s", ref.getFromAddress(), ref.getReferenceType()));
        }
        
        addr = currentProgram.getAddressFactory().getAddress("2000:16a4"); // DS is 2000?
        if (addr != null) {
            println("FindMemRefs> References to 2000:16a4:");
            for (Reference ref : getReferencesTo(addr)) {
                println(String.format("FindMemRefs> %s: %s", ref.getFromAddress(), ref.getReferenceType()));
            }
        }
    }
}
