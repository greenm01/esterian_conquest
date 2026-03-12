import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump1804 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getAddressFactory().getAddress("3000:1804");
        
        while (curr.compareTo(currentProgram.getAddressFactory().getAddress("3000:1850")) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("Dump1804> %s: %s", curr.toString(), inst.toString()));
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
        }
    }
}
