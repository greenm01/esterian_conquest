import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class AnalyzeMasterLoop extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address startAddr = currentProgram.getAddressFactory().getAddress("2000:9960");
        Address endAddr = currentProgram.getAddressFactory().getAddress("2000:9A00");
        
        println("AnalyzeMasterLoop> Disassembling around 2000:997C");
        disassemble(startAddr);
        
        Address curr = startAddr;
        while (curr.compareTo(endAddr) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("AnalyzeMasterLoop> %s: %s", curr.toString(), inst.toString()));
                curr = inst.getAddress().add(inst.getLength());
            } else {
                try {
                    byte b = getByte(curr);
                    println(String.format("AnalyzeMasterLoop> %s: db %02x", curr.toString(), b));
                } catch (Exception e) {
                    println(String.format("AnalyzeMasterLoop> %s: <unreadable>", curr.toString()));
                }
                curr = curr.add(1);
            }
        }
    }
}
