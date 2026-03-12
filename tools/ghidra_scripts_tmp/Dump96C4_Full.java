import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;

public class Dump96C4_Full extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address startAddr = currentProgram.getAddressFactory().getAddress("2000:96C4");
        Address endAddr = currentProgram.getAddressFactory().getAddress("2000:9800");
        
        println("Dump96C4_Full> Disassembling 96C4 to 9800");
        Address curr = startAddr;
        while (curr.compareTo(endAddr) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("Dump96C4_Full> %s: %s", curr.toString(), inst.toString()));
                
                if (inst.getMnemonicString().equals("RETF")) {
                    println("Dump96C4_Full> End of function reached.");
                    break;
                }
                
                curr = inst.getAddress().add(inst.getLength());
            } else {
                try {
                    byte b = getByte(curr);
                    println(String.format("Dump96C4_Full> %s: db %02x", curr.toString(), b));
                } catch (Exception e) {}
                curr = curr.add(1);
            }
        }
    }
}
