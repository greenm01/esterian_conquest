//@category EsterianConquest

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceIterator;
import ghidra.program.model.symbol.ReferenceManager;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.listing.Listing;

public class FindCall9e1e extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address startAddr = toAddr("2000:7200");
        Address endAddr = toAddr("2000:7250");
        Listing listing = currentProgram.getListing();
        Instruction inst = listing.getInstructionAt(startAddr);

        while (inst != null && inst.getAddress().compareTo(endAddr) <= 0) {
            String str = inst.toString();
            println(inst.getAddress() + ": " + str);
            inst = inst.getNext();
        }
    }
}
