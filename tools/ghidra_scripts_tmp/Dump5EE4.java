import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump5EE4 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getAddressFactory().getAddress("2000:5EE4");
        
        println("Dump5EE4> Disassembling from 5EE4");
        int count = 0;
        while (count < 200) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("Dump5EE4> %s: %s", curr.toString(), inst.toString()));
                if (inst.getMnemonicString().equals("RETF") || inst.getMnemonicString().equals("RET")) {
                    println("Dump5EE4> End of function reached.");
                    break;
                }
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
            count++;
        }
    }
}
