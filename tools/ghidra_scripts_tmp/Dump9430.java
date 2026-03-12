import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump9430 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address addr = currentProgram.getAddressFactory().getAddress("00029430");
        println("--- Instructions at " + addr + " ---");
        for (int i=0; i<20; i++) {
            Instruction inst = getInstructionAt(addr);
            if (inst == null) {
                println(addr + ": null");
                addr = addr.add(1);
            } else {
                println(addr.toString() + " " + inst.toString());
                addr = addr.add(inst.getLength());
            }
        }
    }
}
