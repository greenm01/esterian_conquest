import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Find9430Refs extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getMinAddress();
        Address end = currentProgram.getMaxAddress();
        
        println("Find9430Refs> Searching for calls to 9430");
        while (curr != null && curr.compareTo(end) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                String s = inst.toString();
                if (s.contains("9430") || s.contains("9430")) {
                    println(String.format("Find9430Refs> %s: %s", curr.toString(), s));
                }
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
        }
    }
}
