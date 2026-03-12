import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump63CA extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getAddressFactory().getAddress("2000:63CA");
        
        while (curr.compareTo(currentProgram.getAddressFactory().getAddress("2000:63F0")) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("Dump63CA> %s: %s", curr.toString(), inst.toString()));
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
        }
    }
}
