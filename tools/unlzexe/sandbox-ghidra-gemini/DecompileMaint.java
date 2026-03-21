// Decompile a function
//@category Sandbox
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.listing.Function;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;

public class DecompileMaint extends GhidraScript {
    @Override
    public void run() throws Exception {
        byte[] searchBytes = "Scouting mission report:".getBytes();
        Address addr = find(null, searchBytes);
        if (addr == null) {
            println("String not found!");
            return;
        }
        println("Found string at: " + addr);
        
        Reference[] refs = getReferencesTo(addr);
        if (refs.length == 0) {
            println("No references found.");
            return;
        }
        
        Address refAddr = refs[0].getFromAddress();
        println("Referenced from: " + refAddr);
        
        Function func = getFunctionContaining(refAddr);
        if (func == null) {
            println("Not inside a function.");
            return;
        }
        
        println("Function name: " + func.getName());
        println("Function entry: " + func.getEntryPoint());
        
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        DecompileResults results = decompiler.decompileFunction(func, 60, monitor);
        if (results.decompileCompleted()) {
            println("--- Decompiled Code ---");
            println(results.getDecompiledFunction().getC());
        } else {
            println("Decompilation failed.");
        }
        decompiler.dispose();
    }
}
