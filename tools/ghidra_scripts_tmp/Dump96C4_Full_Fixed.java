import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump96C4_Full_Fixed extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getAddressFactory().getAddress("2000:96C4");
        
        println("Dump96C4_Full_Fixed> Disassembling from 96C4");
        int count = 0;
        while (count < 500) { // arbitrary limit to prevent infinite loop
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("Dump96C4_Full_Fixed> %s: %s", curr.toString(), inst.toString()));
                
                if (inst.getMnemonicString().equals("RETF")) {
                    println("Dump96C4_Full_Fixed> End of function reached.");
                    break;
                }
                
                curr = inst.getAddress().add(inst.getLength());
            } else {
                try {
                    byte b = getByte(curr);
                    println(String.format("Dump96C4_Full_Fixed> %s: db %02x", curr.toString(), b));
                } catch (Exception e) {}
                curr = curr.add(1);
            }
            count++;
        }
    }
}
