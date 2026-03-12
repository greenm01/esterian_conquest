import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;

public class FindNearCallers997C extends GhidraScript {
    @Override
    protected void run() throws Exception {
        println("FindNearCallers997C> Searching raw instructions for CALL to 2000:997C...");
        
        InstructionIterator insts = currentProgram.getListing().getInstructions(true);
        while (insts.hasNext()) {
            Instruction inst = insts.next();
            String mnemonic = inst.getMnemonicString();
            if (mnemonic.equals("CALL")) {
                Object[] opObjects = inst.getOpObjects(0);
                if (opObjects.length > 0) {
                    String opStr = inst.getDefaultOperandRepresentation(0);
                    if (opStr.contains("997c") || opStr.contains("997C")) {
                        println("FindNearCallers997C> Found CALL at " + inst.getAddress() + ": " + inst.toString());
                    }
                }
            }
        }
    }
}
