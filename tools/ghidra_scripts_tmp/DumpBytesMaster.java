import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;

public class DumpBytesMaster extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getAddressFactory().getAddress("2000:99d3");
        println("DumpBytesMaster> Bytes from 99d3:");
        for (int i=0; i<20; i++) {
            byte b = getByte(curr.add(i));
            println(String.format("DumpBytesMaster> %02X", b));
        }
    }
}
