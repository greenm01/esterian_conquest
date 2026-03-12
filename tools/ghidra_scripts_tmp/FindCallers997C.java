import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;

public class FindCallers997C extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address target = currentProgram.getAddressFactory().getAddress("2000:997C");
        println("FindCallers997C> Searching for references to " + target);
        
        ReferenceIterator refs = currentProgram.getReferenceManager().getReferencesTo(target);
        boolean found = false;
        while (refs.hasNext()) {
            Reference ref = refs.next();
            println("FindCallers997C> Found reference from: " + ref.getFromAddress() + " type: " + ref.getReferenceType());
            found = true;
        }
        
        if (!found) {
            println("FindCallers997C> No direct references found. Searching raw instructions for CALL FAR to 2000:997C...");
            
            InstructionIterator insts = currentProgram.getListing().getInstructions(true);
            while (insts.hasNext()) {
                Instruction inst = insts.next();
                String mnemonic = inst.getMnemonicString();
                if (mnemonic.equals("CALLF")) {
                    Object[] opObjects = inst.getOpObjects(0);
                    if (opObjects.length > 0) {
                        String opStr = inst.getDefaultOperandRepresentation(0);
                        if (opStr.contains("997c") || opStr.contains("997C")) {
                            println("FindCallers997C> Found CALLF at " + inst.getAddress() + ": " + inst.toString());
                        }
                    }
                }
            }
        }
    }
}
