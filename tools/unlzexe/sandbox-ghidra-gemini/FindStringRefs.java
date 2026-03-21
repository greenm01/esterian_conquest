
//Find string references.
//@category Sandbox
//@keybinding 
//@menupath 
//@toolbar 

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Reference;

public class FindStringRefs extends GhidraScript {
    @Override
    public void run() throws Exception {
        byte[] searchBytes = "*                                  Warning".getBytes();
        Address addr = find(null, searchBytes);
        if (addr == null) {
            println("String not found!");
            return;
        }
        println("Found string at: " + addr);
        
        Reference[] refs = getReferencesTo(addr);
        println("Found " + refs.length + " references:");
        for (Reference ref : refs) {
            println("  From: " + ref.getFromAddress());
            dumpRange(ref.getFromAddress(), 0x50);
        }
    }
    
    private void dumpRange(Address startAddr, int len) throws Exception {
        println("Disassembly at " + startAddr + ":");
        var listing = currentProgram.getListing();
        var ins_iter = listing.getInstructions(startAddr, true);
        int count = 0;
        while (ins_iter.hasNext() && count < 10) {
            var ins = ins_iter.next();
            println("  " + ins.getAddress() + ": " + ins.toString());
            count++;
        }
    }
}
