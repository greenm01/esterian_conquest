import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump997C extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address addr = currentProgram.getAddressFactory().getAddress("0002997c");
        println("--- Instructions at 2000:997C ---");
        for (int i=0; i<80; i++) {
            Instruction inst = getInstructionAt(addr);
            if (inst == null) {
                addr = addr.add(1);
                continue;
            }
            println(addr.toString() + " " + inst.toString());
            addr = addr.add(inst.getLength());
            if (inst.toString().contains("RET")) break;
        }
    }
}
