import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;

public class DumpStrings extends GhidraScript {
    @Override
    protected void run() throws Exception {
        int[] offsets = {0x04CE, 0x04DA, 0x04E5, 0x04F0, 0x04FA, 0x0507, 0x0513, 0x051F};
        for (int offset : offsets) {
            Address addr = currentProgram.getAddressFactory().getAddress("2000:" + String.format("%04X", offset));
            StringBuilder sb = new StringBuilder();
            Address curr = addr;
            while (true) {
                byte b = getByte(curr);
                if (b == 0) break;
                sb.append((char)b);
                curr = curr.add(1);
            }
            println("DumpStrings> 2000:" + String.format("%04X", offset) + " -> " + sb.toString());
        }
        
        Address startAddr = currentProgram.getAddressFactory().getAddress("2000:99D0");
        Address endAddr = currentProgram.getAddressFactory().getAddress("2000:99E6");
        
        println("DumpStrings> Raw bytes 99D0-99E6");
        Address curr = startAddr;
        while (curr.compareTo(endAddr) < 0) {
            byte b = getByte(curr);
            println(String.format("DumpStrings> %s: %02x", curr.toString(), b));
            curr = curr.add(1);
        }
    }
}
