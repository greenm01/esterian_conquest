import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Dump5EE4_Part2 extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getAddressFactory().getAddress("2000:60FC");
        
        println("Dump5EE4_Part2> Disassembling from 60FC");
        int count = 0;
        while (count < 200) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println(String.format("Dump5EE4_Part2> %s: %s", curr.toString(), inst.toString()));
                if (inst.getMnemonicString().equals("RETF") || inst.getMnemonicString().equals("RET")) {
                    println("Dump5EE4_Part2> End of function reached.");
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
