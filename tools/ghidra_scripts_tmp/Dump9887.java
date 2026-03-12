import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump9887 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address startAddr = currentProgram.getAddressFactory().getAddress("2000:9887");
        Address endAddr = currentProgram.getAddressFactory().getAddress("2000:98B0");
        
        disassemble(startAddr);
        
        Address curr = startAddr;
        while (curr.compareTo(endAddr) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("Dump9887> %s: %s", curr.toString(), inst.toString()));
                if (inst.getMnemonicString().equals("RETF")) {
                    break;
                }
                curr = inst.getAddress().add(inst.getLength());
            } else {
                try {
                    byte b = getByte(curr);
                    println(String.format("Dump9887> %s: db %02x", curr.toString(), b));
                } catch (Exception e) {}
                curr = curr.add(1);
            }
        }
    }
}
