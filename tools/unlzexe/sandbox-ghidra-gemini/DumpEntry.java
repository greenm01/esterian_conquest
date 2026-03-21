//Dump entry point.
//@category Sandbox
//@keybinding 
//@menupath 
//@toolbar 

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;

public class DumpEntry extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address entry = currentProgram.getSymbolTable().getPrimarySymbol("entry").getAddress();
        println("Entry Point Address: " + entry);
        dumpRange(entry, 0x100);
    }
    
    private void dumpRange(Address startAddr, int len) throws Exception {
        Address endAddr = startAddr.add(len);
        println("Disassembly from " + startAddr + " to " + endAddr + ":");
        InstructionIterator iter = currentProgram.getListing().getInstructions(startAddr, true);
        while (iter.hasNext()) {
            Instruction ins = iter.next();
            if (ins.getAddress().compareTo(endAddr) > 0) break;
            println("  " + ins.getAddress() + ": " + ins.toString());
        }
    }
}
