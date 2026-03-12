
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;

public class PrintLinear extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address addr = toAddr("2000:0000");
        println("Address: " + addr);
        println("Offset: " + addr.getOffset());
        println("Address Space: " + addr.getAddressSpace().getName());
    }
}
