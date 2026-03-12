
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;

public class DumpDisasm extends GhidraScript {
    @Override
    public void run() throws Exception {
        dumpRange("2000:9970", "2000:9a00");
    }
    
    private void dumpRange(String start, String end) throws Exception {
        Address startAddr = toAddr(start);
        Address endAddr = toAddr(end);
        println("Disassembly from " + start + " to " + end + ":");
        InstructionIterator iter = currentProgram.getListing().getInstructions(startAddr, true);
        while (iter.hasNext()) {
            Instruction ins = iter.next();
            if (ins.getAddress().compareTo(endAddr) > 0) break;
            println("  " + ins.getAddress() + ": " + ins.toString());
        }
    }
}
