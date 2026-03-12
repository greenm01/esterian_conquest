import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;

public class DumpCaller8613 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address startAddr = currentProgram.getAddressFactory().getAddress("2000:85E0");
        Address endAddr = currentProgram.getAddressFactory().getAddress("2000:8630");
        
        println("DumpCaller8613> Disassembling around 2000:8613");
        disassemble(startAddr);
        
        Address curr = startAddr;
        while (curr.compareTo(endAddr) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("DumpCaller8613> %s: %s", curr.toString(), inst.toString()));
                curr = inst.getAddress().add(inst.getLength());
            } else {
                try {
                    byte b = getByte(curr);
                    println(String.format("DumpCaller8613> %s: db %02x", curr.toString(), b));
                } catch (Exception e) {}
                curr = curr.add(1);
            }
        }
    }
}
