import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump08F8 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address addr = currentProgram.getAddressFactory().getAddress("00029d48");
        println("--- Instructions at 29d48 ---");
        for (int i=0; i<30; i++) {
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
