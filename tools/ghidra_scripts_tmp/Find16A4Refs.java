import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Find16A4Refs extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getMinAddress();
        Address end = currentProgram.getMaxAddress();
        
        println("Find16A4Refs> Searching for 16a4 references");
        while (curr != null && curr.compareTo(end) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                String s = inst.toString();
                if (s.contains("16a4") || s.contains("16A4")) {
                    println(String.format("Find16A4Refs> %s: %s", curr.toString(), s));
                }
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
        }
    }
}
