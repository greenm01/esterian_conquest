//Find the main function of a TP7 program.
//@category Sandbox
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.symbol.Reference;
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;

public class FindMain extends GhidraScript {
    @Override
    public void run() throws Exception {
        Address entry = currentProgram.getSymbolTable().getPrimarySymbol("entry").getAddress();
        println("Entry is at: " + entry);
        
        Function entryFunc = getFunctionAt(entry);
        if (entryFunc == null) {
            println("Entry is not a function.");
            return;
        }
        
        // Find the CALLs inside the entry function
        Address mainAddr = null;
        for (Instruction ins : currentProgram.getListing().getInstructions(entryFunc.getBody(), true)) {
            if (ins.getMnemonicString().equals("CALL")) {
                Reference[] refs = ins.getReferencesFrom();
                if (refs.length > 0) {
                    Address target = refs[0].getToAddress();
                    println("Entry calls: " + target);
                    // The last CALL in the entry function of a TP7 program is usually the main body
                    mainAddr = target;
                }
            }
        }
        
        if (mainAddr == null) {
            println("Could not find main call.");
            return;
        }
        
        println("Assuming MAIN is at: " + mainAddr);
        Function mainFunc = getFunctionAt(mainAddr);
        if (mainFunc == null) {
            println("Main is not a function.");
            return;
        }
        
        mainFunc.setName("ecmaint_main", ghidra.program.model.symbol.SourceType.USER_DEFINED);
        
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        DecompileResults results = decompiler.decompileFunction(mainFunc, 120, monitor);
        if (results.decompileCompleted()) {
            println("--- MAIN Decompiled Code ---");
            println(results.getDecompiledFunction().getC());
        } else {
            println("Decompilation failed.");
        }
        decompiler.dispose();
    }
}
