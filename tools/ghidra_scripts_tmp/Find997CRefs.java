import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Find997CRefs extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address curr = currentProgram.getMinAddress();
        Address end = currentProgram.getMaxAddress();
        
        println("Find997CRefs> Searching for calls to 997c");
        while (curr != null && curr.compareTo(end) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                String s = inst.toString();
                if (s.contains("997c") || s.contains("997C")) {
                    println(String.format("Find997CRefs> %s: %s", curr.toString(), s));
                }
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
        }
    }
}
