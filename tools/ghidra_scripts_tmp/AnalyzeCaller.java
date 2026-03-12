import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;

public class AnalyzeCaller extends GhidraScript {
    @Override
    protected void run() throws Exception {
        Address callAddr = currentProgram.getAddressFactory().getAddress("2000:7333");
        println("AnalyzeCaller> 2000:7333 is: " + getInstructionAt(callAddr));
        
        Function func = getFunctionContaining(callAddr);
        if (func != null) {
            println("AnalyzeCaller> Function containing caller: " + func.getName() + " at " + func.getEntryPoint());
        } else {
            println("AnalyzeCaller> No function found containing 2000:7333");
        }
        
        Address startAddr = currentProgram.getAddressFactory().getAddress("2000:7324");
        Address endAddr = currentProgram.getAddressFactory().getAddress("2000:733A");
        Address curr = startAddr;
        while (curr.compareTo(endAddr) < 0) {
            Instruction inst = getInstructionAt(curr);
            if (inst != null) {
                println("AnalyzeCaller> " + curr + ": " + inst);
                curr = inst.getAddress().add(inst.getLength());
            } else {
                curr = curr.add(1);
            }
        }
    }
}
