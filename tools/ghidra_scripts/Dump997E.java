import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;

public class Dump997E extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address startAddr = toAddr("2000:997e");
        println("=== Disassembling function at " + startAddr + " ===");
        
        Instruction inst = getInstructionAt(startAddr);
        int count = 0;
        while (inst != null && count < 100) {
            String opStr = inst.toString();
            println(inst.getAddress() + " " + opStr);
            if (opStr.startsWith("RET")) {
                break;
            }
            inst = inst.getNext();
            count++;
        }
        
        println("\n=== References to " + startAddr + " ===");
        for (Reference ref : currentProgram.getReferenceManager().getReferencesTo(startAddr)) {
            println("  " + ref.getFromAddress() + " (" + ref.getReferenceType() + ")");
        }
    }
}
