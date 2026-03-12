import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class AnalyzeMasterLoop2 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address startAddr = currentProgram.getAddressFactory().getAddress("2000:9970");
        Address endAddr = currentProgram.getAddressFactory().getAddress("2000:998A");
        
        println("AnalyzeMasterLoop2> Raw bytes");
        Address curr = startAddr;
        while (curr.compareTo(endAddr) < 0) {
            byte b = getByte(curr);
            println(String.format("AnalyzeMasterLoop2> %s: %02x", curr.toString(), b));
            curr = curr.add(1);
        }
        
        // Also disassemble from 9979
        disassemble(currentProgram.getAddressFactory().getAddress("2000:997A"));
        
        println("AnalyzeMasterLoop2> Disassembling around 2000:997A");
        curr = currentProgram.getAddressFactory().getAddress("2000:997A");
        while (curr.compareTo(endAddr) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("AnalyzeMasterLoop2> %s: %s", curr.toString(), inst.toString()));
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
        }
    }
}
